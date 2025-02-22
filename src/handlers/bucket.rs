use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

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
            DownloadImageError::BuildUrl(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::BAD_REQUEST
            }
            DownloadImageError::ReadBody(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            DownloadImageError::Request(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
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
            SaveImageError::CreateFile(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            SaveImageError::Write(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
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
            PutObjectError::Put(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
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
            GetObjectError::Get(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            GetObjectError::Read(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            GetObjectError::CreateFile(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            GetObjectError::WriteFile(e) => {
                event!(Level::ERROR, "{}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ListObjectsError {
    #[error("list objects in a bucket")]
    List(#[source] S3Error),
}

impl From<ListObjectsError> for StatusCode {
    fn from(value: ListObjectsError) -> Self {
        match value {
            ListObjectsError::List(_e) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub mod put_object {
    use super::aws::build_shared_config;
    use super::{ObjectSeed, PutObjectError, S3Error};
    use crate::config::APP_CONFIG;
    use axum::{extract::Json, http::StatusCode, response::IntoResponse};
    use axum_macros::debug_handler;
    use tracing::{event, Level};

    #[debug_handler]
    pub async fn handle(Json(obj): Json<ObjectSeed>) -> Result<impl IntoResponse, StatusCode> {
        put_object(&obj).await.map_err(StatusCode::from)?;
        event!(Level::INFO, "{}", &obj.key);
        Ok(())
    }

    async fn put_object(
        obj: &ObjectSeed,
    ) -> Result<aws_sdk_s3::operation::put_object::PutObjectOutput, PutObjectError> {
        let config = build_shared_config().await.load().await;
        let client = aws_sdk_s3::Client::new(&config);
        let source = format!("{}.png", &obj.key);
        let source = std::path::Path::new(&source);
        let body =
            aws_sdk_s3::primitives::ByteStream::from_path(std::path::Path::new(source)).await;

        let app_config = APP_CONFIG.lock().await;

        client
            .put_object()
            .bucket(&app_config.aws_s3_bucket_name)
            .key(&obj.key)
            .body(body.unwrap())
            .send()
            .await
            .map_err(S3Error::from)
            .map_err(PutObjectError::Put)
    }
}

pub mod get_object {
    use super::aws::build_shared_config;
    use super::{GetObjectError, ObjectSeed, S3Error};
    use crate::config::APP_CONFIG;
    use axum::{extract::Query, http::StatusCode, response::IntoResponse};
    use axum_macros::debug_handler;
    use std::{collections::HashMap, io::Write};
    use tracing::{event, Level};

    #[debug_handler]
    pub async fn handle(
        Query(params): Query<HashMap<String, String>>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let key = params.get("key").unwrap();
        let obj = ObjectSeed {
            key: key.to_owned(),
        };
        get_object(&obj).await.map_err(StatusCode::from)?;
        event!(Level::INFO, "{}", &obj.key);
        Ok(())
    }

    async fn get_object(obj: &ObjectSeed) -> Result<usize, GetObjectError> {
        let config = build_shared_config().await.load().await;
        let client = aws_sdk_s3::Client::new(&config);

        let app_config = APP_CONFIG.lock().await;

        let mut object = client
            .get_object()
            .bucket(&app_config.aws_s3_bucket_name)
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

pub mod list_objects {
    use super::aws::build_shared_config;
    use super::{ListObjectsError, S3Error};
    use crate::config::APP_CONFIG;
    use axum::{http::StatusCode, response::IntoResponse, Json};
    use axum_macros::debug_handler;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct ObjectKeys {
        keys: Vec<String>,
    }

    #[debug_handler]
    pub async fn handle() -> Result<impl IntoResponse, StatusCode> {
        let result = list().await.map_err(StatusCode::from)?;
        let obj_keys = ObjectKeys { keys: result };
        Ok((StatusCode::OK, Json(obj_keys)))
    }

    async fn list() -> Result<Vec<String>, ListObjectsError> {
        let config = build_shared_config().await.load().await;
        let client = aws_sdk_s3::Client::new(&config);

        let app_config = APP_CONFIG.lock().await;

        let resp = client
            .list_objects_v2()
            .bucket(&app_config.aws_s3_bucket_name)
            .send()
            .await
            .map_err(S3Error::from)
            .map_err(ListObjectsError::List)?;

        let objs = resp
            .contents()
            .iter()
            .map(|c| c.key().unwrap_or_default().to_owned())
            .collect::<Vec<String>>();
        Ok(objs)
    }
}

mod aws {
    use crate::config::{APP_CONFIG, AWS_DEFAULT_REGION};
    use aws_config::sts::AssumeRoleProvider;
    use aws_config::BehaviorVersion;
    use aws_config::ConfigLoader;
    use aws_types::region::Region;

    pub async fn build_shared_config() -> ConfigLoader {
        let app_config = APP_CONFIG.lock().await;

        let region = Region::from_static(AWS_DEFAULT_REGION);
        let shared_config = aws_config::defaults(BehaviorVersion::latest()).region(region);
        let provider = AssumeRoleProvider::builder(&app_config.aws_iam_role_arn)
            .session_name("webserver-session")
            .build()
            .await;

        shared_config.credentials_provider(provider)
    }
}
