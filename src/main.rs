use actix_web::{client, dev::Body, web, App, HttpServer, Result};
use std::convert::TryFrom;
use std::process::exit;
use std::sync::Mutex;

struct AppStateWithBasicData {
    cursor: Mutex<i32>, // <- Mutex is necessary to mutate safely across threads
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

    let basic_data = web::Data::new(AppStateWithBasicData {
        cursor: Mutex::new(0),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(basic_data.clone())
            .data(CorePort {
                port: core_port.clone(),
            })
            .route("/exit", web::post().to(exit_handler))
            .route("/get_cursor", web::post().to(get_cursor_handler))
            .route("/set_cursor", web::post().to(set_cursor_handler))
            .route("/cmd", web::post().to(cmd_handler))
    })
    .bind(format!("127.0.0.1:{}", port))?
    .run()
    .await
}

async fn cmd_handler(core_port: web::Data<CorePort>, bytes: web::Bytes) -> Result<String> {
    let cmd = String::from(std::str::from_utf8(&bytes[..]).unwrap());

    let cursor = client::Client::new()
        .post(format!("http://localhost:{}/get_cursor", core_port.port))
        .send_body(Body::from_message(cmd.clone()))
        .await
        .unwrap()
        .body()
        .await
        .unwrap();
    let cursor = <[u8; 4]>::try_from(&cursor[..]).unwrap();
    let cursor = i32::from_ne_bytes(cursor);

    client::Client::new()
        .post(format!("http://localhost:{}/print", core_port.port))
        .send_body(Body::from_message(format!("{}\n{}:", cmd, cursor)))
        .await
        .unwrap()
        .body()
        .await
        .unwrap();

    Ok(format!("Welcome {}!", "info.username"))
}

async fn exit_handler() -> Result<web::Bytes> {
    async { exit(0) }.await;
    Ok(web::Bytes::new())
}

async fn get_cursor_handler(
    data: web::Data<AppStateWithBasicData>,
    // bytes: web::Bytes,
) -> Result<web::Bytes> {
    let mut cursor = data.cursor.lock().unwrap();
    *cursor += 1;
    let bytes = cursor.to_ne_bytes().to_vec();
    Ok(web::Bytes::from(bytes))
}

async fn set_cursor_handler(
    data: web::Data<AppStateWithBasicData>,
    // bytes: web::Bytes,
) -> Result<web::Bytes> {
    let mut cursor = data.cursor.lock().unwrap();
    *cursor += 1;
    let bytes = cursor.to_ne_bytes().to_vec();
    Ok(web::Bytes::from(bytes))
}
