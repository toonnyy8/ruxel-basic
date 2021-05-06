use actix_web::{client, dev::Body, web, App, HttpServer, Result};
use clap;
use regex;
use std::convert::TryFrom;
use std::process::exit;
use std::sync::Mutex;

struct State {
    cursor: Mutex<[u64; 2]>, // <- Mutex is necessary to mutate safely across threads
}

#[derive(Debug, Clone)]
struct CorePort {
    port: String, // <- Mutex is necessary to mutate safely across threads
}
#[derive(Debug, Clone)]
struct KeyMap {
    up: u8,
    down: u8,
    left: u8,
    right: u8,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let matches = clap::App::new("ruxel basic")
        .version("0.0")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Does awesome things")
        .arg(
            clap::Arg::new("core_port")
                .long("core_port")
                .value_name("port num")
                .about("Sets the core port")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("port")
                .long("port")
                .value_name("port num")
                .about("Sets the port")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("up")
                .long("up")
                .value_name("key name")
                .about("Sets key of up")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("down")
                .long("down")
                .value_name("key name")
                .about("Sets key of down")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("left")
                .long("left")
                .value_name("key name")
                .about("Sets key of left")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("right")
                .long("right")
                .value_name("key name")
                .about("Sets key of right")
                .takes_value(true),
        )
        .get_matches();

    let core_port = if let Some(port) = matches.value_of("core_port") {
        CorePort {
            port: port.to_string(),
        }
    } else {
        CorePort {
            port: "3030".to_string(),
        }
    };
    let port_num = if let Some(port) = matches.value_of("port") {
        port
    } else {
        "3031"
    }
    .to_string();

    let key_map = {
        let up = if let Some(key) = matches.value_of("up") {
            key.chars().collect::<Vec<_>>()[0]
        } else {
            'k'
        } as u8;
        let down = if let Some(key) = matches.value_of("down") {
            key.chars().collect::<Vec<_>>()[0]
        } else {
            'j'
        } as u8;
        let left = if let Some(key) = matches.value_of("left") {
            key.chars().collect::<Vec<_>>()[0]
        } else {
            'h'
        } as u8;
        let right = if let Some(key) = matches.value_of("right") {
            key.chars().collect::<Vec<_>>()[0]
        } else {
            'l'
        } as u8;

        KeyMap {
            up,
            down,
            left,
            right,
        }
    };

    let state = web::Data::new(State {
        cursor: Mutex::new([0, 0]),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .data(core_port.clone())
            .data(key_map.clone())
            .route("/exit", web::post().to(exit_handler))
            .route("/get_cursor", web::post().to(get_cursor_handler))
            .route("/set_cursor", web::post().to(set_cursor_handler))
            .route("/move", web::post().to(move_handler))
            .route("/render", web::post().to(render_handler))
    })
    .bind(format!("127.0.0.1:{}", port_num))?
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

async fn move_handler(
    key_map: web::Data<KeyMap>,
    state: web::Data<State>,
    bytes: web::Bytes,
) -> Result<web::Bytes> {
    let cmd = std::str::from_utf8(&bytes[..]).unwrap();
    let reg = regex::Regex::new("^(([1-9][0-9]*)?[hjkl])+$").unwrap();
    if reg.is_match(cmd) {
        let mut cursor = state.cursor.lock().unwrap();
        let mut step = 0;
        for c in bytes {
            step = match c as char {
                '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                    step * 10 + (c - '0' as u8) as u64
                }
                _ => {
                    if (c == key_map.left
                        || c == key_map.right
                        || c == key_map.up
                        || c == key_map.down)
                        && step == 0
                    {
                        1
                    } else {
                        step
                    }
                }
            };
            if c == key_map.left {
                cursor[0] -= u64::min(cursor[0], step);
                step = 0;
            } else if c == key_map.right {
                cursor[0] += u64::min(u64::MAX - cursor[0], step);
                step = 0;
            } else if c == key_map.up {
                cursor[1] -= u64::min(cursor[1], step);
                step = 0;
            } else if c == key_map.down {
                cursor[1] += u64::min(u64::MAX - cursor[1], step);
                step = 0;
            }
        }
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

    let mode = client::Client::new()
        .post(format!("http://localhost:{}/get_mode", core_port.port))
        .send_body(Body::from_message(cmd.clone()))
        .await
        .unwrap()
        .body()
        .await
        .unwrap();
    let mode = String::from(std::str::from_utf8(&mode[..]).unwrap());

    client::Client::new()
        .post(format!("http://localhost:{}/print", core_port.port))
        .send_body(Body::from_message(format!(
            "{}|({}, {}):",
            mode, cursor[0], cursor[1]
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
