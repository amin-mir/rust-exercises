// LazyTransform(transformFn)
// set_source gets a source which can be passed to transformFn to get the
// new value which should be cached and served in get_transformed. The
// calculation should not happen until get_transformed is called.

use std::fmt::Debug;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use seize::{reclaim, Collector, Guard, Linked};

// TODO: source and val can be of different types.
pub struct LazyTransform<F, T: Debug> {
    collector: Collector,
    transform: F,
    seq_counter: AtomicUsize,
    val_ctx: AtomicPtr<Linked<ValueContext<T>>>,
    src_ctx: AtomicPtr<Linked<SourceContext<T>>>,

    // Metrics.
    // Incremented when the attempt to set source context through
    // compare_exchange succeeds.
    set_source_comp_exch_success: AtomicUsize,
    // Incremented when our source context is more up-to-date and we're
    // going to try compare_exchange again.
    set_source_comp_exch_failure_retryable: AtomicUsize,
    // Incremented when someone has already inserted source context with a
    // higher sequence numebr than the one we tried to insert.
    set_source_comp_exch_failure_outdated: AtomicUsize,
}

struct ValueContext<T: Debug> {
    seq: usize,
    val: T,
}

struct SourceContext<T: Debug> {
    seq: usize,
    source: Option<T>,
}

impl<T> ValueContext<T>
where
    T: Debug,
{
    fn new(seq: usize, val: T) -> Self {
        Self { seq, val }
    }
}

impl<T> SourceContext<T>
where
    T: Debug,
{
    fn new(seq: usize, source: Option<T>) -> Self {
        Self { seq, source }
    }
}

// impl<T: Debug> Drop for SourceContext<T> {
//     fn drop(&mut self) {
//         println!("dropping source context with seq={}, source={:?}", self.seq, self.source);
//     }
// }

impl<F, T> Drop for LazyTransform<F, T>
where
    T: Debug,
{
    fn drop(&mut self) {
        // SAFETY: because we have a &mut to self, it's safe to drop
        // everything immediate as Rust guarantees that no one else
        // will have a reference to self. And because of this, we won't
        // have double-free error for self.val because there's single
        // &mut when this code runs.
        let guard = unsafe { Guard::unprotected() };

        // Ordering is irrelevant here because Atomics are loaded immediately
        // anyways due to the special guard that we use here.
        let val_ctx = guard.protect(&self.val_ctx, Ordering::Relaxed);
        let src_ctx = guard.protect(&self.src_ctx, Ordering::Relaxed);

        if !val_ctx.is_null() {
            unsafe {
                guard.retire(val_ctx, reclaim::boxed::<T>);
            }
        }
        if !src_ctx.is_null() {
            unsafe {
                guard.retire(src_ctx, reclaim::boxed::<SourceContext<T>>);
            }
        }
    }
}

impl<F, T> LazyTransform<F, T>
where
    T: Debug,
    F: Fn(&T) -> T,
{
    pub fn new(transform: F) -> Self {
        Self {
            collector: Collector::new(),
            transform,
            seq_counter: AtomicUsize::new(0),
            val_ctx: AtomicPtr::default(),
            src_ctx: AtomicPtr::default(),
            set_source_comp_exch_success: AtomicUsize::new(0),
            set_source_comp_exch_failure_retryable: AtomicUsize::new(0),
            set_source_comp_exch_failure_outdated: AtomicUsize::new(0),
        }
    }

    pub fn set_source(&self, source: T) {
        // TODO: should Ordering be Relaxed?
        let new_seq = self.seq_counter.fetch_add(1, Ordering::AcqRel) + 1;

        // Make the heap allocation once outside the loop.
        let new_src = self
            .collector
            .link_boxed(SourceContext::new(new_seq, Some(source)));

        let guard = self.collector.enter();
        let mut cur_src = guard.protect(&self.src_ctx, Ordering::Acquire);

        loop {
            // Ordering for failure is set to Acquire because in case of success, cur
            // is guaranteed to be the actual previous value which can be retired now.
            // In case of failure, (a) we need to compare the sequence numebrs between
            // new and cur, and (b) we might need to retry the CAS, thus Acquire is used.
            match self.src_ctx.compare_exchange(
                cur_src,
                new_src,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(cur) => unsafe {
                    self.set_source_comp_exch_success
                        .fetch_add(1, Ordering::Relaxed);
                    // SAFETY: the old value has been swapped out so new threads won't have
                    // access to it, thus it's safe to retire it.
                    //
                    // On the first call to set_source, cur is still empty, so we should
                    // make sure it's not null before retiring.
                    if !cur.is_null() {
                        self.collector
                            .retire(cur, reclaim::boxed::<SourceContext<T>>);
                    }
                    break;
                },
                Err(cur) => {
                    // It's also possible to not handle the error case here because source_ctx
                    // val_ctx are independent of each other. Handling the error here causes
                    // set_source to not write outdated source (less seq) ever. If we don't handle
                    // we can end up writing an outdated source which is then picked up by another
                    // thread doing a write, does the expensive calculation and then checks val_ctx
                    // only to realize that there's already newer val (higher seq). If this is used
                    // in a write heavy application though, it's possible to lose the newer source
                    // if we're not handling the the errors here.
                    let cur_ref = unsafe { &*cur };

                    // Impossible for two threads to acquire the same sequence number.
                    assert_ne!(new_seq, cur_ref.seq);

                    if new_seq > cur_ref.seq {
                        self.set_source_comp_exch_failure_retryable
                            .fetch_add(1, Ordering::Relaxed);
                        // We have the latest data, so we should over-write.
                        cur_src = cur;
                    } else {
                        self.set_source_comp_exch_failure_outdated
                            .fetch_add(1, Ordering::Relaxed);
                        // Our source context is already outdated, so retire the allocation.
                        // SAFETY: because we're the sole owner of this allocation, and we
                        // haven't stored it anywhere, it's safe to retire at any time.
                        unsafe {
                            self.collector
                                .retire(new_src, reclaim::boxed::<SourceContext<T>>);
                        }
                        break;
                    }
                }
            }
        }
    }

    pub fn guard(&self) -> GuardedLazyTransform<'_, F, T> {
        let guard = self.collector.enter();
        GuardedLazyTransform { guard, lt: self }
    }

    pub fn get<'a>(&self, guard: &'a Guard<'a>) -> &'a T {
        let mut cur_src_ctx = guard.protect(&self.src_ctx, Ordering::Acquire);

        if unsafe { &(*cur_src_ctx) }.source.is_some() {
            let seq = unsafe { &(*cur_src_ctx) }.seq;
            let new_src_ctx = self
                .collector
                .link_boxed(SourceContext::new(seq, None));

            loop {
                match self.src_ctx.compare_exchange(
                    cur_src_ctx,
                    new_src_ctx,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(cur_src) => {
                        // Eventually, cur_src_ctx must be deallocated because CAS was successful
                        // so no new threads will have access to it anymore, thus safe to retire.
                        // cur_src is guaranteed to be the cur_src_ctx. We should prefer to use cur_src
                        // because we're in a loop and this CAS could be retried with a different cur_src_ctx
                        // so in every iteration we need to get the most up-to-date value.
                        //
                        // Also we know for a fact that source is not None because we checked it above.
                        // So it's safe to do unwrap here. If someone else changes the src_ctx, CAS will fail.
                        let src = unsafe { &(*cur_src) }.source.as_ref().unwrap();
                        
                        // Perform the potentially expensive calculation.
                        let new_val = (self.transform)(&src);

                        let new_val_ctx = self
                            .collector
                            .link_boxed(ValueContext::new(seq, new_val));

                        let mut cur_val_ctx = guard.protect(&self.val_ctx, Ordering::Acquire);
                        let cur_val_ctx_ref = unsafe { &(*cur_val_ctx) };

                        // When sequence number of current value is greater than the one we used for
                        // the calculation, someone else has already done the calcuation with a newer source.
                        // So we can retire new_val_ctx and cur_src_ctx. 
                        if seq < cur_val_ctx_ref.seq {
                            // Using guard to delay retiring until the guard is dropped.
                            unsafe { guard.retire(new_val_ctx, reclaim::boxed::<ValueContext<T>>) };
                            unsafe {
                                guard.retire(cur_src, reclaim::boxed::<SourceContext<T>>)
                            };
                            return &cur_val_ctx_ref.val;
                        } else {
                            // We have a more up-to-date value so we attemp to over-write the current one.
                            loop {
                                match self.val_ctx.compare_exchange(
                                    cur_val_ctx,
                                    new_val_ctx,
                                    Ordering::AcqRel,
                                    Ordering::Acquire,
                                ) {
                                    Ok(_) => {
                                        // Ok will contain a ptr that is equal to cur_val_ctx so we 
                                        // just ignore that.
                                        //
                                        // We've successfully stored the value we calculated, so we can retire
                                        // cur_val_ctx & cur_src_ctx.
                                        unsafe {
                                            guard.retire(
                                                cur_val_ctx,
                                                reclaim::boxed::<ValueContext<T>>,
                                            )
                                        };
                                        unsafe {
                                            guard.retire(
                                                cur_src,
                                                reclaim::boxed::<SourceContext<T>>,
                                            )
                                        };

                                        return unsafe { &(*new_val_ctx).val };
                                    }
                                    Err(cur_val) => {
                                        let old_seq = unsafe { &(*cur_val) }.seq;

                                        // `new_seq == old_seq` is impossible because there's no way that two threads
                                        // can take on the responsibility of calculating the value with same seq.
                                        assert_ne!(seq, old_seq);

                                        if seq > old_seq {
                                            // We have value with newer sequence number and coming here
                                            // means that someone else with older value managed to do the CAS
                                            // first so we should retry.
                                            cur_val_ctx = cur_val;
                                        } else {
                                            // Someone with newer value already succeeded so we can retire our
                                            // new_val and old src_ctx. So return the current value.
                                            unsafe {
                                                guard.retire(
                                                    new_val_ctx,
                                                    reclaim::boxed::<ValueContext<T>>,
                                                )
                                            };
                                            unsafe {
                                                guard.retire(
                                                    cur_src,
                                                    reclaim::boxed::<SourceContext<T>>,
                                                )
                                            };

                                            return unsafe { &(**cur_val).val };
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(cur_src) => {
                        let cur_src_ref = unsafe { &(*cur_src) };
                        let cur_seq = cur_src_ref.seq;

                        // It's not possible to have our_seq > their_seq which means that
                        // we found another seq number that is lower than ours. The reason
                        // for it is because if we loaded a higher seq number, when the
                        // holder of lower seq tries the CAS, they will find out that there's
                        // already been a successful set_source and their seq is now obsolete.
                        assert!(seq <= cur_seq);

                        if seq < cur_seq {
                            // It means that there's newer source from set_source and we
                            // should update the new allocation with the new sequence number
                            // and retry the CAS.
                            if cur_src_ref.source.is_some() {
                                // We're the sole owner of new_src_ctx, so it's safe to get a
                                // mutable reference to it.
                                unsafe { &mut (**new_src_ctx) }.seq = cur_seq;
                                cur_src_ctx = cur_src;
                            }
                        } else {
                            // Means that the source _must_ be None. which means someone
                            // else is already taking care of it. We can proceed to load the val.
                            // We should retire our allocation for new_src_ctx.
                            // The thread with successful CAS should take care of retiring the
                            // cur_src_ctx at the end.
                            assert!(cur_src_ref.source.is_none());
                            unsafe {
                                guard.retire(new_src_ctx, reclaim::boxed::<SourceContext<T>>)
                            };
                            break;
                        }
                    }
                }
            }
        }

        let val_ctx = guard.protect(&self.val_ctx, Ordering::Acquire);
        unsafe { &(**val_ctx).val }
    }
}

pub struct GuardedLazyTransform<'a, F, T: Debug> {
    guard: Guard<'a>,
    lt: &'a LazyTransform<F, T>,
}

impl<F, T> GuardedLazyTransform<'_, F, T>
where
    T: Debug,
    F: Fn(&T) -> T,
{
    pub fn get(&self) -> &T {
        self.lt.get(&self.guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;

    const CONC_CALL_COUNT: usize = 1_000_000;

    fn string_transform(s: &String) -> String {
        format!("{} - extended!!!", s)
    }

    #[test]
    fn drop_empty_lazy_transform() {
        let lt = LazyTransform::new(string_transform);

        drop(lt);
    }
    #[test]
    fn set_source_first_call() {
        let lt = LazyTransform::new(string_transform);

        lt.set_source("input".to_string());
    }

    #[test]
    fn set_source_many_concurrent_calls() {
        let lt = LazyTransform::new(|src: &(String, usize)| (src.0.to_owned(), src.1));

        thread::scope(|s| {
            for _ in 0..20 {
                s.spawn(|| {
                    for i in 0..CONC_CALL_COUNT {
                        lt.set_source((format!("{:?}", thread::current().id()), i));
                    }
                });
            }
        });

        let src_ctx = lt.src_ctx.load(Ordering::Relaxed);
        println!("{:?}", unsafe { &(*src_ctx) }.source);

        let success_count = lt.set_source_comp_exch_success.load(Ordering::Relaxed);
        println!(
            "set_source compare_exchange success count = {}",
            success_count
        );

        let failure_retryable_count = lt
            .set_source_comp_exch_failure_outdated
            .load(Ordering::Relaxed);
        println!(
            "set_source compare_exchange failure retryable count = {}",
            failure_retryable_count
        );

        let failure_outdated_count = lt
            .set_source_comp_exch_failure_outdated
            .load(Ordering::Relaxed);
        println!(
            "set_source compare_exchange failure outdated count ={}",
            failure_outdated_count
        );
    }
}
