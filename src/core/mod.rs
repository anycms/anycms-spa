pub mod path;

use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::collections::HashMap;
use thiserror::Error;

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
    pub security_headers: Vec<(String, String)>,
    pub error_pages: HashMap<u16, String>,
}

impl Default for SpaConfig {
    fn default() -> Self {
        SpaConfig {
            base_path: "/".to_string(),
            index_files: vec!["index.html".to_string()],
            security_headers: Vec::new(),
            error_pages: HashMap::new(),
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

    pub fn with_error_page(mut self, status_code: u16, file_path: &str) -> Self {
        self.error_pages.insert(status_code, file_path.to_string());
        self
    }

    pub fn with_security_header(mut self, key: &str, value: &str) -> Self {
        self.security_headers.push((key.to_string(), value.to_string()));
        self
    }

    pub fn with_default_security_headers(self) -> Self {
        self.with_security_header("X-Content-Type-Options", "nosniff")
            .with_security_header("X-Frame-Options", "SAMEORIGIN")
            .with_security_header("X-XSS-Protection", "1; mode=block")
            .with_security_header("Referrer-Policy", "strict-origin-when-cross-origin")
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
    #[cfg(feature = "brotli")]
    pub brotli_data: Option<Vec<u8>>,
}

impl SpaResponse {
    /// 是否有压缩变体可用
    #[cfg(any(feature = "gzip", feature = "brotli"))]
    pub fn has_compression(&self) -> bool {
        #[cfg(feature = "gzip")]
        if self.gzip_data.is_some() {
            return true;
        }
        #[cfg(feature = "brotli")]
        if self.brotli_data.is_some() {
            return true;
        }
        false
    }

    /// 根据 Accept-Encoding 选择最优编码，返回 (Content-Encoding 值, 压缩数据)
    /// 优先级：br > gzip
    #[cfg(any(feature = "gzip", feature = "brotli"))]
    pub fn select_encoding(&self, accept_encoding: &str) -> Option<(&'static str, &[u8])> {
        #[cfg(feature = "brotli")]
        {
            if accepts_brotli(accept_encoding) {
                if let Some(ref data) = self.brotli_data {
                    return Some(("br", data.as_slice()));
                }
            }
        }
        #[cfg(feature = "gzip")]
        {
            if accepts_gzip(accept_encoding) {
                if let Some(ref data) = self.gzip_data {
                    return Some(("gzip", data.as_slice()));
                }
            }
        }
        None
    }
}

/// 判断 MIME 类型是否值得压缩
#[cfg(any(feature = "gzip", feature = "brotli"))]
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

/// 对文本类 MIME 类型追加 `; charset=utf-8`
pub fn content_type_with_charset(mime: &str) -> Cow<'static, str> {
    const CHARSET_TYPES: &[&str] = &[
        "text/",
        "application/javascript",
        "application/json",
        "application/xml",
    ];
    if CHARSET_TYPES.iter().any(|prefix| mime.starts_with(prefix)) {
        format!("{}; charset=utf-8", mime).into()
    } else {
        mime.to_string().into()
    }
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

/// 检查 Accept-Encoding 是否包含 br（Brotli）
pub fn accepts_brotli(accept_encoding: &str) -> bool {
    accept_encoding.contains("br")
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

/// 解析后的 Range 请求
#[derive(Debug, Clone, PartialEq)]
pub struct RangeSpec {
    pub start: usize,
    pub end: usize,
}

impl RangeSpec {
    /// 解析 `Range: bytes=0-1023` 头，返回 RangeSpec 或 None
    pub fn parse(range_header: &str, total_len: usize) -> Option<Self> {
        let range_header = range_header.trim();
        let suffix = range_header.strip_prefix("bytes=")?;
        let suffix = suffix.trim();

        // 暂只支持单段 Range
        if suffix.contains(',') {
            return None;
        }

        let parts: Vec<&str> = suffix.splitn(2, '-').collect();
        if parts.len() != 2 {
            return None;
        }

        let spec = match (parts[0].trim(), parts[1].trim()) {
            // bytes=start-end
            (start_s, end_s) if !start_s.is_empty() && !end_s.is_empty() => {
                let start: usize = start_s.parse().ok()?;
                let end: usize = end_s.parse().ok()?;
                if start > end || start >= total_len {
                    return None;
                }
                RangeSpec {
                    start,
                    end: end.min(total_len - 1),
                }
            }
            // bytes=start- (from start to end)
            (start_s, "") if !start_s.is_empty() => {
                let start: usize = start_s.parse().ok()?;
                if start >= total_len {
                    return None;
                }
                RangeSpec {
                    start,
                    end: total_len - 1,
                }
            }
            // bytes=-suffix (last N bytes)
            ("", suffix_s) if !suffix_s.is_empty() => {
                let suffix_len: usize = suffix_s.parse().ok()?;
                if suffix_len == 0 {
                    return None;
                }
                let start = total_len.saturating_sub(suffix_len);
                RangeSpec {
                    start,
                    end: total_len - 1,
                }
            }
            _ => return None,
        };

        Some(spec)
    }

    /// 格式化 Content-Range 头值
    pub fn content_range(&self, total_len: usize) -> String {
        format!("bytes {}-{}/{}", self.start, self.end, total_len)
    }

    /// 获取切片范围长度
    pub fn len(&self) -> usize {
        self.end - self.start + 1
    }
}

/// 检查 If-Range 是否匹配（支持 ETag 或 HTTP-date，这里只处理 ETag）
pub fn if_range_matches(if_range: &str, etag: &str) -> bool {
    let if_range = if_range.trim();
    // ETag 形式：以双引号开头
    if if_range.starts_with('"') {
        if_range == etag
    } else {
        // HTTP-date 形式：暂不支持，跳过 If-Range 检查
        false
    }
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

#[cfg(feature = "brotli")]
fn brotli_compress(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut compressor = brotli::CompressorWriter::new(
        Vec::with_capacity(data.len() / 2),
        4096,
        4,
        22,
    );
    compressor.write_all(data).expect("brotli compression failed");
    compressor.into_inner()
}

/// SPA 处理器
pub struct SpaHandler<E: RustEmbed> {
    config: SpaConfig,
    #[cfg(feature = "gzip")]
    compression_cache: HashMap<String, Vec<u8>>,
    #[cfg(feature = "brotli")]
    brotli_cache: HashMap<String, Vec<u8>>,
    _marker: std::marker::PhantomData<E>,
}

impl<E: RustEmbed> SpaHandler<E> {
    pub fn new(config: SpaConfig) -> Self {
        #[cfg(feature = "gzip")]
        let mut gzip_cache = HashMap::new();
        #[cfg(feature = "brotli")]
        let mut br_cache = HashMap::new();

        #[cfg(any(feature = "gzip", feature = "brotli"))]
        for path in E::iter() {
            let path_str = path.as_ref();
            if let Some(file) = E::get(path_str) {
                let mime = mime_guess::from_path(path_str)
                    .first_raw()
                    .unwrap_or("");
                if is_compressible(mime) {
                    #[cfg(feature = "gzip")]
                    {
                        let compressed = gzip_compress(&file.data);
                        if compressed.len() < file.data.len() {
                            gzip_cache.insert(path_str.to_string(), compressed);
                        }
                    }
                    #[cfg(feature = "brotli")]
                    {
                        let compressed = brotli_compress(&file.data);
                        if compressed.len() < file.data.len() {
                            br_cache.insert(path_str.to_string(), compressed);
                        }
                    }
                }
            }
        }

        Self {
            config,
            #[cfg(feature = "gzip")]
            compression_cache: gzip_cache,
            #[cfg(feature = "brotli")]
            brotli_cache: br_cache,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn security_headers(&self) -> &[(String, String)] {
        &self.config.security_headers
    }

    /// 获取嵌入的文件（考虑基础路径）
    pub fn get_file(&self, request_path: &str) -> Result<SpaResponse, SpaError> {
        let clean_path = crate::core::path::collapse_slashes(request_path);
        let normalized_path = crate::core::path::normalize_path(&clean_path)?;
        let resource_path = crate::core::path::relative_to_base(&normalized_path, &self.config.base_path);

        if let Some(content) = E::get(&resource_path) {
            let mime = mime_guess::from_path(&resource_path)
                .first_raw()
                .unwrap_or("application/octet-stream");
            let is_html = mime.starts_with("text/html");
            let etag = format_etag(&content.metadata.sha256_hash());

            #[cfg(feature = "gzip")]
            let gzip_data = self.compression_cache.get(&resource_path).cloned();
            #[cfg(feature = "brotli")]
            let brotli_data = self.brotli_cache.get(&resource_path).cloned();

            return Ok(SpaResponse {
                data: content.data,
                mime,
                etag,
                is_html,
                #[cfg(feature = "gzip")]
                gzip_data,
                #[cfg(feature = "brotli")]
                brotli_data,
            });
        }

        // SPA fallback：尝试索引文件
        for index_file in &self.config.index_files {
            if let Some(content) = E::get(index_file) {
                let etag = format_etag(&content.metadata.sha256_hash());

                #[cfg(feature = "gzip")]
                let gzip_data = self.compression_cache.get(index_file).cloned();
                #[cfg(feature = "brotli")]
                let brotli_data = self.brotli_cache.get(index_file).cloned();

                return Ok(SpaResponse {
                    data: content.data,
                    mime: "text/html",
                    etag,
                    is_html: true,
                    #[cfg(feature = "gzip")]
                    gzip_data,
                    #[cfg(feature = "brotli")]
                    brotli_data,
                });
            }
        }

        Err(SpaError::IndexFileNotFound)
    }

    /// 获取自定义错误页面
    pub fn get_error_page(&self, status: u16) -> Option<SpaResponse> {
        let file_path = self.config.error_pages.get(&status)?;
        let content = E::get(file_path)?;
        let mime = mime_guess::from_path(file_path.as_str())
            .first_raw()
            .unwrap_or("text/html");
        let etag = format_etag(&content.metadata.sha256_hash());

        #[cfg(feature = "gzip")]
        let gzip_data = self.compression_cache.get(file_path).cloned();
        #[cfg(feature = "brotli")]
        let brotli_data = self.brotli_cache.get(file_path).cloned();

        Some(SpaResponse {
            data: content.data,
            mime,
            etag,
            is_html: mime.starts_with("text/html"),
            #[cfg(feature = "gzip")]
            gzip_data,
            #[cfg(feature = "brotli")]
            brotli_data,
        })
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

    #[test]
    fn test_content_type_with_charset() {
        assert_eq!(content_type_with_charset("text/html"), "text/html; charset=utf-8");
        assert_eq!(content_type_with_charset("text/css"), "text/css; charset=utf-8");
        assert_eq!(content_type_with_charset("application/javascript"), "application/javascript; charset=utf-8");
        assert_eq!(content_type_with_charset("application/json"), "application/json; charset=utf-8");
        assert_eq!(content_type_with_charset("image/png"), "image/png");
        assert_eq!(content_type_with_charset("application/wasm"), "application/wasm");
    }

    #[test]
    fn test_accepts_brotli() {
        assert!(accepts_brotli("br"));
        assert!(accepts_brotli("gzip, deflate, br"));
        assert!(accepts_brotli("br, gzip"));
        assert!(!accepts_brotli("gzip, deflate"));
        assert!(!accepts_brotli(""));
    }

    #[cfg(feature = "brotli")]
    #[test]
    fn test_brotli_compress() {
        let data = b"hello world hello world hello world hello world";
        let compressed = brotli_compress(data);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_range_parse_start_end() {
        // bytes=0-4
        let spec = RangeSpec::parse("bytes=0-4", 10).unwrap();
        assert_eq!(spec, RangeSpec { start: 0, end: 4 });
        assert_eq!(spec.len(), 5);
    }

    #[test]
    fn test_range_parse_start_open() {
        // bytes=5-
        let spec = RangeSpec::parse("bytes=5-", 10).unwrap();
        assert_eq!(spec, RangeSpec { start: 5, end: 9 });
        assert_eq!(spec.len(), 5);
    }

    #[test]
    fn test_range_parse_suffix() {
        // bytes=-3
        let spec = RangeSpec::parse("bytes=-3", 10).unwrap();
        assert_eq!(spec, RangeSpec { start: 7, end: 9 });
        assert_eq!(spec.len(), 3);
    }

    #[test]
    fn test_range_parse_invalid() {
        assert!(RangeSpec::parse("bytes=5-3", 10).is_none()); // start > end
        assert!(RangeSpec::parse("bytes=10-", 10).is_none()); // start >= total
        assert!(RangeSpec::parse("bytes=abc-4", 10).is_none()); // non-numeric
        assert!(RangeSpec::parse("chunks=0-4", 10).is_none()); // wrong unit
        assert!(RangeSpec::parse("", 10).is_none());
    }

    #[test]
    fn test_range_parse_clamp_end() {
        // end beyond total
        let spec = RangeSpec::parse("bytes=0-999", 100).unwrap();
        assert_eq!(spec, RangeSpec { start: 0, end: 99 });
    }

    #[test]
    fn test_range_content_range() {
        let spec = RangeSpec { start: 0, end: 4 };
        assert_eq!(spec.content_range(10), "bytes 0-4/10");
    }

    #[test]
    fn test_if_range_matches() {
        let etag = "\"abc123\"";
        assert!(if_range_matches("\"abc123\"", etag));
        assert!(!if_range_matches("\"def456\"", etag));
        // HTTP-date form -> not supported, returns false
        assert!(!if_range_matches("Sun, 24 May 2026 00:00:00 GMT", etag));
    }
}
