use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, error::BlockingError, middleware, web::{self, Query}};
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

    let mut f: File;
    let response: String;
    let local: DateTime<Local> = Local::now();
    if dir.is_empty() {
        let path = format!(
            "{}/{}",
            local.format("%Y-%m-%d").to_string(),
            local.format("%H:%M:%S%.3f").to_string()
        );
        response = path.clone();
        f = create_path(path).await?;
    } else {
        let path = format!(
            "{}/{}/{}",
            dir,
            local.format("%Y-%m-%d").to_string(),
            local.format("%H:%M:%S%.3f").to_string()
        );
        response = path.clone();
        f = create_path(path).await?;
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

async fn create_path(path: String) -> Result<File, BlockingError<std::io::Error>> {
    let prefix = std::path::Path::new(&path).parent().unwrap();
    std::fs::create_dir_all(prefix).unwrap();
    web::block(|| std::fs::File::create(path)).await
}

fn index(req: HttpRequest) -> HttpResponse {
    let called_path: String = req
        .match_info()
        .get("path")
        .unwrap()
        .parse::<String>()
        .unwrap();
    let path_buf = PathBuf::new().join("./").join(called_path);
    if Path::new(&path_buf).is_dir() {
        let mut html = String::new();
        let paths = fs::read_dir(path_buf).unwrap();
        for path in paths {
            let path_buf = path.unwrap().path();
            let path_display: String = path_buf.display().to_string().chars().skip(1).collect();

            html.push_str(
                format!(
                    "<a href='{}'>{}</a><br/>",
                    path_display,
                    path_buf
                        .file_name()
                        .unwrap()
                        .to_owned()
                        .into_string()
                        .unwrap()
                )
                .as_str(),
            );
        }
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    } else {
        let html = match fs::read_to_string(&path_buf) {
            Ok(x) => x,
            Err(_) => path_buf.to_str().unwrap().to_string(),
        };
        HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body(html)
    }
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
