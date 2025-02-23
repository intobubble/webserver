use serde::{Deserialize, Serialize};
use url::{ParseError, Url};

#[derive(Serialize, Deserialize)]
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
    use super::Image;
    use axum::extract::Query;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use axum_macros::debug_handler;
    use std::fs::File;
    use tracing::{event, Level};

    type ErrResp = (StatusCode, String);

    #[derive(Debug, thiserror::Error)]
    pub enum FetchImageError {
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

    impl From<FetchImageError> for ErrResp {
        fn from(value: FetchImageError) -> Self {
            match value {
                FetchImageError::BuildUrl(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::BAD_REQUEST, e.to_string())
                }
                FetchImageError::ReadBody(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
                FetchImageError::Request(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
                FetchImageError::CreateFile(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
                FetchImageError::Write(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            }
        }
    }

    #[debug_handler]
    pub async fn handle(
        Query(image): Query<Image>,
    ) -> axum::response::Result<impl IntoResponse, ErrResp> {
        let bytes = send_get(&image).await.map_err(ErrResp::from)?;
        save_as_file(&image, &bytes).await.map_err(ErrResp::from)?;
        Ok(())
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
