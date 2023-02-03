use std::future::{Future, Pending};
use std::io::Result;
use std::pin::Pin;
use std::task::{self, Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};
use tokio::time::{self, Duration, Instant, Sleep};

pub struct SlowReader<R> {
    sleep: Sleep,
    reader: R,
}

impl<R> SlowReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            sleep: time::sleep(Duration::from_millis(200)),
            reader,
        }
    }
}

impl<R> AsyncRead for SlowReader<R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> task::Poll<Result<()>> {
        let (mut sleep, reader) = unsafe {
            let this = self.get_unchecked_mut();
            (Pin::new_unchecked(&mut this.sleep), &mut this.reader)
        };

        match sleep.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => {
                let reader = Pin::new(reader);
                if let Poll::Ready(res) = reader.poll_read(cx, buf) {
                    Poll::Ready(res)
                } else {
                    sleep.reset(Instant::now() + Duration::from_millis(25));
                    Poll::Pending
                }
            }
        }
    }
}
