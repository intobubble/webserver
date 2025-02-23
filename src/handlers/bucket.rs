use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ObjectSeed {
    key: String,
}

#[derive(Debug, Clone)]
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

pub mod put_object {
    use crate::config::{AppConfig, AWS_DEFAULT_REGION};

    use super::{ObjectSeed, S3Error};
    use aws_config::{BehaviorVersion, Region};
    use axum::extract::{Json, State};
    use axum::http::{Response, StatusCode};
    use axum::response::IntoResponse;
    use axum_macros::debug_handler;
    use tracing::{event, Level};

    type ErrResp = (StatusCode, String);

    #[derive(Debug, thiserror::Error)]
    pub enum PutObjectError {
        #[error("put object")]
        Put(#[source] S3Error),
    }

    impl From<PutObjectError> for ErrResp {
        fn from(value: PutObjectError) -> Self {
            match value {
                PutObjectError::Put(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            }
        }
    }

    #[debug_handler]
    pub async fn handle(
        State(app_config): State<AppConfig>,
        Json(obj): Json<ObjectSeed>,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        put_object(&app_config, &obj).await.map_err(ErrResp::from)?;
        let r = Response::builder().body("success".to_string()).unwrap();
        Ok((StatusCode::OK, r))
    }

    async fn put_object(app_config: &AppConfig, obj: &ObjectSeed) -> Result<(), PutObjectError> {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::from_static(AWS_DEFAULT_REGION))
            .load()
            .await;
        let client = aws_sdk_s3::Client::new(&sdk_config);
        let source = format!("{}.png", &obj.key);
        let source = std::path::Path::new(&source);
        let body =
            aws_sdk_s3::primitives::ByteStream::from_path(std::path::Path::new(source)).await;

        client
            .put_object()
            .bucket(&app_config.aws_s3_bucket_name)
            .key(&obj.key)
            .body(body.unwrap())
            .send()
            .await
            .map_err(S3Error::from)
            .map_err(PutObjectError::Put)?;

        Ok(())
    }
}

pub mod get_object {
    use crate::config::{AppConfig, AWS_DEFAULT_REGION};

    use super::{ObjectSeed, S3Error};
    use aws_config::{BehaviorVersion, Region};
    use axum::extract::{Query, State};
    use axum::{http::StatusCode, response::IntoResponse};
    use axum_macros::debug_handler;
    use std::{collections::HashMap, io::Write};
    use tracing::{event, Level};

    type ErrResp = (StatusCode, String);

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

    impl From<GetObjectError> for ErrResp {
        fn from(value: GetObjectError) -> Self {
            match value {
                GetObjectError::Get(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
                GetObjectError::Read(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
                GetObjectError::CreateFile(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
                GetObjectError::WriteFile(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            }
        }
    }

    #[debug_handler]
    pub async fn handle(
        State(app_config): State<AppConfig>,
        Query(params): Query<HashMap<String, String>>,
    ) -> Result<impl IntoResponse, ErrResp> {
        let key = params.get("key").unwrap();
        let obj = ObjectSeed {
            key: key.to_owned(),
        };
        get_object(&app_config, &obj).await.map_err(ErrResp::from)?;
        Ok(())
    }

    async fn get_object(app_config: &AppConfig, obj: &ObjectSeed) -> Result<usize, GetObjectError> {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::from_static(AWS_DEFAULT_REGION))
            .load()
            .await;
        let client = aws_sdk_s3::Client::new(&sdk_config);
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
    use crate::config::{AppConfig, AWS_DEFAULT_REGION};

    use super::S3Error;
    use aws_config::{BehaviorVersion, Region};
    use axum::extract::State;
    use axum::{http::StatusCode, response::IntoResponse, Json};
    use axum_macros::debug_handler;
    use serde::{Deserialize, Serialize};
    use tracing::{event, Level};

    type ErrResp = (StatusCode, String);

    #[derive(Debug, thiserror::Error)]
    pub enum ListObjectsError {
        #[error("list objects in a bucket")]
        List(#[source] S3Error),
    }

    impl From<ListObjectsError> for ErrResp {
        fn from(value: ListObjectsError) -> Self {
            match value {
                ListObjectsError::List(e) => {
                    event!(Level::ERROR, "{}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct ObjectKeys {
        keys: Vec<String>,
    }

    #[debug_handler]
    pub async fn handle(State(app_config): State<AppConfig>) -> Result<impl IntoResponse, ErrResp> {
        let result = list(&app_config).await.map_err(ErrResp::from)?;
        let obj_keys = ObjectKeys { keys: result };
        Ok((StatusCode::OK, Json(obj_keys)))
    }

    async fn list(app_config: &AppConfig) -> Result<Vec<String>, ListObjectsError> {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::from_static(AWS_DEFAULT_REGION))
            .load()
            .await;

        let client = aws_sdk_s3::Client::new(&sdk_config);
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
