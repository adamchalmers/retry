extern crate restartables;
use restartables::{Failure, Restartable};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

// A Future that yields a random u16 when it resolves.
struct RandomNum {}
impl Future for RandomNum {
    type Output = u16;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        cx.waker().wake_by_ref();
        Poll::Ready(rand::random())
    }
}

#[tokio::main]
async fn main() {
    let factory = || RandomNum {};
    let future = factory();
    // This test returns even numbers, and fails odd numbers.
    let test_is_even = |num| {
        if num % 2 == 0 {
            Ok(num)
        } else {
            Err("number wasn't even")
        }
    };

    // Wrap the inner `RandomNum` future into a `Restartable` future.
    let retrying = Restartable::new(
        future,
        factory,
        Some(Duration::from_millis(1)),
        test_is_even,
    );

    match retrying.await {
        Ok(success) => println!(
            "Final number was {}, which took {}us and {} restarts to get",
            success.value,
            success.duration.as_micros(),
            success.restarts
        ),
        Err(Failure::Timeout) => println!("Never found an even number :("),
        Err(Failure::Err { error, restarts }) => {
            println!("Error {} after {} restarts", error, restarts)
        }
    };
}
