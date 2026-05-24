pub mod path;

use rust_embed::RustEmbed;
use std::borrow::Cow;
use thiserror::Error;

#[cfg(feature = "gzip")]
use std::collections::HashMap;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SpaError {
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("MIME type detection failed")]
    MimeDetection,
    #[error("Path error: {0}")]
    PathError(#[from] crate::core::path::PathError),
    #[error("Index file not found")]
    IndexFileNotFound,
}

/// SPA 配置
#[derive(Clone)]
#[non_exhaustive]
pub struct SpaConfig {
    pub base_path: String,
    pub index_files: Vec<String>,
}

impl Default for SpaConfig {
    fn default() -> Self {
        SpaConfig {
            base_path: "/".to_string(),
            index_files: vec!["index.html".to_string()],
        }
    }
}

impl SpaConfig {
    pub fn with_base_path(mut self, base_path: &str) -> Self {
        self.base_path = base_path.to_string();
        self
    }

    pub fn with_index_files(mut self, files: &[&str]) -> Self {
        self.index_files = files.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn add_index_file(mut self, file: &str) -> Self {
        self.index_files.push(file.to_string());
        self
    }
}

/// SPA 响应数据
pub struct SpaResponse {
    pub data: Cow<'static, [u8]>,
    pub mime: &'static str,
    pub etag: String,
    pub is_html: bool,
    #[cfg(feature = "gzip")]
    pub gzip_data: Option<Vec<u8>>,
}

/// 判断 MIME 类型是否值得压缩
#[cfg(feature = "gzip")]
fn is_compressible(mime: &str) -> bool {
    const COMPRESSIBLE: &[&str] = &[
        "text/",
        "application/javascript",
        "application/json",
        "application/xml",
        "application/wasm",
        "image/svg+xml",
    ];
    COMPRESSIBLE.iter().any(|prefix| mime.starts_with(prefix))
}

/// 格式化 ETag：取 SHA256 前 16 字节 hex 编码，加双引号
fn format_etag(hash: &[u8; 32]) -> String {
    let hex: String = hash[..16].iter().map(|b| format!("{:02x}", b)).collect();
    format!("\"{}\"", hex)
}

/// 检查 Accept-Encoding 是否包含 gzip
pub fn accepts_gzip(accept_encoding: &str) -> bool {
    accept_encoding.contains("gzip")
}

/// 检查 If-None-Match 是否匹配当前 ETag
pub fn etag_matches(if_none_match: &str, etag: &str) -> bool {
    if if_none_match.trim() == "*" {
        return true;
    }
    if_none_match
        .split(',')
        .any(|tag| tag.trim() == etag)
}

#[cfg(feature = "gzip")]
fn gzip_compress(data: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::with_capacity(data.len() / 2), Compression::fast());
    encoder.write_all(data).expect("gzip compression failed");
    encoder.finish().expect("gzip finalization failed")
}

/// SPA 处理器
pub struct SpaHandler<E: RustEmbed> {
    config: SpaConfig,
    #[cfg(feature = "gzip")]
    compression_cache: HashMap<String, Vec<u8>>,
    _marker: std::marker::PhantomData<E>,
}

impl<E: RustEmbed> SpaHandler<E> {
    pub fn new(config: SpaConfig) -> Self {
        #[cfg(feature = "gzip")]
        let compression_cache = {
            let mut cache = HashMap::new();
            for path in E::iter() {
                let path_str = path.as_ref();
                if let Some(file) = E::get(path_str) {
                    let mime = mime_guess::from_path(path_str)
                        .first_raw()
                        .unwrap_or("");
                    if is_compressible(mime) {
                        let compressed = gzip_compress(&file.data);
                        if compressed.len() < file.data.len() {
                            cache.insert(path_str.to_string(), compressed);
                        }
                    }
                }
            }
            cache
        };

        Self {
            config,
            #[cfg(feature = "gzip")]
            compression_cache,
            _marker: std::marker::PhantomData,
        }
    }

    /// 获取嵌入的文件（考虑基础路径）
    pub fn get_file(&self, request_path: &str) -> Result<SpaResponse, SpaError> {
        let clean_path = crate::core::path::collapse_slashes(request_path);
        let normalized_path = crate::core::path::normalize_path(&clean_path)?;
        let resource_path = crate::core::path::relative_to_base(&normalized_path, &self.config.base_path);

        if let Some(content) = E::get(&resource_path) {
            let mime = mime_guess::from_path(&resource_path)
                .first_raw()
                .ok_or(SpaError::MimeDetection)?;
            let is_html = mime.starts_with("text/html");
            let etag = format_etag(&content.metadata.sha256_hash());

            #[cfg(feature = "gzip")]
            let gzip_data = self.compression_cache.get(&resource_path).cloned();

            return Ok(SpaResponse {
                data: content.data,
                mime,
                etag,
                is_html,
                #[cfg(feature = "gzip")]
                gzip_data,
            });
        }

        // SPA fallback：尝试索引文件
        for index_file in &self.config.index_files {
            if let Some(content) = E::get(index_file) {
                let etag = format_etag(&content.metadata.sha256_hash());

                #[cfg(feature = "gzip")]
                let gzip_data = self.compression_cache.get(index_file).cloned();

                return Ok(SpaResponse {
                    data: content.data,
                    mime: "text/html",
                    etag,
                    is_html: true,
                    #[cfg(feature = "gzip")]
                    gzip_data,
                });
            }
        }

        Err(SpaError::IndexFileNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "gzip")]
    #[test]
    fn test_is_compressible() {
        assert!(is_compressible("text/html"));
        assert!(is_compressible("text/css"));
        assert!(is_compressible("application/javascript"));
        assert!(is_compressible("application/json"));
        assert!(is_compressible("image/svg+xml"));
        assert!(!is_compressible("image/png"));
        assert!(!is_compressible("image/jpeg"));
        assert!(!is_compressible("font/woff2"));
    }

    #[test]
    fn test_format_etag() {
        let hash = [0xab; 32];
        let etag = format_etag(&hash);
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert_eq!(etag.len(), 34); // 16 hex chars + 2 quotes
    }

    #[test]
    fn test_etag_matches() {
        let etag = "\"abc123\"";
        assert!(etag_matches("\"abc123\"", etag));
        assert!(etag_matches("\"abc123\", \"def456\"", etag));
        assert!(etag_matches("*", etag));
        assert!(!etag_matches("\"def456\"", etag));
    }

    #[test]
    fn test_accepts_gzip() {
        assert!(accepts_gzip("gzip"));
        assert!(accepts_gzip("gzip, deflate, br"));
        assert!(accepts_gzip("deflate, gzip"));
        assert!(!accepts_gzip("deflate, br"));
        assert!(!accepts_gzip(""));
    }
}
