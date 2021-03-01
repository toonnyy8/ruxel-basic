use actix_web::{client, dev::Body, web, App, HttpServer, Result};
use std::convert::TryFrom;
use std::sync::Mutex;

struct AppStateWithBasicData {
    cursor: Mutex<i32>, // <- Mutex is necessary to mutate safely across threads
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let basic_data = web::Data::new(AppStateWithBasicData {
        cursor: Mutex::new(0),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(basic_data.clone())
            .route("/get_cursor", web::post().to(get_cursor_handler))
            .route("/cmd", web::post().to(cmd_handler))
    })
    .bind("127.0.0.1:3031")?
    .run()
    .await
}

async fn cmd_handler(bytes: web::Bytes) -> Result<String> {
    let cmd = String::from(std::str::from_utf8(&bytes[..]).unwrap());

    let cursor = client::Client::new()
        .post("http://localhost:3030/get_cursor")
        .send_body(Body::from_message(cmd.clone()))
        .await
        .unwrap()
        .body()
        .await
        .unwrap();
    let cursor = <[u8; 4]>::try_from(&cursor[..]).unwrap();
    let cursor = i32::from_ne_bytes(cursor);

    client::Client::new()
        .post("http://localhost:3030/print")
        .send_body(Body::from_message(format!("{}\n{}:", cmd, cursor)))
        .await
        .unwrap()
        .body()
        .await
        .unwrap();

    Ok(format!("Welcome {}!", "info.username"))
}

async fn get_cursor_handler(
    data: web::Data<AppStateWithBasicData>,
    // bytes: web::Bytes,
) -> Result<web::Bytes> {
    // let cmd = String::from(std::str::from_utf8(&bytes[..]).unwrap());
    // println!("{}", cmd);
    let mut cursor = data.cursor.lock().unwrap();
    *cursor += 1;
    let bytes = cursor.to_ne_bytes().to_vec();
    Ok(web::Bytes::from(bytes))
}
