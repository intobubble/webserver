use serde::{Deserialize, Serialize};
use url::{ParseError, Url};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Image {
    pub x: u16,
    pub y: u16,
    pub key: String,
}

impl Image {
    pub fn to_url(&self) -> Result<Url, ParseError> {
        let raw = format!("https://picsum.photos/{}/{}", &self.x, &self.y);
        let url = Url::parse(&raw)?;
        Ok(url)
    }

    pub fn to_file_name(&self) -> String {
        let file_name = format!("{}.png", &self.key);
        file_name
    }
}

pub mod fetch {
    use crate::handlers::error::{ErrResp, ErrRespBody};

    use super::Image;
    use axum::extract::Query;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use axum_macros::debug_handler;
    use std::fs::File;
    use validator::Validate;

    #[derive(Debug, thiserror::Error)]
    pub enum FetchImageError {
        #[error("invalid input")]
        InvalidInput(#[source] validator::ValidationErrors),
        #[error("build url")]
        BuildUrl(#[source] url::ParseError),
        #[error("request")]
        Request(#[source] reqwest::Error),
        #[error("read body")]
        ReadBody(#[source] reqwest::Error),
        #[error("create file")]
        CreateFile(#[source] std::io::Error),
        #[error("write buffer")]
        Write(#[source] std::io::Error),
    }

    impl From<FetchImageError> for ErrRespBody {
        fn from(value: FetchImageError) -> Self {
            ErrRespBody {
                message: value.to_string(),
            }
        }
    }

    impl From<FetchImageError> for ErrResp {
        fn from(value: FetchImageError) -> Self {
            match value {
                FetchImageError::InvalidInput(_) | FetchImageError::BuildUrl(_) => {
                    let body = ErrRespBody::from(value);
                    (StatusCode::BAD_REQUEST, axum::Json(body))
                }
                FetchImageError::ReadBody(_)
                | FetchImageError::Request(_)
                | FetchImageError::CreateFile(_)
                | FetchImageError::Write(_) => {
                    let body = ErrRespBody::from(value);
                    (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(body))
                }
            }
        }
    }

    #[debug_handler]
    pub async fn handle(
        Query(image): Query<Image>,
    ) -> axum::response::Result<impl IntoResponse, ErrResp> {
        image
            .validate()
            .map_err(FetchImageError::InvalidInput)
            .map_err(ErrResp::from)?;
        let bytes = send_get(&image).await.map_err(ErrResp::from)?;
        save_as_file(&image, &bytes).await.map_err(ErrResp::from)?;
        Ok((StatusCode::OK, ()))
    }

    async fn send_get(image: &Image) -> Result<bytes::Bytes, FetchImageError> {
        let url = image.to_url().map_err(FetchImageError::BuildUrl)?;
        let response = reqwest::get(url).await.map_err(FetchImageError::Request)?;
        let bytes = response.bytes().await.map_err(FetchImageError::ReadBody)?;
        Ok(bytes)
    }

    async fn save_as_file(image: &Image, bytes: &bytes::Bytes) -> Result<(), FetchImageError> {
        let file_name = image.to_file_name();
        let mut out_file = File::create(file_name).map_err(FetchImageError::CreateFile)?;
        std::io::copy(&mut bytes.as_ref(), &mut out_file).map_err(FetchImageError::Write)?;
        Ok(())
    }
}
