//! Michael-Scott Queue:
//! At the beginning the tail and head are pointing to the same
//! dummy node. The fact that the head and tail are pointing to the
//! same node means that the queue is empty.
//! `push/enqueue` to the tail. `pop/dequeue` from the head.
//! There's always a dummy node in the queue which head points to.
//!
//! Push:
//! `cas` tail.next to point to the new node.
//! This cas should succeed. If it fails we retry.
//! tail.next should be null. if it is not null, it means that it's
//! lagging behind and we need to do cleanup.
//! `cas` tail to point to the new node. if this fails, we don't care.
//! Because the only way that this fails is if someone else did this already.
//! In case of failure we generate more cleanup for future operations
//! possibly on different threads to take care of.
//! Any thread can help with this (cleanup) that comes along and realizes that
//! tail pointer is poiting to a node whose next pointer is not null.
//!
//! Pop:
//! Read data from the next pointer of the dummy node (head.next.data).
//! If dummy.next is null, the queue is empty.
//! After reading the data, dummy.next becomes the new dummy/head node
//! thus `cas` the head to point to dummy.next. Then drop the dummy node.
use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::sync::atomic::Ordering;

use crossbeam_epoch::{self, Atomic, Guard, Owned, Shared};
use crossbeam_utils::CachePadded;

pub struct Queue<T: Debug> {
    head: CachePadded<Atomic<Node<T>>>,
    tail: CachePadded<Atomic<Node<T>>>,
}
pub struct Node<T> {
    data: MaybeUninit<T>,
    next: Atomic<Node<T>>,
}

// TODO: should T be Send?
unsafe impl<T: Debug> Send for Queue<T> {}
unsafe impl<T: Debug> Sync for Queue<T> {}

impl<T: Debug> Drop for Queue<T> {
    fn drop(&mut self) {
        let head = std::mem::take(&mut *self.head);

        let head = unsafe { head.into_owned() }.into_box();
        let mut next = unsafe { head.next.try_into_owned() };

        while let Some(current) = next {
            // drop is called automatically for Box. Another property
            // of Box is it gets dereferenced to its target, meaning
            // that we get &mut of Node and can call assume_init(_drop).
            let current = current.into_box();

            // Drop the data.
            let _ = unsafe { current.data.assume_init() };
            // println!("dropping {:?}", data);

            next = unsafe { current.next.try_into_owned() };
        }
    }
}

impl<T: Debug> Queue<T> {
    pub fn new() -> Self {
        let dummy = Owned::new(Node {
            data: MaybeUninit::uninit(),
            next: Atomic::null(),
        });

        // Owned is not Copy, so we need to convert it to Shared to be
        // able to have both head and tail point to the same dummy node.
        let dummy = dummy.into_shared(unsafe { crossbeam_epoch::unprotected() });

        Self {
            head: CachePadded::new(dummy.into()),
            tail: CachePadded::new(dummy.into()),
        }
    }

    pub fn is_empty(&self) -> bool {
        let guard = &crossbeam_epoch::pin();
        let head = self.head.load(Ordering::Acquire, guard);

        // We know that tail cannot be null.
        let next = unsafe { head.deref() }.next.load(Ordering::Acquire, guard);
        next.is_null()
    }

    pub fn push(&self, data: T) {
        let guard = &crossbeam_epoch::pin();

        let new = Owned::new(Node {
            data: MaybeUninit::new(data),
            next: Atomic::null(),
        })
        .into_shared(guard);

        loop {
            let tail = self.tail.load(Ordering::Acquire, guard);

            // tail can never be null, because there's at least the dummy node.
            let tail_ref = unsafe { tail.deref() };

            let next = tail_ref.next.load(Ordering::Acquire, guard);

            // Help with the cleanup when tail is lagging behind.
            if !next.is_null() {
                // We don't care whether success or failure. If it succeeds it means
                // that we moved the tail to the tail.next and now we need the next
                // for the new tail so start the loop again. If we failed, it means
                // someone else has done this for us, so we need to load the tail and
                // tail.next again.
                let _ = self.tail.compare_exchange(
                    tail,
                    next,
                    Ordering::Release,
                    Ordering::Relaxed,
                    guard,
                );
                continue;
            }

            // Change tail.next to point to new if still null.
            if tail_ref
                .next
                .compare_exchange(
                    Shared::null(),
                    new,
                    Ordering::Release,
                    Ordering::Relaxed,
                    guard,
                )
                .is_err()
            {
                // If it fails, it means that tail.next is no longer null.
                continue;
            }

            // change tail to point to next. We don't care about the result of this
            // operation. If it fails, it means another thread helped with the cleanup
            // and moved the tail already.
            let _ =
                self.tail
                    .compare_exchange(tail, new, Ordering::Release, Ordering::Relaxed, guard);
            break;
        }
    }

    fn try_pop(&self, guard: &Guard) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire, guard);
            // We know for a fact that head is the dummy node so it cannot be empty.
            // Alternatively, we could use as_ref with ? which is compatible with the
            // function signature as well.
            let next = unsafe { head.deref() }.next.load(Ordering::Acquire, guard);

            // If head doesn't have a next anymore (someone popped in the meanwhile)
            // the list is empty.
            let next_ref = unsafe { next.as_ref() }?;

            // TODO: can we use Relaxed here?
            let tail = self.tail.load(Ordering::Acquire, guard);

            // if head and tail are the same
            // and tail.next is not null, move the tail.
            // otherwise the list is empty.
            if head == tail {
                // We will continue in case of success or failure. In case of failure
                // it means someone else move the tail futher, by a push or something.
                let _ = self.tail.compare_exchange(
                    tail,
                    next,
                    Ordering::Release,
                    Ordering::Relaxed,
                    guard,
                );
            }

            if self
                .head
                .compare_exchange(head, next, Ordering::Release, Ordering::Relaxed, guard)
                .is_err()
            {
                // If head is not the same, we need to retry.
                continue;
            }

            // SAFETY: We've successfully set tail.next to be the new head/dummy
            // node. No one is going to read the data from that anymore.
            // We still have the guard so it is not going to be freed either.
            let data = unsafe { next_ref.data.assume_init_read() };
            unsafe { guard.defer_destroy(head) };
            return Some(data);
        }
    }

    pub fn pop(&self) -> T {
        let guard = &crossbeam_epoch::pin();
        loop {
            if let Some(data) = self.try_pop(guard) {
                return data;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    const CONC_COUNT: i64 = 1_000_000;

    #[test]
    fn push_try_pop_1() {
        let q: Queue<i64> = Queue::new();
        assert!(q.is_empty());

        q.push(37);
        assert!(!q.is_empty());
        assert_eq!(try_pop(&q), Some(37));
        assert!(q.is_empty());
    }

    #[test]
    fn push_try_pop_2() {
        let q: Queue<i64> = Queue::new();
        assert!(q.is_empty());

        q.push(37);
        q.push(48);
        assert_eq!(try_pop(&q), Some(37));
        assert!(!q.is_empty());
        assert_eq!(try_pop(&q), Some(48));
        assert!(q.is_empty());
    }

    #[test]
    fn push_try_pop_many_seq() {
        let q: Queue<i64> = Queue::new();
        assert!(q.is_empty());

        for i in 0..200 {
            q.push(i)
        }
        assert!(!q.is_empty());

        for i in 0..200 {
            assert_eq!(try_pop(&q), Some(i));
        }

        assert!(q.is_empty());
    }

    #[test]
    fn push_pop_1() {
        let q: Queue<i64> = Queue::new();
        assert!(q.is_empty());

        q.push(37);
        assert!(!q.is_empty());
        assert_eq!(q.pop(), 37);
        assert!(q.is_empty());
    }

    #[test]
    fn push_pop_2() {
        let q: Queue<i64> = Queue::new();
        q.push(37);
        q.push(48);
        assert_eq!(q.pop(), 37);
        assert_eq!(q.pop(), 48);
    }

    #[test]
    fn push_pop_many_seq() {
        let q: Queue<i64> = Queue::new();
        assert!(q.is_empty());

        for i in 0..200 {
            q.push(i)
        }
        assert!(!q.is_empty());

        for i in 0..200 {
            assert_eq!(q.pop(), i);
        }
        assert!(q.is_empty());
    }

    #[test]
    fn push_try_pop_many_spsc() {
        let q: Queue<i64> = Queue::new();
        assert!(q.is_empty());

        thread::scope(|s| {
            s.spawn(|| {
                let mut next = 0;

                while next < CONC_COUNT {
                    if let Some(elem) = try_pop(&q) {
                        assert_eq!(elem, next);
                        next += 1;
                    }
                }
            });

            for i in 0..CONC_COUNT {
                q.push(i)
            }
        });
    }

    #[test]
    fn push_try_pop_many_spmc() {
        fn recv(q: &Queue<i64>) {
            let mut cur = -1;
            for _ in 0..CONC_COUNT {
                if let Some(elem) = try_pop(&q) {
                    assert!(elem > cur);
                    cur = elem;

                    if cur == CONC_COUNT - 1 {
                        break;
                    }
                }
            }
        }

        let q: Queue<i64> = Queue::new();
        assert!(q.is_empty());

        // All scoped threads that haven't been manually joined
        // are automatically joined at the end.
        thread::scope(|s| {
            for _ in 0..3 {
                s.spawn(|| recv(&q));
            }

            s.spawn(|| {
                for i in 0..CONC_COUNT {
                    q.push(i);
                }
            });
        });
    }

    #[test]
    fn push_try_pop_many_mpmc() {
        #[derive(Debug)]
        enum LR {
            Left(i64),
            Right(i64),
        }

        let q: Queue<LR> = Queue::new();
        assert!(q.is_empty());

        thread::scope(|s| {
            s.spawn(|| {
                for i in 0..CONC_COUNT {
                    q.push(LR::Left(i))
                }
            });

            s.spawn(|| {
                for i in 0..CONC_COUNT {
                    q.push(LR::Right(i))
                }
            });

            for _ in 0..2 {
                s.spawn(|| {
                    let mut vl = vec![];
                    let mut vr = vec![];

                    for _ in 0..CONC_COUNT {
                        match try_pop(&q) {
                            Some(LR::Left(x)) => vl.push(x),
                            Some(LR::Right(x)) => vr.push(x),
                            _ => {}
                        }
                    }

                    let mut vl2 = vl.clone();
                    let mut vr2 = vr.clone();
                    vl2.sort();
                    vr2.sort();

                    assert_eq!(vl, vl2);
                    assert_eq!(vr, vr2);
                });
            }
        });
    }

    #[test]
    fn push_pop_many_spsc() {
        let q: Queue<i64> = Queue::new();

        thread::scope(|s| {
            s.spawn(|| {
                let mut next = 0;
                while next < CONC_COUNT {
                    assert_eq!(q.pop(), next);
                    next += 1;
                }
            });

            for i in 0..CONC_COUNT {
                q.push(i)
            }
        });
        assert!(q.is_empty());
    }

    #[test]
    fn is_empty_dont_pop() {
        let q: Queue<i64> = Queue::new();
        q.push(20);
        q.push(20);
        assert!(!q.is_empty());
        assert!(!q.is_empty());
        assert!(try_pop(&q).is_some());
    }

    // try_pop makes calling try_pop on the Queue convenient.
    // Because it expected a &Guard and this function takes
    // care of providing that.
    fn try_pop<T: Debug>(q: &Queue<T>) -> Option<T> {
        let guard = &crossbeam_epoch::pin();
        q.try_pop(guard)
    }
}
