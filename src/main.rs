use axum::routing::{get, put};
use axum::Router;
use config::AppConfig;
use serde::Serialize;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::Span;
pub mod config;
pub mod handlers;

#[derive(Serialize, Debug)]
struct RequestLog {
    path: String,
    method: String,
}

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
    axum::serve(listener, app).await.unwrap();
}

fn app() -> Router {
    let app_conf = AppConfig::from_env();

    Router::new()
        .route("/image", get(handlers::image::fetch::handle))
        .route("/object", put(handlers::bucket::put_object::handle))
        .route("/object", get(handlers::bucket::get_object::handle))
        .route("/object/list", get(handlers::bucket::list_objects::handle))
        .layer(
            ServiceBuilder::new().layer(TraceLayer::new_for_http().on_request(
                |req: &axum::http::Request<_>, _span: &Span| {
                    let l = RequestLog {
                        path: req.uri().path().to_string(),
                        method: req.method().to_string(),
                    };
                    tracing::info!("{}", serde_json::to_string(&l).unwrap())
                },
            )),
        )
        .with_state(app_conf)
}
