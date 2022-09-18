use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, Arc};
use std::task::{Context, Poll, Waker};

pub struct ManualFuture<T> {
    inner: Arc<Mutex<ManualFutureInner<T>>>,
}

pub struct ManualFutureInner<T> {
    state: State<T>,
    waker: Option<Waker>,
}

// impl<T> Unpin for ManualFuture<T> {}

enum State<T> {
    NotReady,
    Ready(T),
    Consumed,
}

impl<T> ManualFuture<T> {
    pub fn ready(&mut self, val: T) {
        let mut inner = self.inner.lock().unwrap();
        inner.state = State::Ready(val);
        if let Some(waker) = &inner.waker {
            waker.wake_by_ref();
        }
    }
}

impl<T> Future for ManualFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.inner.lock().unwrap();
        match inner.state {
            State::NotReady => {
                if let Some(waker) = &inner.waker {
                    if !waker.will_wake(cx.waker()) {
                        inner.waker = Some(cx.waker().clone());
                        // unsafe {
                        //     self.get_unchecked_mut().waker = Some(cx.waker().clone());
                        // }
                    }
                }

                Poll::Pending
            }
            State::Ready(_) => {
                    let state = std::mem::replace(&mut inner.state, State::Consumed);
                    match state {
                        State::Ready(res) => Poll::Ready(res),
                        _ => unreachable!(),
                    }
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

impl<T> Default for ManualFutureInner<T> {
    fn default() -> Self {
        Self {
            state: State::NotReady,
            waker: None,
        }
    }
}

impl <T> Default for ManualFuture<T> {
    fn default() -> Self {
        Self {
            inner: Default::default()
        }
    }
}
