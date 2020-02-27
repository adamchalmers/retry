use super::{Outcome, Retry};
use std::time::Duration;
pub async fn execute<T, E, Test>(
    client: &reqwest::Client,
    req: reqwest::Request,
    test: Test,
    timeout: Duration,
) -> Outcome<T, E>
where
    Test: Fn(Result<reqwest::Response, reqwest::Error>) -> Result<T, E>,
{
    let factory = || client.execute(req.try_clone().unwrap());
    let future = client.execute(req.try_clone().unwrap());
    let retrying = Retry::new(future, factory, timeout, test);
    let outcome = retrying.await;
    outcome
}
