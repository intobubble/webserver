use axum::http::StatusCode;
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

#[derive(Debug, thiserror::Error)]
pub enum DownloadImageError {
    #[error("build url")]
    BuildUrl(#[source] url::ParseError),
    #[error("request")]
    Request(#[source] reqwest::Error),
    #[error("read body")]
    ReadBody(#[source] reqwest::Error),
}

impl From<DownloadImageError> for StatusCode {
    fn from(value: DownloadImageError) -> Self {
        match value {
            DownloadImageError::BuildUrl(_e) => StatusCode::BAD_REQUEST,
            DownloadImageError::ReadBody(_e) => StatusCode::INTERNAL_SERVER_ERROR,
            DownloadImageError::Request(_e) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SaveImageError {
    #[error("create file")]
    CreateFile(#[source] std::io::Error),
    #[error("write buffer")]
    Write(#[source] std::io::Error),
}

impl From<SaveImageError> for StatusCode {
    fn from(value: SaveImageError) -> Self {
        match value {
            SaveImageError::CreateFile(_e) => StatusCode::INTERNAL_SERVER_ERROR,
            SaveImageError::Write(_e) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub mod fetch {
    use super::{DownloadImageError, Image, SaveImageError};
    use axum::{extract::Query, http::StatusCode};
    use axum_macros::debug_handler;
    use std::fs::File;
    use tracing::{event, Level};

    #[debug_handler]
    pub async fn handle(Query(image): Query<Image>) -> axum::response::Result<(), StatusCode> {
        let bytes = send_get(&image).await.map_err(StatusCode::from)?;
        save_as_file(&image, &bytes)
            .await
            .map_err(StatusCode::from)?;
        event!(Level::INFO, "{}, {}, {}", &image.key, &image.x, &image.y);
        Ok(())
    }

    async fn send_get(image: &Image) -> Result<bytes::Bytes, DownloadImageError> {
        let url = image.to_url().map_err(DownloadImageError::BuildUrl)?;
        let response = reqwest::get(url)
            .await
            .map_err(DownloadImageError::Request)?;
        let bytes = response
            .bytes()
            .await
            .map_err(DownloadImageError::ReadBody)?;
        Ok(bytes)
    }

    async fn save_as_file(image: &Image, bytes: &bytes::Bytes) -> Result<(), SaveImageError> {
        let file_name = image.to_file_name();
        let mut out_file = File::create(file_name).map_err(SaveImageError::CreateFile)?;
        std::io::copy(&mut bytes.as_ref(), &mut out_file).map_err(SaveImageError::Write)?;
        Ok(())
    }
}
