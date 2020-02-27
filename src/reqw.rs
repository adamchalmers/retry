//! Convenience functions for using `reqwest` futures with `Restartable`. Requires the
//! `use_reqwest` feature to be enabled.
use super::{Failure, Restartable, Success};
use std::time::Duration;

/// Keeps resending a request until its response passes the test, or it times out. Panics if the
/// Request can't be cloned.
///
/// If the timeout is None, this might never resolve.
///
/// ```
/// // The `reqw` module is only included if the `use_reqwest` feature is enabled.
/// use restartables::reqw::execute;
///
/// let url = reqwest::Url::parse("https://google.com").unwrap();
/// let req = reqwest::Request::new(Method::GET, url);
/// let client: reqwest::Client = Default::default();
///
/// let fut = execute(
///     &client,
///     &req,
///     |r| match r {
///         // If the response is 200, resolve this future and return ()
///         Ok(resp) if resp.status().is_success() => Ok(()),
///         // Otherwise, return an error and restart the request.
///         Ok(resp) => Err(MyError::BadStatus(resp.status())),
///         Err(e) => Err(MyError::Reqwest(e)),
///     },
///     Some(Duration::from_secs(2)), // timeout after 2 seconds
/// );
/// println!("Pinging {}", url_to_hit);
/// let outcome = fut.await;
/// println!("{:?}", outcome);
/// ```
pub async fn execute<T, E, Test>(
    client: &reqwest::Client,
    req: &reqwest::Request,
    test: Test,
    timeout: Option<Duration>,
) -> Result<Success<T>, Failure<E>>
where
    Test: Fn(Result<reqwest::Response, reqwest::Error>) -> Result<T, E>,
{
    let factory = || client.execute(req.try_clone().unwrap());
    let retrying = Restartable::new(factory, timeout, test);
    let outcome = retrying.await;
    outcome
}
