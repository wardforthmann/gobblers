use std::{fs::{self, File}, io::Write, path::Path};

use actix_web::{
    middleware,
    web::{self, Query},
    App, Error, HttpResponse,HttpRequest, HttpServer,
};
use chrono::prelude::*;
use futures::StreamExt;
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    dir: Option<String>,
}

async fn save_file(
    mut payload: web::Payload,
    dir: Query<Info>,
    req: web::HttpRequest,
) -> Result<HttpResponse, Error> {
    let dir = match &dir.dir {
        Some(x) => x,
        None => "",
    };

    // let path : String;
    let mut f: File;
    let path: String;
    let response: String;
    if dir.is_empty() {
        let local: DateTime<Local> = Local::now();
        path = format!(
            "{}/{}",
            local.format("%Y-%m-%d").to_string(),
            local.format("%H:%M:%S%.3f").to_string()
        );
        response = path.clone();
        let prefix = std::path::Path::new(&path).parent().unwrap();
        std::fs::create_dir_all(prefix).unwrap();
        f = web::block(|| std::fs::File::create(path)).await.unwrap();
    } else {
        let local: DateTime<Local> = Local::now();
        path = format!(
            "{}/{}/{}",
            dir,
            local.format("%Y-%m-%d").to_string(),
            local.format("%H:%M:%S%.3f").to_string()
        );
        response = path.clone();
        let prefix = std::path::Path::new(&path).parent().unwrap();
        std::fs::create_dir_all(prefix).unwrap();
        f = web::block(|| std::fs::File::create(path)).await.unwrap();
    }

    let mut bytes = web::BytesMut::new();
    bytes.extend_from_slice(format!("POST {} \n", &response).as_bytes());
    for (key, value) in req.headers() {
        let value = value.to_str().unwrap();
        bytes.extend_from_slice(format!("{}: {}\n", key, value).as_bytes())
    }
    bytes.extend_from_slice(b"\n");
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item?);
    }
    web::block(move || f.write_all(&bytes).map(|_| f)).await?;

    Ok(HttpResponse::Ok().body(response))
}

fn index(req: HttpRequest) -> HttpResponse {
    let called_path : String = req.match_info().get("path").unwrap().parse::<String>().unwrap();
    let mut from = String::from("./");
    from.push_str(&called_path);
    let mut html = String::new();
    if Path::new(&from).is_dir() {
        let paths = fs::read_dir(from).unwrap();
        for path in paths {
            let path_buf = path.unwrap().path();
            let path_display = path_buf.display();
            html.push_str(format!("<a href='{}'>{}</a><br/>", path_display, path_buf.file_name().unwrap().to_owned().into_string().unwrap()).as_str());
        }
    } else {
        html = match fs::read_to_string(&from) {
            Ok(x) => x,
            Err(x) => from,
        };

        return HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(html);
    }


    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    // std::fs::create_dir_all("./tmp").unwrap();

    let ip = "0.0.0.0:8000";

    HttpServer::new(|| {
        App::new().wrap(middleware::Logger::default()).service(
            web::resource("/{path:.*}")
                .route(web::get().to(index))
                .route(web::post().to(save_file)),
        )
    })
    .bind(ip)?
    .run()
    .await
}
