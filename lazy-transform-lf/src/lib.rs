// LazyTransform(transformFn)
// set_source gets a source which can be passed to transformFn to get the
// new value which should be cached and served in get_transformed. The
// calculation should not happen until get_transformed is called.

use std::fmt::Debug;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use seize::{reclaim, Collector, Guard, Linked};

pub struct LazyTransform<F, T: Debug> {
    collector: Collector,
    transform: F,
    seq_counter: AtomicUsize,
    val: AtomicPtr<Linked<T>>,
    source_ctx: AtomicPtr<Linked<SourceContext<T>>>,

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

struct SourceContext<T: Debug> {
    seq: usize,
    in_progress: bool,
    source: T,
}

impl<T: Debug> SourceContext<T> {
    fn new(seq: usize, source: T) -> Self {
        Self {
            seq,
            in_progress: false,
            source,
        }
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
        // will have a reference to self.
        let guard = unsafe { Guard::unprotected() };

        // Ordering is irrelevant here because Atomics are loaded immediately
        // anyways due to the special guard that we use here.
        let val = guard.protect(&self.val, Ordering::Relaxed);
        let src_ctx = guard.protect(&self.source_ctx, Ordering::Relaxed);

        if !val.is_null() {
            unsafe { guard.retire(val, reclaim::boxed::<T>); }
        }
        if !src_ctx.is_null() {
            unsafe { guard.retire(src_ctx, reclaim::boxed::<SourceContext<T>>); }
        }
    }
}

impl<F, T> LazyTransform<F, T>
where
    T: Debug,
    F: Fn(T) -> T,
{
    pub fn new(transform: F) -> Self {
        Self {
            collector: Collector::new(),
            transform,
            seq_counter: AtomicUsize::new(0),
            val: AtomicPtr::default(),
            source_ctx: AtomicPtr::default(),
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
            .link_boxed(SourceContext::new(new_seq, source));

        let guard = self.collector.enter();
        let mut cur_src = guard.protect(&self.source_ctx, Ordering::Acquire);

        loop {
            // TODO: should failure ordering be Acquire? In case of success, cur
            // is guaranteed to be the actual previous value which can be retired now.
            // In case of failure, (a) we need to compare the sequence numebrs between
            // new and cur, and (b) we might need to retry the CAS, thus Acquire is used.
            match self.source_ctx.compare_exchange(
                cur_src,
                new_src,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(cur) => unsafe {
                    self.set_source_comp_exch_success.fetch_add(1, Ordering::Relaxed);
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
                    let cur_ref = unsafe { &*cur };

                    // Impossible for two threads to acquire the same sequence number.
                    assert_ne!(new_seq, cur_ref.seq);

                    if new_seq > cur_ref.seq {
                        self.set_source_comp_exch_failure_retryable.fetch_add(1, Ordering::Relaxed);
                        // We have the latest data, so we should over-write.
                        cur_src = cur;
                    } else {
                        self.set_source_comp_exch_failure_outdated.fetch_add(1, Ordering::Relaxed);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;

    const CONC_CALL_COUNT: usize = 1_000;

    fn string_transform(s: String) -> String {
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
        let lt = LazyTransform::new(|src: (String, usize)| src);

        thread::scope(|s| {
            for _ in 0..20 {
                s.spawn(|| {
                    for i in 0..CONC_CALL_COUNT {
                        lt.set_source((format!("{:?}", thread::current().id()), i));
                    }
                });
            }
        });

        let src_ctx = lt.source_ctx.load(Ordering::Relaxed);
        println!("{:?}", unsafe { &(*src_ctx) }.source);

        let success_count = lt.set_source_comp_exch_success.load(Ordering::Relaxed);
        println!("set_source compare_exchange success count = {}", success_count);
        
        let failure_retryable_count = lt.set_source_comp_exch_failure_outdated.load(Ordering::Relaxed);
        println!("set_source compare_exchange failure retryable count = {}", failure_retryable_count);

        let failure_outdated_count = lt.set_source_comp_exch_failure_outdated.load(Ordering::Relaxed);
        println!("set_source compare_exchange failure outdated count ={}", failure_outdated_count);
    }
}
