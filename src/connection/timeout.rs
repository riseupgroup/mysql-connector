use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

pub type TimeoutFuture = Pin<Box<dyn Future<Output = ()>>>;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Timeout<F: Future> {
    future: Pin<Box<F>>,
    delay: TimeoutFuture,
}

impl<F: Future> Timeout<F> {
    pub fn new(future: F, sleep: &dyn Fn(Duration) -> TimeoutFuture, timeout: Duration) -> Self {
        Self {
            future: Box::pin(future),
            delay: sleep(timeout),
        }
    }
}

impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, io::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if let Poll::Ready(x) = this.future.as_mut().poll(cx) {
            return Poll::Ready(Ok(x));
        }

        match this.delay.as_mut().poll(cx) {
            Poll::Ready(_) => Poll::Ready(Err(io::ErrorKind::TimedOut.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}
