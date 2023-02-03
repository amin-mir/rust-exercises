use std::future::Future;
use std::pin::Pin;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;

pub struct ManualFuture<T> {
    val: Option<T>,
    inner: Arc<Mutex<ManualFutureInner>>,
    // Receive halve is given to the thread to wait for ready signal.
    ready_rx: Option<mpsc::Receiver<()>>,
}

struct ManualFutureInner {
    state: State,
    waker: Option<Waker>,
}

// impl<T> Unpin for ManualFuture<T> {}

enum State {
    NotReady,
    Ready,
    Consumed,
}

// TODO: allow determinning the final resolved value to be sent via ready.

impl<T> ManualFuture<T> {
    pub fn new(val: T) -> (Self, impl FnOnce()) {
        let (tx, rx) = mpsc::channel();

        let inner = ManualFutureInner {
            state: State::NotReady,
            waker: None,
        };

        let fut = ManualFuture {
            val: Some(val),
            inner: Arc::new(Mutex::new(inner)),
            ready_rx: Some(rx),
        };

        let ready = move || match tx.send(()) {
            Ok(_) => println!("successfully sent ready signal"),
            Err(_) => println!("ERROR failed to send ready signal ERROR"),
        };

        (fut, ready)
    }
}

impl<T> Future for ManualFuture<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner_cloned = self.inner.clone();

        // First time `poll` is called, ready_rx is taken out and replaced by None.
        // It is then sent to the thread, and the next times it will be None, that's
        // why we can't call unwrap on it here and it is sent to the thread as Option.
        let ready_rx = unsafe {
            let this = self.as_mut().get_unchecked_mut();
            this.ready_rx.take()
        };

        let mut inner = self.inner.lock().unwrap();

        match &inner.waker {
            None => {
                inner.waker = Some(cx.waker().clone());

                thread::spawn(move || match ready_rx.unwrap().recv() {
                    Ok(_) => {
                        println!("receive on the channel was ok");
                        let mut inner = inner_cloned.lock().unwrap();
                        inner.state = State::Ready;
                        inner.waker.as_ref().unwrap().wake_by_ref();
                    }
                    Err(_) => println!("ERROR receive on the channel returned ERROR"),
                });
            }
            Some(waker) => {
                if !waker.will_wake(cx.waker()) {
                    inner.waker = Some(cx.waker().clone());
                }
            }
        }

        match inner.state {
            State::NotReady => Poll::Pending,
            State::Ready => {
                inner.state = State::Consumed;

                // Lock is longer needed, so we release it.
                drop(inner);

                let val = unsafe {
                    let this = self.as_mut().get_unchecked_mut();
                    this.val.take().unwrap()
                };
                Poll::Ready(val)
                // let res = unsafe {
                //     let this = self.get_unchecked_mut();
                //     let state = std::mem::replace(&mut this.state, State::Consumed);
                //     match state {
                //         State::Ready(res) => res,
                //         _ => unreachable!(),
                //     }
                // };
                // Poll::Ready(res)
            }
            State::Consumed => unreachable!("Consumed Future polled again!"),
        }
    }
}
