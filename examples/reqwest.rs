extern crate restartables;
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
    example("https://google.com").await;
    example("https://google.asdfasdfasdf").await;
}

async fn example(url_to_hit: &'static str) {
    let url = reqwest::Url::parse(url_to_hit).unwrap();
    let req = reqwest::Request::new(Method::GET, url);
    let client: reqwest::Client = Default::default();
    let timeout = Duration::from_secs(2);
    // The `reqw` module is only included if the `use_reqwest` feature is enabled.
    let retrying = restartables::reqw::execute(
        &client,
        &req,
        |r| match r {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => Err(MyError::BadStatus(resp.status())),
            Err(e) => Err(MyError::Reqwest(e)),
        },
        Some(timeout),
    );
    println!("Pinging {}", url_to_hit);
    let outcome = retrying.await;
    println!("{:?}", outcome);
}
