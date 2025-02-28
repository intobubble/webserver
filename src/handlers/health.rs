pub mod get_health {
    use crate::handlers::error::ErrResp;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum_macros::debug_handler;

    #[debug_handler]
    pub async fn handle() -> Result<impl IntoResponse, ErrResp> {
        Ok((StatusCode::OK, ()))
    }
}
