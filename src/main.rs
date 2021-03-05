use actix_web::{client, dev::Body, web, App, HttpServer, Result};
use std::convert::TryFrom;
use std::process::exit;
use std::sync::Mutex;

struct State {
    cursor: Mutex<[u64; 2]>, // <- Mutex is necessary to mutate safely across threads
}
struct CorePort {
    port: String, // <- Mutex is necessary to mutate safely across threads
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut core_port = String::from("3030");
    let mut port = String::from("3031");
    for idx in 0..args.len() {
        if args[idx] == "--core_port" {
            core_port = args[idx + 1].clone();
        } else if args[idx] == "--port" {
            port = args[idx + 1].clone();
        }
    }

    let state = web::Data::new(State {
        cursor: Mutex::new([0, 0]),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .data(CorePort {
                port: core_port.clone(),
            })
            .route("/exit", web::post().to(exit_handler))
            .route("/get_cursor", web::post().to(get_cursor_handler))
            .route("/set_cursor", web::post().to(set_cursor_handler))
            .route("/move", web::post().to(move_handler))
            .route("/render", web::post().to(render_handler))
    })
    .bind(format!("127.0.0.1:{}", port))?
    .run()
    .await
}

async fn exit_handler() -> Result<web::Bytes> {
    async { exit(0) }.await;
    Ok(web::Bytes::new())
}

async fn get_cursor_handler(
    state: web::Data<State>,
    // bytes: web::Bytes,
) -> Result<web::Bytes> {
    let cursor = state.cursor.lock().unwrap();
    let x_bytes = cursor[0].to_ne_bytes().to_vec();
    let y_bytes = cursor[1].to_ne_bytes().to_vec();
    Ok(web::Bytes::from([x_bytes, y_bytes].concat()))
}

async fn set_cursor_handler(state: web::Data<State>, bytes: web::Bytes) -> Result<web::Bytes> {
    let new_cursor = bytes_to_cursor(bytes);
    let mut cursor = state.cursor.lock().unwrap();
    cursor[0] = new_cursor[0];
    cursor[1] = new_cursor[1];
    Ok(web::Bytes::new())
}

async fn move_handler(state: web::Data<State>, bytes: web::Bytes) -> Result<web::Bytes> {
    let mut cursor = state.cursor.lock().unwrap();
    let mut step = 0;
    for c in bytes {
        step = match c as char {
            'h' | 'j' | 'k' | 'l' => {
                if step == 0 {
                    1
                } else {
                    step
                }
            }
            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                step * 10 + (c - '0' as u8) as u64
            }
            _ => step,
        };
        match c as char {
            'h' => {
                cursor[0] -= u64::min(cursor[0], step);
                step = 0;
            }
            'l' => {
                cursor[0] += u64::min(u64::MAX - cursor[0], step);
                step = 0;
            }
            'j' => {
                cursor[1] += u64::min(u64::MAX - cursor[1], step);
                step = 0;
            }
            'k' => {
                cursor[1] -= u64::min(cursor[1], step);
                step = 0;
            }
            _ => {}
        };
    }
    Ok(web::Bytes::new())
}

async fn render_handler(core_port: web::Data<CorePort>, bytes: web::Bytes) -> Result<String> {
    let cmd = String::from(std::str::from_utf8(&bytes[..]).unwrap());

    let cursor = client::Client::new()
        .post(format!("http://localhost:{}/get_cursor", core_port.port))
        .send_body(Body::from_message(cmd.clone()))
        .await
        .unwrap()
        .body()
        .await
        .unwrap();
    let cursor = bytes_to_cursor(cursor);

    client::Client::new()
        .post(format!("http://localhost:{}/print", core_port.port))
        .send_body(Body::from_message(format!(
            "({}, {}):",
            cursor[0], cursor[1]
        )))
        .await
        .unwrap()
        .body()
        .await
        .unwrap();

    Ok(format!("Welcome {}!", "info.username"))
}

fn bytes_to_cursor(bytes: web::Bytes) -> [u64; 2] {
    let x = <[u8; 8]>::try_from(&bytes[0..8]).unwrap();
    let x = u64::from_ne_bytes(x);
    let y = <[u8; 8]>::try_from(&bytes[8..16]).unwrap();
    let y = u64::from_ne_bytes(y);
    [x, y]
}
