extern crate retry_future;
use reqwest;
use reqwest::Method;
use std::default::Default;
use std::time::Duration;

#[derive(Debug)]
enum MyError {
    Reqwest(reqwest::Error),
    BadStatus(reqwest::StatusCode),
}

#[tokio::main]
async fn main() {
    let url = reqwest::Url::parse("https://google.asdfasdfasdfasdf").unwrap();
    let req = reqwest::Request::new(Method::GET, url);
    let client: reqwest::Client = Default::default();
    let timeout = Duration::from_secs(2);
    let retrying = retry_future::execute(
        client,
        req,
        |r| match r {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => Err(MyError::BadStatus(resp.status())),
            Err(e) => Err(MyError::Reqwest(e)),
        },
        timeout,
    );
    let outcome = retrying.await;
    println!("{:?}", outcome);
}
