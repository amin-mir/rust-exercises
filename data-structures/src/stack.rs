pub struct Stack<T> {
    head: Option<Box<Entry<T>>>,
}

pub struct Stack2<T> {
    head: Option<Entry<T>>,
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

impl<T> Stack2<T> {
    pub fn new() -> Stack2<T> {
        Stack2 { head: None }
    }

    pub fn push(&mut self, val: T) {
        let mut entry = Entry::new(val);
        if let Some(top) = std::mem::replace(&mut self.head, None) {
            entry.prev = Some(Box::new(top));
        }
        self.head = Some(entry);
    }

    pub fn pop(&mut self) -> Option<T> {
        match std::mem::replace(&mut self.head, None) {
            None => None,
            Some(head) => {
                self.head = match head.prev {
                    None => None,
                    Some(val) => Some(*val),
                };
                Some(head.val)
            }
        }
    }

    pub fn peek(&self) -> Option<&T> {
        match self.head {
            None => None,
            Some(ref head) => Some(&head.val),
        }
    }

    pub fn peek_nth(&self, n: usize) -> Option<&T> {
        if self.head.is_none() {
            return None;
        }

        let head = self.head.as_ref().unwrap();
        if n == 0 {
            return Some(&head.val);
        }

        let mut current = head.prev.as_ref();
        for _ in 1..n {
            match current {
                None => return None,
                Some(e) => current = (*e).prev.as_ref(),
            }
        }

        match current {
            None => None,
            Some(e) => Some(&(*e).val),
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
    fn stack_last_in_first_out() {
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

    #[test]
    fn stack2_peek() {
        let mut stack = Stack2::new();

        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(Some(&3), stack.peek());

        assert_eq!(Some(&1), stack.peek_nth(2));
        assert_eq!(Some(&2), stack.peek_nth(1));
        assert_eq!(Some(&3), stack.peek_nth(0));
        assert_eq!(None, stack.peek_nth(3));
    }
}
