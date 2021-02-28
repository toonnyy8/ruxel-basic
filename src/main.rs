#![feature(allocator_api, new_uninit)]
use byteorder::{LittleEndian, WriteBytesExt};
use bytes::{self, Buf, BufMut};
use reqwest;
use std::sync::Arc;
use std::{convert::TryFrom, io::Read};
use std::{fmt::Write, thread};
use tokio::sync::Mutex;
use warp::{hyper::body::HttpBody, Filter};

fn get_i32() -> Arc<Mutex<i32>> {
    Arc::new(Mutex::new(0))
}

#[tokio::main]
async fn main() {
    let cmd_post: warp::filters::BoxedFilter<(
        Result<warp::http::Response<bytes::Bytes>, warp::http::Error>,
    )> = warp::post()
        .and(warp::path("cmd"))
        .and(warp::path::end())
        .and(warp::any().map(move || reqwest::Client::new()))
        .and(warp::body::bytes())
        .and_then(
            |http_client: reqwest::Client, bytes: bytes::Bytes| async move {
                let cmd = std::str::from_utf8(&bytes[..]).unwrap();
                let cursor = http_client
                    .post("http://localhost:3030/get_cursor")
                    .send()
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();
                let cursor = <[u8; 4]>::try_from(&cursor[..]).unwrap();
                let cursor = i32::from_ne_bytes(cursor);

                let is_ok = http_client
                    .post("http://localhost:3030/print")
                    .body(format!("{}\n{}:", cmd, cursor))
                    .send()
                    .await;
                match is_ok {
                    Ok(is_ok) => match is_ok.bytes().await {
                        Ok(ret) => Ok(warp::http::Response::builder().body(ret)),
                        Err(_) => Err(warp::reject::not_found()),
                    },
                    Err(_) => Err(warp::reject::not_found()),
                }
            },
        )
        .boxed();

    let cursor = get_i32();
    let get_cursor_post: warp::filters::BoxedFilter<(
        Result<warp::http::Response<bytes::Bytes>, warp::http::Error>,
    )> = warp::post()
        .and(warp::path("get_cursor"))
        .and(warp::path::end())
        .and(warp::any().map(move || cursor.clone()))
        .and_then(|cursor: Arc<Mutex<i32>>| async move {
            let begin = *cursor.lock().await;
            let bytes = begin.to_ne_bytes();
            let bytes = bytes::BytesMut::from(&bytes[..]);
            if true {
                Ok(warp::http::Response::builder().body(bytes::Bytes::from(bytes)))
            } else {
                Err(warp::reject::not_found())
            }
        })
        .boxed();

    warp::serve(cmd_post.or(get_cursor_post))
        .run(([127, 0, 0, 1], 3031))
        .await;
}
