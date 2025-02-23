use std::env;

pub static AWS_DEFAULT_REGION: &str = "ap-northeast-1";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub http_host: String,
    pub http_port: String,
    pub aws_s3_bucket_name: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let http_host = env::var("HTTP_HOST").expect("HTTP_HOST is not set");
        let http_port = env::var("HTTP_PORT").expect("HTTP_PORT is not set");
        let aws_s3_bucket_name =
            env::var("AWS_S3_BUCKET_NAME").expect("AWS_S3_BUCKET_NAME is not set");

        AppConfig {
            http_host,
            http_port,
            aws_s3_bucket_name,
        }
    }
}
