use crate::core::{SpaConfig, SpaHandler};
use salvo::http::header::{HeaderName, HeaderValue, CACHE_CONTROL, CONTENT_TYPE};
use salvo::http::StatusCode;
use salvo::{Request, Response};
use std::borrow::Cow;

pub struct SalvoSpa<E: rust_embed::RustEmbed> {
    handler: SpaHandler<E>,
}

impl<E: rust_embed::RustEmbed> SalvoSpa<E> {
    pub fn new(config: SpaConfig) -> Self {
        Self {
            handler: SpaHandler::new(config),
        }
    }

    pub async fn handle_request(&self, req: &mut Request, res: &mut Response) {
        let path = req.uri().path();
        let security_headers = self.handler.security_headers();

        match self.handler.get_file(path) {
            Ok(spa_resp) => {
                let cache = if spa_resp.is_html { "no-cache" } else { "public, max-age=31536000" };

                // ETag / 304 check
                if let Some(if_none_match) = req.headers().get("If-None-Match").and_then(|v| v.to_str().ok()) {
                    if crate::core::etag_matches(if_none_match, &spa_resp.etag) {
                        res.status_code(StatusCode::NOT_MODIFIED);
                        res.headers_mut().insert("ETag", HeaderValue::from_str(&spa_resp.etag).unwrap());
                        res.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static(cache));
                        #[cfg(any(feature = "gzip", feature = "brotli"))]
                        if spa_resp.has_compression() {
                            res.headers_mut().insert("Vary", HeaderValue::from_static("Accept-Encoding"));
                        }
                        for (key, value) in security_headers {
                            if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(key.as_bytes()), HeaderValue::from_str(value)) {
                                res.headers_mut().insert(name, val);
                            }
                        }
                        return;
                    }
                }

                // Range request
                let data_len = spa_resp.data.len();
                let range_header = req.headers().get("Range").and_then(|v| v.to_str().ok());
                let if_range_header = req.headers().get("If-Range").and_then(|v| v.to_str().ok());
                let can_range = match if_range_header {
                    Some(ir) => crate::core::if_range_matches(ir, &spa_resp.etag),
                    None => true,
                };
                if let (Some(rh), true) = (range_header, can_range) {
                    if let Some(range) = crate::core::RangeSpec::parse(rh, data_len) {
                        let content_type = crate::core::content_type_with_charset(spa_resp.mime);
                        res.status_code(StatusCode::PARTIAL_CONTENT);
                        if let Ok(mime_val) = HeaderValue::from_str(&content_type) {
                            res.headers_mut().insert(CONTENT_TYPE, mime_val);
                        }
                        if let Ok(v) = HeaderValue::from_str(&range.content_range(data_len)) {
                            res.headers_mut().insert("Content-Range", v);
                        }
                        if let Ok(v) = HeaderValue::from_str(&range.len().to_string()) {
                            res.headers_mut().insert("Content-Length", v);
                        }
                        res.headers_mut().insert("Accept-Ranges", HeaderValue::from_static("bytes"));
                        res.headers_mut().insert("ETag", HeaderValue::from_str(&spa_resp.etag).unwrap());
                        for (key, value) in security_headers {
                            if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(key.as_bytes()), HeaderValue::from_str(value)) {
                                res.headers_mut().insert(name, val);
                            }
                        }
                        res.write_body(spa_resp.data[range.start..=range.end].to_vec()).ok();
                        return;
                    }
                }

                let content_type = crate::core::content_type_with_charset(spa_resp.mime);
                if let Ok(mime_val) = HeaderValue::from_str(&content_type) {
                    res.headers_mut().insert(CONTENT_TYPE, mime_val);
                }
                res.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static(cache));
                res.headers_mut().insert("ETag", HeaderValue::from_str(&spa_resp.etag).unwrap());
                res.headers_mut().insert("Accept-Ranges", HeaderValue::from_static("bytes"));

                #[cfg(any(feature = "gzip", feature = "brotli"))]
                if spa_resp.has_compression() {
                    res.headers_mut().insert("Vary", HeaderValue::from_static("Accept-Encoding"));
                }

                for (key, value) in security_headers {
                    if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(key.as_bytes()), HeaderValue::from_str(value)) {
                        res.headers_mut().insert(name, val);
                    }
                }

                #[cfg(any(feature = "gzip", feature = "brotli"))]
                {
                    let accept_encoding = req.headers()
                        .get("Accept-Encoding")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    if let Some((encoding, data)) = spa_resp.select_encoding(accept_encoding) {
                        res.headers_mut().insert("Content-Encoding", HeaderValue::from_static(encoding));
                        res.write_body(data.to_vec()).ok();
                        return;
                    }
                }

                match spa_resp.data {
                    Cow::Borrowed(b) => {
                        res.write_body(b.to_vec()).ok();
                    }
                    Cow::Owned(v) => {
                        res.write_body(v).ok();
                    }
                }
            }
            Err(e) => {
                tracing::warn!("SPA resource not found: {} - {}", path, e);
                res.status_code(StatusCode::NOT_FOUND);
                for (key, value) in security_headers {
                    if let (Ok(name), Ok(val)) = (HeaderName::from_bytes(key.as_bytes()), HeaderValue::from_str(value)) {
                        res.headers_mut().insert(name, val);
                    }
                }
                if let Some(error_resp) = self.handler.get_error_page(404) {
                    let content_type = crate::core::content_type_with_charset(error_resp.mime);
                    if let Ok(mime_val) = HeaderValue::from_str(&content_type) {
                        res.headers_mut().insert(CONTENT_TYPE, mime_val);
                    }
                    #[cfg(any(feature = "gzip", feature = "brotli"))]
                    {
                        let accept_encoding = req.headers()
                            .get("Accept-Encoding")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");
                        if let Some((encoding, data)) = error_resp.select_encoding(accept_encoding) {
                            res.headers_mut().insert("Content-Encoding", HeaderValue::from_static(encoding));
                            res.write_body(data.to_vec()).ok();
                            return;
                        }
                    }
                    match error_resp.data {
                        Cow::Borrowed(b) => {
                            res.write_body(b.to_vec()).ok();
                        }
                        Cow::Owned(v) => {
                            res.write_body(v).ok();
                        }
                    }
                    return;
                }
            }
        }
    }
}

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! spa {
    ($struct:ident, $path:expr $(,)?) => {
        spa!($struct, $path, "/", ["index.html"]);
    };

    ($struct:ident, $path:expr, { $($config:tt)* } $(,)?) => {
        spa!($struct, $path, "/", ["index.html"], { $($config)* });
    };

    ($struct:ident, $path:expr, $base:expr $(,)?) => {
        spa!($struct, $path, $base, ["index.html"]);
    };

    ($struct:ident, $path:expr, $base:expr, { $($config:tt)* } $(,)?) => {
        spa!($struct, $path, $base, ["index.html"], { $($config)* });
    };

    ($struct:ident, $path:expr, $base:expr, [$($index:expr),*] $(,)?) => {
        spa!($struct, $path, $base, [$($index),*], {});
    };

    ($struct:ident, $path:expr, $base:expr, [$($index:expr),*], { $($config:tt)* } $(,)?) => {
        #[derive(rust_embed::RustEmbed)]
        #[folder = $path]
        pub struct $struct;

        paste::paste! {

            pub mod [<mod_ $struct:lower>] {
                pub static CONFIG: std::sync::OnceLock<anycms_spa::core::SpaConfig> = std::sync::OnceLock::new();
                pub static SPA: std::sync::OnceLock<anycms_spa::salvo::SalvoSpa<crate::$struct>> = std::sync::OnceLock::new();

                #[::salvo::handler]
                pub async fn handle(req: &mut ::salvo::Request, res: &mut ::salvo::Response) {
                    SPA.get().unwrap().handle_request(req, res).await;
                }
            }

            impl $struct {
                pub fn spa_router() -> salvo::Router {
                    [<mod_ $struct:lower>]::CONFIG.get_or_init(|| {
                        anycms_spa::core::SpaConfig::default()
                            .with_base_path($base)
                            .with_index_files(&[$($index),*])
                            .with_override_dir($path)
                            $($config)*
                    });
                    [<mod_ $struct:lower>]::SPA.get_or_init(|| anycms_spa::salvo::SalvoSpa::new([<mod_ $struct:lower>]::CONFIG.get().unwrap().clone()));

                    let base = $base.trim_matches('/');
                    let route_path_all = if base.is_empty() {
                        "/{**rest}".to_string()
                    } else {
                        format!("/{}/{{**rest}}", base)
                    };
                    let route_path = if base.is_empty() {
                        "/".to_string()
                    } else {
                        format!("/{}", base)
                    };

                    salvo::Router::new()
                        .push(salvo::Router::with_path(&route_path_all).get([<mod_ $struct:lower>]::handle))
                        .push(salvo::Router::with_path(&route_path).get([<mod_ $struct:lower>]::handle))
                }
            }
        }
    };
}
