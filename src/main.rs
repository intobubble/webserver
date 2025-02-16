use axum::{routing::get, Router};
use std::env;
use tracing::{event, Level};
pub mod handlers;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    let app = app();
    let host = env::var("HTTP_HOST").unwrap();
    let port = env::var("HTTP_PORT").unwrap();
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", &host, &port))
        .await
        .unwrap();
    event!(Level::INFO, "web server is running on {}:{}", &host, &port);
    axum::serve(listener, app).await.unwrap();
}

fn app() -> Router {
    let app = Router::new().route("/image/{x}/{y}", get(handlers::image::fetch::handle));
    app
}
