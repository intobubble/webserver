use http::StatusCode;
use serde::{Deserialize, Serialize};

pub type ErrResp = (StatusCode, axum::Json<ErrRespBody>);

#[derive(Deserialize, Serialize)]
pub struct ErrRespBody {
    pub message: String,
}
