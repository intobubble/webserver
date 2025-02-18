use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ObjectSeed {
    key: String,
}

#[derive(Debug)]
pub struct S3Error(String);

impl S3Error {
    pub fn new(value: impl Into<String>) -> Self {
        S3Error(value.into())
    }

    pub fn add_message(self, message: impl Into<String>) -> Self {
        S3Error(format!("{}: {}", message.into(), self.0))
    }
}

impl std::error::Error for S3Error {}

impl std::fmt::Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

impl<T: aws_sdk_s3::error::ProvideErrorMetadata> From<T> for S3Error {
    fn from(value: T) -> Self {
        S3Error(format!(
            "{}: {}",
            value
                .code()
                .map(String::from)
                .unwrap_or("unknown code".into()),
            value
                .message()
                .map(String::from)
                .unwrap_or("missing reason".into())
        ))
    }
}

impl From<SaveImageError> for StatusCode {
    fn from(value: SaveImageError) -> Self {
        match value {
            SaveImageError::CreateFile(_e) => StatusCode::INTERNAL_SERVER_ERROR,
            SaveImageError::Write(_e) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PutObjectError {
    #[error("put object")]
    Put(#[source] S3Error),
}

impl From<PutObjectError> for StatusCode {
    fn from(value: PutObjectError) -> Self {
        match value {
            PutObjectError::Put(_e) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetObjectError {
    #[error("get object")]
    Get(#[source] S3Error),
    #[error("read content")]
    Read(#[source] S3Error),
    #[error("create file")]
    CreateFile(#[source] std::io::Error),
    #[error("write file")]
    WriteFile(#[source] std::io::Error),
}

impl From<GetObjectError> for StatusCode {
    fn from(value: GetObjectError) -> Self {
        match value {
            GetObjectError::Get(_e) => StatusCode::INTERNAL_SERVER_ERROR,
            GetObjectError::Read(_e) => StatusCode::INTERNAL_SERVER_ERROR,
            GetObjectError::CreateFile(_e) => StatusCode::INTERNAL_SERVER_ERROR,
            GetObjectError::WriteFile(_e) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub mod put_object {
    use std::env;

    use super::{ObjectSeed, PutObjectError, S3Error};
    use axum::{extract::Json, http::StatusCode};
    use axum_macros::debug_handler;
    use tracing::{event, Level};

    #[debug_handler]
    pub async fn handle(Json(obj): Json<ObjectSeed>) -> axum::response::Result<(), StatusCode> {
        put_object(&obj).await.map_err(StatusCode::from)?;
        event!(Level::INFO, "{}", &obj.key);
        Ok(())
    }

    async fn put_object(
        obj: &ObjectSeed,
    ) -> Result<aws_sdk_s3::operation::put_object::PutObjectOutput, PutObjectError> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);
        let body =
            aws_sdk_s3::primitives::ByteStream::from_path(std::path::Path::new(&obj.key)).await;

        let bucket = env::var("AWS_S3_BUCKET_NAME").unwrap();

        client
            .put_object()
            .bucket(bucket)
            .key("")
            .body(body.unwrap())
            .send()
            .await
            .map_err(S3Error::from)
            .map_err(PutObjectError::Put)
    }
}

pub mod get_object {
    use std::{env, io::Write};

    use super::{GetObjectError, ObjectSeed, S3Error};
    use axum::{extract::Json, http::StatusCode};
    use axum_macros::debug_handler;
    use tracing::{event, Level};

    #[debug_handler]
    pub async fn handle(Json(obj): Json<ObjectSeed>) -> axum::response::Result<(), StatusCode> {
        get_object(&obj).await.map_err(StatusCode::from)?;
        event!(Level::INFO, "{}", &obj.key);
        Ok(())
    }

    async fn get_object(obj: &ObjectSeed) -> Result<usize, GetObjectError> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);

        let bucket = env::var("AWS_S3_BUCKET_NAME").unwrap();

        let mut object = client
            .get_object()
            .bucket(bucket)
            .key(&obj.key)
            .send()
            .await
            .map_err(S3Error::from)
            .map_err(GetObjectError::Get)?;

        let dest = format!("{}-dest.png", &obj.key);
        let mut file = std::fs::File::create(&dest).map_err(GetObjectError::CreateFile)?;

        let mut byte_count = 0_usize;
        while let Some(bytes) = object
            .body
            .try_next()
            .await
            .map_err(|err| {
                S3Error::new(format!(
                    "Failed to write from S3 download to local file: {err:?}"
                ))
            })
            .map_err(GetObjectError::Read)?
        {
            let bytes_len = bytes.len();
            file.write_all(&bytes).map_err(GetObjectError::WriteFile)?;
            byte_count += bytes_len;
        }

        Ok(byte_count)
    }
}

pub mod list_bucket {}
