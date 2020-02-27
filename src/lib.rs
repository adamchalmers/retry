pub mod reqw;

use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

/// This is a Future adaptor, meaning it wraps other futures, like `future::Map`.
/// It adds a configurable timeout and retries.
/// When this future is polled, it polls the inner future. If the inner futures resolves, its value
/// is run through a `test` closure.
/// If the test is successful, the value is returned with timing information.
/// If the test is unsuccessful, the future is recreated and retried.
/// Because this fail-retry loop could go on forever, you have to supply a timeout.
#[pin_project]
pub struct Retry<Fut, Test, Factory, Client, T, E>
where
    Fut: Future,
    Factory: Fn(&Client) -> Fut,
    Test: Fn(Fut::Output) -> Result<T, E>,
{
    #[pin]
    future: Fut,
    start: Option<Instant>,
    factory: Factory,
    timeout: Duration,
    client: Client,
    test: Test,
    retries: usize,
}

impl<Fut, Test, Factory, Client, T, E> Retry<Fut, Test, Factory, Client, T, E>
where
    Fut: Future,
    Factory: Fn(&Client) -> Fut,
    Test: Fn(Fut::Output) -> Result<T, E>,
{
    pub fn new(
        future: Fut,
        factory: Factory,
        timeout: Duration,
        client: Client,
        test: Test,
    ) -> Self {
        Retry {
            future,
            factory,
            timeout,
            client,
            test,
            start: None,
            retries: 0,
        }
    }
}

#[derive(Debug)]
pub enum Outcome<T, E> {
    Timeout,
    Err {
        error: E,
        retries: usize,
    },
    Ok {
        value: T,
        duration: Duration,
        retries: usize,
    },
}

impl<Fut, Test, Factory, Client, T, E> Future for Retry<Fut, Test, Factory, Client, T, E>
where
    Fut: Future,
    Factory: Fn(&Client) -> Fut,
    Test: Fn(Fut::Output) -> Result<T, E>,
{
    type Output = Outcome<T, E>;

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
            (Poll::Pending, true) => Poll::Ready(Outcome::Timeout),
            // There's still time to retry
            (Poll::Pending, false) => Poll::Pending,
            // Success!
            (Poll::Ready(Ok(resp)), _) => Poll::Ready(Outcome::Ok {
                value: resp,
                duration: elapsed,
                retries: *this.retries,
            }),
            // Failure, but there's still time for a retry
            (Poll::Ready(Err(_)), false) => {
                cx.waker().wake_by_ref();
                let new_future = (this.factory)(this.client);
                this.future.set(new_future);
                *this.retries += 1;
                Poll::Pending
            }
            // Failure, and the timeout has expired, so return the failure.
            (Poll::Ready(Err(e)), true) => Poll::Ready(Outcome::Err {
                error: e,
                retries: *this.retries,
            }),
        }
    }
}
