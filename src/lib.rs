//! Say, for example, that you want to keep pinging a URL until it returns 200, or five seconds pass.
//! And if the URL _does_ return 200, you'd like to know how long that took.
//!
//! This library contains a Future wrapper named [`Restartable`](https://docs.rs/restartables/0.4.1/restartables/struct.Restartable.html). It wraps up a Future you want to retry, and it keeps retrying
//! the future until it passes a Test you provide. If the inner future passes the Test, then the wrapper
//! resolves your value. But if the inner future fails the Test, the wrapper will just restart the future.
//! Assuming the timeout hasn't expired.
//!
//! To do this, you need to provide three things when instantiating the wrapper:
//! - A future to poll
//! - A test, i.e. a closure which takes values from the inner future, runs a test on it, and returns Result
//! - A factory to make new futures if the previous future resolved a value that failed the test.
//!
//! The wrapper will also return some metrics, i.e. how much time elapsed before the future resolved, and
//! how many restarts were necessary.
//!
//! # Example
//!
//! ```
//! use restartables::{Failure, Restartable};
//! use std::future::Future;
//! use std::pin::Pin;
//! use std::task::{Context, Poll};
//! use std::time::Duration;
//!
//! // A Future that yields a random u16 when it resolves.
//! struct RandomNum {}
//! impl Future for RandomNum {
//!     type Output = u16;
//!     fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
//!         cx.waker().wake_by_ref();
//!         Poll::Ready(rand::random())
//!     }
//! }
//!
//! async fn print_random_even_number() {
//!     // This closure produces futures that the Restartable will poll
//!     let factory = || RandomNum {};
//!
//!     // This test returns even numbers, and fails odd numbers.
//!     let test_is_even = |num| {
//!         if num % 2 == 0 {
//!             Ok(num)
//!         } else {
//!             Err("number wasn't even")
//!         }
//!     };
//!
//!     // Wrap the inner `RandomNum` future into a `Restartable` future.
//!     let retrying = Restartable::new(
//!         factory,
//!         Some(Duration::from_millis(1)),
//!         test_is_even,
//!     );
//!
//!     match retrying.await {
//!         Ok(success) => println!(
//!             "Final number was {}, which took {}us and {} restarts to get",
//!             success.value,
//!             success.duration.as_micros(),
//!             success.restarts
//!         ),
//!         Err(Failure::Timeout) => println!("Never found an even number :("),
//!         Err(Failure::Err { error, restarts }) => {
//!             println!("Error {} after {} restarts", error, restarts)
//!         }
//!     };
//! }
//! ```

mod outcome;

pub use outcome::{Failure, Success};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

/// Wraps an inner future, restarting it until it resolves a value that passes a test, or times out.
///
/// This is a Future adaptor, meaning it wraps other futures, like [`future::map`](https://docs.rs/futures/0.3.4/futures/future/trait.FutureExt.html#method.map)
/// When this future is polled, it polls the inner future. If the inner futures resolves, its value
/// is run through a `test` closure, which is of type `Fn(Future::Output) -> Result<T,E>`.
///
/// If the test is successful, `Restartable` will resolve to a [`Success<T>`](https://docs.rs/restartables/0.4.1/restartables/struct.Success.html).
///
/// If the test is unsuccessful, the future is recreated and retried. Unless the timeout has expired,
/// in which case `Restartable` will resolve to a [`Failure<E>`](https://docs.rs/restartables/0.4.1/restartables/enum.Failure.html)
///
/// Because this fail-restart loop could go on forever, you should supply a timeout. If a `None`
/// timeout is used, then awaiting the `Restartable` might never finish (because of this fail-restart
/// loop).
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
    timeout: Option<Duration>,
    test: Test,
    restarts: usize,
}

impl<Fut, Test, Factory, T, E> Restartable<Fut, Test, Factory, T, E>
where
    Fut: Future,
    Factory: Fn() -> Fut,
    Test: Fn(Fut::Output) -> Result<T, E>,
{
    pub fn new(factory: Factory, timeout: Option<Duration>, test: Test) -> Self {
        Restartable {
            future: factory(),
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
        let timed_out = if let Some(timeout) = *this.timeout {
            elapsed > timeout
        } else {
            false
        };

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
