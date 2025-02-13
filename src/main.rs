use core::fmt;
use std::{io, fs::File, str::FromStr};
use std::env;
use axum::{
    routing::get,
    Router,
    extract::Query,
    extract::Path,
    Json,
};
use axum_macros::debug_handler;
use reqwest::StatusCode;
use serde::{de, Deserialize, Deserializer, Serialize};
use log::{debug, info, error};
use url::Url;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenvy::dotenv().unwrap();

    let app = app();
    let host = env::var("HTTP_HOST").unwrap();
    let port = env::var("HTTP_PORT").unwrap();
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", &host, &port)).await.unwrap();
    debug!("web server is running on {}:{}", &host, &port);
    axum::serve(listener,app).await.unwrap();
}

fn app() -> Router {
    let app =  Router::new()
        .route("/", get(get_root))
        .route("/image/{x}/{y}", get(get_image));
    app
}

async fn get_root(Query(params): Query<Params>) -> Json<Params> {
    let j  = Json(params);
    info!("{:?}", j);
    j
}

struct Size {
    pub x: u16,
    pub y: u16
}

/// https://docs.rs/axum/latest/axum/handler/index.html
/// https://docs.rs/axum-macros/latest/axum_macros/attr.debug_handler.html
#[debug_handler]
async fn get_image(Path((x, y)): Path<(u16, u16)>) -> Result<(), StatusCode> {
    match store_image(&Size{x,y}).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("{}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn store_image(size: &Size) -> Result<(), Box<dyn std::error::Error>> {
    let raw = format!("https://picsum.photos/{}/{}", &size.x, &size.y);
    let url = Url::parse(&raw)?;
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let mut out = File::create(format!("image-{}-{}.png",  &size.x,  &size.y))?;
    io::copy(&mut bytes.as_ref(), &mut out)?;
    Ok(())
}

#[derive(Debug,  Deserialize, Serialize)]
#[allow(dead_code)]
struct Params {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    foo: Option<i32>,
    bar: Option<i32>
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}