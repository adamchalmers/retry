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
    good_example().await;
    bad_example().await;
}

async fn bad_example() {
    let url = reqwest::Url::parse("https://google.asdfasdfasdfasdf").unwrap();
    let req = reqwest::Request::new(Method::GET, url);
    let client: reqwest::Client = Default::default();
    let timeout = Duration::from_secs(2);
    let retrying = retry_future::reqw::execute(
        client,
        req,
        |r| match r {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => Err(MyError::BadStatus(resp.status())),
            Err(e) => Err(MyError::Reqwest(e)),
        },
        timeout,
    );
    println!("Running bad example");
    let outcome = retrying.await;
    println!("{:?}", outcome);
}

async fn good_example() {
    let url = reqwest::Url::parse("https://google.com").unwrap();
    let req = reqwest::Request::new(Method::GET, url);
    let client: reqwest::Client = Default::default();
    let timeout = Duration::from_secs(2);
    let retrying = retry_future::reqw::execute(
        client,
        req,
        |r| match r {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => Err(MyError::BadStatus(resp.status())),
            Err(e) => Err(MyError::Reqwest(e)),
        },
        timeout,
    );
    println!("Running good example");
    let outcome = retrying.await;
    println!("{:?}", outcome);
}
