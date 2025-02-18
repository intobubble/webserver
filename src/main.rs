use axum::{
    routing::{get, put},
    Router,
};
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
    Router::new()
        .route("/image", get(handlers::image::fetch::handle))
        .route("/bucket", put(handlers::bucket::put_object::handle))
        .route("/bucket", get(handlers::bucket::get_object::handle))
}
