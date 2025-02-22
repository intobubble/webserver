use axum::{
    routing::{get, put},
    Router,
};
use config::APP_CONFIG;
use tracing::{event, Level};
pub mod config;
pub mod handlers;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();
    let conf = APP_CONFIG.lock().await;
    let app = app();
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", &conf.http_host, &conf.http_port))
            .await
            .unwrap();
    event!(
        Level::INFO,
        "web server is running on {}:{}",
        &conf.http_host,
        &conf.http_port
    );
    axum::serve(listener, app).await.unwrap();
}

fn app() -> Router {
    Router::new()
        .route("/image", get(handlers::image::fetch::handle))
        .route("/object", put(handlers::bucket::put_object::handle))
        .route("/object", get(handlers::bucket::get_object::handle))
        .route("/object/list", get(handlers::bucket::list_objects::handle))
}
