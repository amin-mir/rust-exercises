pub struct Stack<T> {
    head: Option<Box<Entry<T>>>,
}

struct Entry<T> {
    val: T,
    prev: Option<Box<Entry<T>>>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self { head: None }
    }

    pub fn push(&mut self, val: T) {
        match self.head {
            None => self.head = Some(Box::new(Entry::new(val))),
            Some(_) => {
                let mut new_head = Box::new(Entry::new(val));
                new_head.prev = self.head.take();
                self.head = Some(new_head);
            }
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        match self.head {
            None => None,
            Some(_) => {
                let mut head = self.head.take().unwrap();
                // if there's another entry down the stack, make it head.
                self.head = head.prev.take();
                Some(head.val)
            }
        }
    }
}

impl<T> Entry<T> {
    pub fn new(val: T) -> Self {
        Self { val, prev: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_in_first_out() {
        let mut stack = Stack::new();

        stack.push(1);
        assert_eq!(Some(1), stack.pop());
        assert_eq!(None, stack.pop());

        stack.push(2);
        stack.push(3);
        assert_eq!(Some(3), stack.pop());
        assert_eq!(Some(2), stack.pop());
        assert_eq!(None, stack.pop());
    }
}
