use core::fmt;
use std::str::FromStr;
use std::env;
use axum::{
    routing::get,
    Router,
    extract::Query,
    Json,
};
use serde::{de, Deserialize, Deserializer, Serialize};
use log::{debug, info};

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
    Router::new().route("/", get(get_root))
}

async fn get_root(Query(params): Query<Params>) -> Json<Params> {
    let j  = Json(params);
    info!("{:?}", j);
    j
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