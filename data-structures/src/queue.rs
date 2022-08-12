use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

type OptNode<T> = Option<Rc<RefCell<Node<T>>>>;

#[derive(Debug)]
pub struct Queue<T: Debug + Default> {
    next_id: usize,
    head: OptNode<T>,
    tail: OptNode<T>,
}

#[derive(Debug)]
struct Node<T: Debug> {
    id: usize,
    val: T,
    prev: OptNode<T>,
}

impl<T: Debug> Drop for Node<T> {
    fn drop(&mut self) {
        println!("Node dropped = {:?}", self);
    }
}

impl<T: Debug> Node<T> {
    fn new(id: usize, val: T) -> Self {
        Self {
            id,
            val,
            prev: None,
        }
    }

    fn rc(id: usize, val: T) -> Rc<RefCell<Node<T>>> {
        Rc::new(RefCell::new(Node::new(id, val)))
    }
}

impl<T: Debug + Default> Queue<T> {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            head: None,
            tail: None,
        }
    }

    pub fn push(&mut self, val: T) {
        match self.tail {
            None => {
                let tail = Node::rc(self.next_id, val);
                self.head = Some(tail.clone());
                self.tail = Some(tail);
            }
            Some(_) => {
                let old_tail = self.tail.take().unwrap();
                let mut old_tail = old_tail.borrow_mut();

                let tail = Node::rc(self.next_id, val);
                old_tail.prev = Some(Rc::clone(&tail));
                self.tail = Some(tail);
            }
        }

        self.next_id = self.next_id + 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        match self.head {
            None => None,
            Some(_) => {
                // Take ownership of Queue head and temporarily leave it as None.
                let old_head = self.head.take().unwrap();

                // If strong_count > 1, head and tail are poiting to the same
                // Node so we should drop tail as well. Otherwise Rc::try_unwrap
                // will panic.
                if Rc::strong_count(&old_head) > 1 {
                    let _ = self.tail.take();
                }

                // Unwrap Rc.
                let old_head = Rc::try_unwrap(old_head).unwrap();

                // Unwrap Refcell.
                let mut old_head = old_head.into_inner();

                self.head = old_head.prev.take();
                Some(std::mem::take(&mut old_head.val))
            }
        }
    }

    pub fn pop2(&mut self) -> Option<T> {
        match self.head {
            None => None,
            Some(_) => {
                // head and tail are poiting to the same Node so we should drop
                // tail reference to the that Node.
                if self.is_single_node() {
                    let _ = std::mem::replace(&mut self.tail, None);
                }

                // Take ownership of Queue head and temporarily leave it as None.
                let old_head = std::mem::replace(&mut self.head, None).unwrap();
                let mut old_head = old_head.borrow_mut();

                self.head = old_head.prev.take();
                Some(std::mem::take(&mut old_head.val))
            }
        }
    }

    // Returns true if the queue only contains a single node.
    // In which case the head and tail point to the same node.
    fn is_single_node(&self) -> bool {
        self.head.as_ref()
            .zip(self.tail.as_ref())
            .map_or(false, |(head, tail)| head.borrow().id == tail.borrow().id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_should_work() {
        let mut q = Queue::new();

        q.push("elem1".to_owned());
        q.push("elem2".to_owned());

        assert_eq!(Some("elem1".to_owned()), q.pop());
        assert_eq!(Some("elem2".to_owned()), q.pop());
        assert_eq!(None, q.pop());

        q.push("elem3".to_owned());
        q.push("elem4".to_owned());
        assert_eq!(Some("elem3".to_owned()), q.pop());
        assert_eq!(Some("elem4".to_owned()), q.pop());
        assert_eq!(None, q.pop());
    }

    #[test]
    fn push_pop2_should_work() {
        let mut q = Queue::new();

        q.push("elem1".to_owned());
        q.push("elem2".to_owned());

        assert_eq!(Some("elem1".to_owned()), q.pop2());
        assert_eq!(Some("elem2".to_owned()), q.pop2());
        assert_eq!(None, q.pop2());

        q.push("elem3".to_owned());
        q.push("elem4".to_owned());
        assert_eq!(Some("elem3".to_owned()), q.pop2());
        assert_eq!(Some("elem4".to_owned()), q.pop2());
        assert_eq!(None, q.pop2());
    }
}
