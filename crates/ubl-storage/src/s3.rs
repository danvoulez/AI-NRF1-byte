use anyhow::Result;
use aws_sdk_s3::{primitives::ByteStream, Client};

#[derive(Clone)]
pub struct S3Store {
    client: Client,
    bucket: String,
    public_base: String,
}

impl S3Store {
    pub async fn new(bucket: String, public_base: String) -> Result<Self> {
        let conf = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&conf);
        Ok(Self { client, bucket, public_base })
    }

    pub async fn from_env() -> Result<Self> {
        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "ubl-receipts".into());
        let public_base = std::env::var("S3_PUBLIC_BASE").unwrap_or_else(|_| "https://cdn.ubl.agency".into());
        Self::new(bucket, public_base).await
    }

    pub async fn put_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes))
            .send()
            .await?;
        Ok(())
    }

    pub async fn put_json(&self, key: &str, bytes: &[u8]) -> Result<String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes.to_vec()))
            .content_type("application/json")
            .send()
            .await?;
        Ok(format!("{}/{}", self.public_base.trim_end_matches('/'), key))
    }

    pub async fn put_zip(&self, key: &str, bytes: Vec<u8>) -> Result<String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes))
            .content_type("application/zip")
            .send()
            .await?;
        Ok(format!("{}/{}", self.public_base.trim_end_matches('/'), key))
    }
}
