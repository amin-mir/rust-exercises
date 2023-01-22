use std::thread;

use treiber_stack::Stack;

fn main() {
    let stack = Stack::<String>::new();

    thread::scope(|s| {
        let mut handles = vec![];
        for _ in 0..3 {
            let h = s.spawn(|| {
                let id = thread::current().id();
                for j in 0..100 {
                    stack.push(format!("pusher-{:?}-{}", id, j));
                }
            });
            handles.push(h);

            let h = s.spawn(|| {
                let id = thread::current().id();
                for j in 0..100 {
                    println!("popper-{:?} iteration {} => {:?}", id, j, stack.pop());
                }
            });
            handles.push(h);
        }
        for h in handles {
            h.join().unwrap();
        }
    });

    assert!(stack.pop().is_none());
}
