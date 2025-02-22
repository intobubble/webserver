use std::{env, sync::LazyLock};
use tokio::sync::Mutex;

pub static AWS_DEFAULT_REGION: &str = "ap-northeast-1";

pub static APP_CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| {
    let http_host = env::var("HTTP_HOST").expect("HTTP_HOST is not set");
    let http_port = env::var("HTTP_PORT").expect("HTTP_PORT is not set");
    let aws_s3_bucket_name = env::var("AWS_S3_BUCKET_NAME").expect("AWS_S3_BUCKET_NAME is not set");
    let aws_iam_role_arn = env::var("AWS_IAM_ROLE_ARN").expect("AWS_IAM_ROLE_ARN is not set");

    let c = Config {
        http_host,
        http_port,
        aws_iam_role_arn,
        aws_s3_bucket_name,
    };

    Mutex::new(c)
});

pub struct Config {
    pub http_host: String,
    pub http_port: String,
    pub aws_s3_bucket_name: String,
    pub aws_iam_role_arn: String,
}
