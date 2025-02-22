use axum::{
    routing::{get, put},
    Router,
};
use config::AppConfig;
use tracing::{event, Level};
pub mod config;
pub mod handlers;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let app_conf = AppConfig::from_env();
    let app = app();
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", &app_conf.http_host, &app_conf.http_port))
            .await
            .unwrap();
    event!(
        Level::INFO,
        "web server is running on {}:{}",
        &app_conf.http_host,
        &app_conf.http_port
    );
    axum::serve(listener, app).await.unwrap();
}

fn app() -> Router {
    let app_conf = AppConfig::from_env();
    Router::new()
        .route("/image", get(handlers::image::fetch::handle))
        .route("/object", put(handlers::bucket::put_object::handle))
        .route("/object", get(handlers::bucket::get_object::handle))
        .route("/object/list", get(handlers::bucket::list_objects::handle))
        .with_state(app_conf)
}
