//! Content-addressed blob storage on top of `object_store`.
//!
//! The same code path works for the local FS (default) and S3-compatible
//! object stores (MinIO, real S3, etc.). The local backend is the dev
//! default so a fresh `cargo run` works without docker.
//!
//! Path scheme: `blobs/AA/<sha256>` where `AA` is the first two hex chars
//! of the digest. Splitting prevents a single flat directory from growing
//! to millions of entries on FS backends.

use bytes::Bytes;
use object_store::ObjectStore;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjPath;
use std::path::Path;
use std::sync::Arc;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct Storage {
    inner: Arc<dyn ObjectStore>,
}

impl Storage {
    /// Build the storage backend from environment.
    ///
    /// - `VAULT_STORAGE=local` (default): blobs under `VAULT_LOCAL_ROOT`.
    /// - `VAULT_STORAGE=s3`: AWS_* + `S3_BUCKET` + `AWS_ENDPOINT`.
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let backend = std::env::var("VAULT_STORAGE").unwrap_or_else(|_| "local".to_string());
        match backend.as_str() {
            "s3" => {
                let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "vault".to_string());
                let endpoint = std::env::var("AWS_ENDPOINT").ok();
                let mut builder = AmazonS3Builder::new()
                    .with_bucket_name(&bucket)
                    .with_region(std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into()))
                    .with_access_key_id(std::env::var("AWS_ACCESS_KEY_ID")?)
                    .with_secret_access_key(std::env::var("AWS_SECRET_ACCESS_KEY")?)
                    .with_allow_http(true);
                if let Some(ep) = endpoint {
                    builder = builder.with_endpoint(ep);
                }
                let s3 = builder.build()?;
                Ok(Storage { inner: Arc::new(s3) })
            }
            _ => {
                let root =
                    std::env::var("VAULT_LOCAL_ROOT").unwrap_or_else(|_| "./data/blobs".into());
                std::fs::create_dir_all(&root)?;
                let local = LocalFileSystem::new_with_prefix(Path::new(&root))?;
                Ok(Storage { inner: Arc::new(local) })
            }
        }
    }

    /// `blobs/AA/<sha256>` — the canonical key for a hash.
    pub fn blob_path(sha256: &str) -> ObjPath {
        let prefix = &sha256[0..2];
        ObjPath::from(format!("blobs/{prefix}/{sha256}"))
    }

    pub async fn put(&self, sha256: &str, bytes: Bytes) -> AppResult<()> {
        let path = Self::blob_path(sha256);
        self.inner
            .put(&path, bytes.into())
            .await
            .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn get(&self, sha256: &str) -> AppResult<Bytes> {
        let path = Self::blob_path(sha256);
        let r = self.inner.get(&path).await.map_err(AppError::from)?;
        Ok(r.bytes().await.map_err(AppError::from)?)
    }

    pub async fn exists(&self, sha256: &str) -> AppResult<bool> {
        let path = Self::blob_path(sha256);
        match self.inner.head(&path).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(AppError::from(e)),
        }
    }
}
