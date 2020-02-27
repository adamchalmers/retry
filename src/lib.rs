mod outcome;
#[cfg(use_reqwest)]
pub mod reqw;

pub use outcome::{Failure, Success};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

/// This is a Future adaptor, meaning it wraps other futures, like `future::Map`.
/// It adds a configurable timeout and restarts.
/// When this future is polled, it polls the inner future. If the inner futures resolves, its value
/// is run through a `test` closure.
/// If the test is successful, the value is returned with timing information.
/// If the test is unsuccessful, the future is recreated and retried.
/// Because this fail-restart loop could go on forever, you have to supply a timeout.
#[pin_project]
pub struct Restartable<Fut, Test, Factory, T, E>
where
    Fut: Future,
    Factory: Fn() -> Fut,
    Test: Fn(Fut::Output) -> Result<T, E>,
{
    #[pin]
    future: Fut,
    start: Option<Instant>,
    factory: Factory,
    timeout: Duration,
    test: Test,
    restarts: usize,
}

impl<Fut, Test, Factory, T, E> Restartable<Fut, Test, Factory, T, E>
where
    Fut: Future,
    Factory: Fn() -> Fut,
    Test: Fn(Fut::Output) -> Result<T, E>,
{
    pub fn new(future: Fut, factory: Factory, timeout: Duration, test: Test) -> Self {
        Restartable {
            future,
            factory,
            timeout,
            test,
            start: None,
            restarts: 0,
        }
    }
}

impl<Fut, Test, Factory, T, E> Future for Restartable<Fut, Test, Factory, T, E>
where
    Fut: Future,
    Factory: Fn() -> Fut,
    Test: Fn(Fut::Output) -> Result<T, E>,
{
    type Output = Result<Success<T>, Failure<E>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut this = self.project();
        let start = this.start.get_or_insert_with(Instant::now);

        // Call the inner poll, run the result through `self.test`.
        let inner_poll = this.future.as_mut().poll(cx).map(this.test);

        // Measure timing
        let elapsed = start.elapsed();
        let timed_out = elapsed > *this.timeout;

        match (inner_poll, timed_out) {
            // Inner future timed out without ever resolving
            (Poll::Pending, true) => Poll::Ready(Err(Failure::Timeout)),
            // There's still time to poll again
            (Poll::Pending, false) => Poll::Pending,
            // Success!
            (Poll::Ready(Ok(resp)), _) => Poll::Ready(Ok(Success {
                value: resp,
                duration: elapsed,
                restarts: *this.restarts,
            })),
            // Failure, but there's still time to restart the future and try again.
            (Poll::Ready(Err(_)), false) => {
                cx.waker().wake_by_ref();
                let new_future = (this.factory)();
                this.future.set(new_future);
                *this.restarts += 1;
                Poll::Pending
            }
            // Failure, and the timeout has expired, so return the failure.
            (Poll::Ready(Err(e)), true) => Poll::Ready(Err(Failure::Err {
                error: e,
                restarts: *this.restarts,
            })),
        }
    }
}
