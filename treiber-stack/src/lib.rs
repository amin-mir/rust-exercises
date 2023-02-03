use std::fmt::Debug;
use std::mem::{self, ManuallyDrop};
use std::ptr;
use std::sync::atomic::Ordering;

use crossbeam_epoch::{self as epoch, Atomic};
use epoch::Owned;

pub struct Stack<T: Debug> {
    head: Atomic<Node<T>>,
}

// TODO: should T be Send as well?
unsafe impl<T: Debug> Send for Stack<T> {}
unsafe impl<T: Debug> Sync for Stack<T> {}

impl<T: Debug> Drop for Stack<T> {
    fn drop(&mut self) {
        println!("inside drop");
        let guard = &epoch::pin();

        let mut current = mem::replace(&mut self.head, Atomic::null());
        unsafe {
            while let Some(node) = current.try_into_owned() {
                // Alternatively, we can try the following, but we'll have to use
                // std::ptr::read because node is Owned which doesn't implement Copy.
                // let data = ptr::read(&mut node.data);
                // drop(ManuallyDrop::into_inner(data));

                let node = node.into_box();
                println!("dropping {:?}", node.data);
                drop(ManuallyDrop::into_inner(node.data));

                let node = node.prev.load(Ordering::Relaxed, guard);
                current = Atomic::from(node);
            }
        }
    }
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
        Self {
            data: ManuallyDrop::new(data),
            prev,
        }
    }
}

impl<T: Debug> Stack<T> {
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
            // finishes, compare_exchange is going to fail. This is a sign that
            // we can replace Atomic with Shared. But Shared is only valid for
            // the lifetime of guard. So we should convert it to *const Node
            // and store that instead of an Atomic. Then we can do Shared::from
            // to go back to having a shared.
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
            // finishes, compare_exchange is going to fail. This is a sign that
            // we can replace Atomic with Shared. But Shared is only valid for
            // the lifetime of guard. So we should convert it to *const Node
            // and store that instead of an Atomic. Then we can do Shared::from
            // to go back to having a shared.
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
        }
    }
}
