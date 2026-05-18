use std::path::{Path, PathBuf};

use api::ApiErrorCode;
use base64::Engine;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StoredAttachment {
    pub storage_kind: String, // "local_fs" | "data_url" (legacy)
    pub storage_path: String, // relative file path for local_fs, or data: URL for legacy
    pub size_bytes: i64,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AttachmentValidationConfig {
    pub max_bytes: usize,
}

impl AttachmentValidationConfig {
    pub fn from_env() -> Self {
        let max_bytes = std::env::var("ATTACHMENTS_MAX_BYTES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5 * 1024 * 1024);
        Self { max_bytes }
    }
}

pub fn validate_content_type(content_type: &str) -> bool {
    // Keep dev flexible but block obviously risky MIME types for inline rendering.
    // This can be expanded or made configurable later.
    let ct = content_type.trim().to_lowercase();
    if ct.is_empty() {
        return false;
    }
    if ct.starts_with("image/") {
        return true;
    }
    matches!(
        ct.as_str(),
        "application/pdf" | "text/plain" | "application/octet-stream"
    )
}

pub fn parse_data_url(data_url: &str) -> Result<(String, Vec<u8>), axum::response::Response> {
    // Expected: data:<mime>;base64,<payload>
    let s = data_url.trim();
    if !s.starts_with("data:") {
        return Err(crate::util::api_error(ApiErrorCode::ValidationError));
    }
    let Some((meta, b64)) = s.split_once(',') else {
        return Err(crate::util::api_error(ApiErrorCode::ValidationError));
    };
    let meta = meta.strip_prefix("data:").unwrap_or(meta);
    let (mime, enc) = meta
        .split_once(';')
        .unwrap_or((meta, ""));
    if enc.to_lowercase() != "base64" {
        return Err(crate::util::api_error(ApiErrorCode::ValidationError));
    }

    let mime = mime.trim().to_string();
    if mime.is_empty() || !validate_content_type(&mime) {
        return Err(crate::util::api_error(ApiErrorCode::ValidationError));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| crate::util::api_error(ApiErrorCode::ValidationError))?;
    Ok((mime, bytes))
}

pub trait AttachmentStorage: Send + Sync {
    fn kind(&self) -> &'static str;

    fn put(
        &self,
        org_id: Uuid,
        attachment_id: Uuid,
        bytes: &[u8],
        ext: Option<&str>,
    ) -> Result<String, axum::response::Response>;

    fn get_path(&self, storage_path: &str) -> PathBuf;
}

#[derive(Debug, Clone)]
pub struct LocalFsAttachmentStorage {
    base_dir: PathBuf,
}

impl LocalFsAttachmentStorage {
    pub fn from_env() -> Self {
        let base_dir = std::env::var("ATTACHMENTS_DIR")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".local/attachments"));
        Self { base_dir }
    }

    fn ensure_base_dir(&self) -> Result<(), axum::response::Response> {
        std::fs::create_dir_all(&self.base_dir)
            .map_err(|_| crate::util::api_error(ApiErrorCode::InternalError))?;
        Ok(())
    }

    fn ext_sanitized(ext: Option<&str>) -> Option<String> {
        let ext = ext?.trim().trim_start_matches('.');
        if ext.is_empty() {
            return None;
        }
        let ok = ext
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !ok {
            return None;
        }
        Some(ext.to_lowercase())
    }

    pub fn guess_ext(filename: &str) -> Option<&str> {
        Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.trim().is_empty())
    }
}

impl AttachmentStorage for LocalFsAttachmentStorage {
    fn kind(&self) -> &'static str {
        "local_fs"
    }

    fn put(
        &self,
        org_id: Uuid,
        attachment_id: Uuid,
        bytes: &[u8],
        ext: Option<&str>,
    ) -> Result<String, axum::response::Response> {
        self.ensure_base_dir()?;

        let day = OffsetDateTime::now_utc().date();
        let org_prefix = format!("org-{}", org_id);
        let ymd = format!("{:04}-{:02}-{:02}", day.year(), day.month() as u8, day.day());
        let mut rel = format!("{org_prefix}/{ymd}/{}", attachment_id);
        if let Some(ext) = Self::ext_sanitized(ext) {
            rel.push('.');
            rel.push_str(&ext);
        }

        let full = self.base_dir.join(&rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| crate::util::api_error(ApiErrorCode::InternalError))?;
        }
        std::fs::write(&full, bytes).map_err(|_| crate::util::api_error(ApiErrorCode::InternalError))?;
        Ok(rel)
    }

    fn get_path(&self, storage_path: &str) -> PathBuf {
        self.base_dir.join(storage_path)
    }
}

pub fn storage_from_env() -> Box<dyn AttachmentStorage> {
    // Placeholder for future backends (S3, GCS, etc.). For now, local filesystem.
    Box::new(LocalFsAttachmentStorage::from_env())
}
