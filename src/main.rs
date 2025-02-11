use core::fmt;
use std::str::FromStr;
use std::env;
use axum::{
    routing::get,
    Router,
    extract::Query
};
use serde::{de, Deserialize, Deserializer};
use log::info;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenvy::dotenv().unwrap();

    let app = app();
    let host = env::var("HTTP_HOST").unwrap();
    let port = env::var("HTTP_PORT").unwrap();
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", &host, &port)).await.unwrap();
    axum::serve(listener,app).await.unwrap();
}

fn app() -> Router {
    Router::new().route("/", get(get_root))
}

async fn get_root(Query(params): Query<Params>) -> String {
    let f = format!("{:?}", params);
    info!("{}", f);
    f
}

#[derive(Debug,  Deserialize)]
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