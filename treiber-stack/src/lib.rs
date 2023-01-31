use std::{ptr, mem::ManuallyDrop};
use std::sync::atomic::Ordering;

use crossbeam_epoch::{self as epoch, Atomic};
use epoch::Owned;

pub struct Stack<T> {
    head: Atomic<Node<T>>,
}

struct Node<T> {
    // ManuallyDrop inhibits the compiler from automatically calling
    // the destructor for data. That's useful since we extract the data
    // when pop is called and the caller will call the drop for that.
    // But the Node is `defer_destroy`ed there as well. So if we don't
    // use ManuallyDrop, it will result in double-free error.
    data: ManuallyDrop<T>,
    prev: Atomic<Node<T>>,
}

impl<T> Node<T> {
    fn new(data: T, prev: Atomic<Node<T>>) -> Self {
        Self { data: ManuallyDrop::new(data), prev }
    }
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
            let old_head = self.head.load(Ordering::Acquire, &guard);

            // This requires minimal synchronizatoin and can be Relaxed.
            // Because if there's another push or pop before this method
            // finishes, compare_exchange is going to fail.
            node.prev.store(old_head, Ordering::Relaxed);

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
            let old_head = self.head.load(Ordering::Acquire, guard);

            // Alternatively instead of as_ref() which returns Option, we can
            // manually check for null and then use deref(). But as_ref() is
            // cleaner as it allows mixing with ? operator.
            // if old_head.is_null() {
            //     None
            // }
            // unsafe { old_head.deref() }

            let node = unsafe { old_head.as_ref()? };

            // This requires minimal synchronizatoin and can be Relaxed.
            // Because if there's another push or pop before this method
            // finishes, compare_exchange is going to fail.
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
                    let data = ptr::read(&node.data);
                    guard.defer_destroy(old_head);
                    return Some(ManuallyDrop::into_inner(data));
                }
            }
        };
    }
}
