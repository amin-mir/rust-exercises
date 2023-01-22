use std::ptr;
use std::sync::atomic::Ordering;

use crossbeam_epoch::{self as epoch, Atomic};
use epoch::Owned;

struct Node<T> {
    data: T,
    prev: Atomic<Node<T>>,
}

impl<T> Node<T> {
    fn new(data: T, prev: Atomic<Node<T>>) -> Self {
        Self { data, prev }
    }
}

pub struct Stack<T> {
    head: Atomic<Node<T>>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self {
            head: Atomic::null(),
        }
    }

    pub fn push(&self, data: T) {
        let node = Node::new(data, Atomic::null());
        let mut node = Owned::new(node);

        let guard = epoch::pin();

        loop {
            // TODO: what should the ordering be here?
            let old_head = self.head.load(Ordering::Acquire, &guard);
            node.prev.store(old_head, Ordering::Relaxed);

            // TODO: what should the orderings be here?
            match self.head.compare_exchange(
                old_head,
                node,
                Ordering::Release,
                Ordering::Relaxed,
                &guard,
            ) {
                Ok(_) => break,
                Err(e) => node = e.new,
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        let guard = &epoch::pin();

        loop {
            // What should the ordering be here?
            let old_head = self.head.load(Ordering::Acquire, guard);

            let node = unsafe { old_head.as_ref()? };
            let new_head = node.prev.load(Ordering::Relaxed, guard);
            let result = self.head.compare_exchange(
                old_head,
                new_head,
                Ordering::Release,
                Ordering::Relaxed,
                guard,
            );
            if result.is_ok() {
                unsafe {
                    guard.defer_destroy(old_head);
                    return Some(ptr::read(&node.data as *const T));
                }
            }
        };
    }
}
