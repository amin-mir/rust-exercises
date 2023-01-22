use std::thread;
use std::sync::Arc;

use crossbeam_channel;

use treiber_stack::Stack;

fn main() {
    let stack = Arc::new(Stack::<String>::new());
    let (start_tx, start_rx) = crossbeam_channel::unbounded::<()>();

    let mut handles = vec![];
    for _ in 0..3 {
        let pusher_start_rx = start_rx.clone();
        let pusher_stack = stack.clone();
        let _ = thread::spawn(move || {
            let _ = pusher_start_rx.recv();
            let id = thread::current().id();
            for j in 0..100 {
                pusher_stack.push(format!("pusher-{:?}-{}", id, j));
            }
        });
        // handles.push(h);

        let popper_start_rx = start_rx.clone();
        let popper_stack = stack.clone();
        let h = thread::spawn(move || {
            let mut stolen = vec![];
            let _ = popper_start_rx.recv();
            let id = thread::current().id();
            for j in 0..100 {
                stolen.push(format!("popper-{:?} iteration {} => {:?}", id, j, popper_stack.pop()));
            }
            stolen
        });
        handles.push(h);
    }

    // Signal the start to other threads.
    drop(start_tx);

    let mut results = vec![];
    for h in handles {
        results.push(h.join().unwrap());
    }

    for r in results {
        for s in r {
            println!("{s}");
        }
    }

    assert!(stack.pop().is_none());
}
