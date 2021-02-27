use bytes;
use reqwest;
use std::thread;
use warp::Filter;

#[tokio::main]
async fn main() {
    // let http_client = reqwest::Client::new();
    // let http_client = warp::any().map(move || http_client.clone());
    let cmd_post = warp::post()
        .and(warp::path("cmd"))
        .and(warp::any().map(move || reqwest::Client::new()))
        .and(warp::body::bytes())
        .and_then(
            |http_client: reqwest::Client, bytes: bytes::Bytes| async move {
                let cmd = std::str::from_utf8(&bytes[..]).unwrap();
                let is_ok = http_client
                    .post("http://localhost:3030/print")
                    .body(format!("{}\n:", cmd))
                    .send()
                    .await;
                match is_ok {
                    Ok(_) => Ok(""),
                    Err(_) => Err(warp::reject::not_found()),
                }
            },
        );
    cmd_post.or(cmd_post);
    warp::serve(cmd_post).run(([127, 0, 0, 1], 3031)).await;
}
