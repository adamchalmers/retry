# Retryable Futures

Say, for example, that you want to keep pinging a URL until it returns 200, or five seconds pass.
And if the URL _does_ return 200, you'd like to know how long that took.

This library contains a Future wrapper. It wraps up a Future you want to retry, and it keeps retrying
the future until it passes a Test you provide. If the inner future passes the Test, then the wrapper
resolves your value. But if the inner future fails the Test, the wrapper will just restart the future.
Assuming the timeout hasn't expired.

To do this, you need to provide three things when instantiating the wrapper:
- A future to poll
- A test, i.e. a closure which takes values from the inner future, runs a test on it, and returns Result
- A factory to make new futures if the previous future resolved a value that failed the test.

The wrapper will also return some metrics, i.e. how much time elapsed before the future resolved, and
how many restarts were necessary.

If the future you're using is from [`reqwest`](docs.rs/reqwest), I've added a `reqw` module with a
convenience function to simplify setting up the wrapper.

To run the examples,
```bash
cargo run --example reqwest
```