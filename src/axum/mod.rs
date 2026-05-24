use axum::{
    http::{HeaderValue, StatusCode, Uri, HeaderMap},
    response::{IntoResponse, Response},
    body::Body,
};
use crate::core::SpaHandler;
use std::borrow::Cow;

pub struct AxumSpa<E: rust_embed::RustEmbed> {
    handler: SpaHandler<E>,
}

impl<E: rust_embed::RustEmbed> AxumSpa<E> {
    pub fn new(config: crate::core::SpaConfig) -> Self {
        Self {
            handler: SpaHandler::new(config),
        }
    }

    pub async fn handle_request(&self, uri: Uri, headers: &HeaderMap) -> Response {
        let path = uri.path();
        let security_headers = self.handler.security_headers();

        match self.handler.get_file(path) {
            Ok(spa_resp) => {
                let cache = if spa_resp.is_html { "no-cache" } else { "public, max-age=31536000" };

                // ETag / 304 check
                if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok()) {
                    if crate::core::etag_matches(if_none_match, &spa_resp.etag) {
                        let mut builder = Response::builder()
                            .status(StatusCode::NOT_MODIFIED)
                            .header("ETag", &spa_resp.etag)
                            .header("Cache-Control", cache);
                        #[cfg(any(feature = "gzip", feature = "brotli"))]
                        if spa_resp.has_compression() {
                            builder = builder.header("Vary", "Accept-Encoding");
                        }
                        for (key, value) in security_headers {
                            builder = builder.header(key.as_str(), value.as_str());
                        }
                        return builder.body(Body::empty()).unwrap();
                    }
                }

                let content_type = crate::core::content_type_with_charset(spa_resp.mime);
                let mime = match HeaderValue::from_str(&content_type) {
                    Ok(v) => v,
                    Err(_) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, "MIME error").into_response();
                    }
                };

                // Range request
                let data_len = spa_resp.data.len();
                let range_header = headers.get("Range").and_then(|v| v.to_str().ok());
                let if_range_header = headers.get("If-Range").and_then(|v| v.to_str().ok());
                let can_range = match if_range_header {
                    Some(ir) => crate::core::if_range_matches(ir, &spa_resp.etag),
                    None => true,
                };
                if let (Some(rh), true) = (range_header, can_range) {
                    if let Some(range) = crate::core::RangeSpec::parse(rh, data_len) {
                        let mut builder = Response::builder()
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header("Content-Type", &mime)
                            .header("Content-Range", range.content_range(data_len))
                            .header("Content-Length", range.len().to_string())
                            .header("Accept-Ranges", "bytes")
                            .header("ETag", &spa_resp.etag);
                        for (key, value) in security_headers {
                            builder = builder.header(key.as_str(), value.as_str());
                        }
                        return builder.body(Body::from(spa_resp.data[range.start..=range.end].to_vec())).unwrap();
                    }
                }

                #[cfg(any(feature = "gzip", feature = "brotli"))]
                {
                    let accept_encoding = headers
                        .get("Accept-Encoding")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    if let Some((encoding, data)) = spa_resp.select_encoding(accept_encoding) {
                        let mut builder = Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", &mime)
                            .header("Cache-Control", cache)
                            .header("ETag", &spa_resp.etag)
                            .header("Content-Encoding", encoding);
                        if spa_resp.has_compression() {
                            builder = builder.header("Vary", "Accept-Encoding");
                        }
                        for (key, value) in security_headers {
                            builder = builder.header(key.as_str(), value.as_str());
                        }
                        return builder.body(Body::from(data.to_vec())).unwrap();
                    }
                }

                let body = match &spa_resp.data {
                    Cow::Borrowed(b) => Body::from(*b),
                    Cow::Owned(v) => Body::from(v.clone()),
                };

                let mut builder = Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", mime)
                    .header("Cache-Control", cache)
                    .header("ETag", &spa_resp.etag)
                    .header("Accept-Ranges", "bytes");
                #[cfg(any(feature = "gzip", feature = "brotli"))]
                if spa_resp.has_compression() {
                    builder = builder.header("Vary", "Accept-Encoding");
                }
                for (key, value) in security_headers {
                    builder = builder.header(key.as_str(), value.as_str());
                }
                builder.body(body).unwrap()
            }
            Err(e) => {
                tracing::warn!("SPA resource not found: {} - {}", path, e);
                if let Some(error_resp) = self.handler.get_error_page(404) {
                    let content_type = crate::core::content_type_with_charset(error_resp.mime);
                    let mut builder = Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header("Content-Type", content_type.as_ref());
                    for (key, value) in security_headers {
                        builder = builder.header(key.as_str(), value.as_str());
                    }
                    #[cfg(any(feature = "gzip", feature = "brotli"))]
                    {
                        let accept_encoding = headers
                            .get("Accept-Encoding")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");
                        if let Some((encoding, data)) = error_resp.select_encoding(accept_encoding) {
                            return builder
                                .header("Content-Encoding", encoding)
                                .body(Body::from(data.to_vec()))
                                .unwrap();
                        }
                    }
                    let body = match error_resp.data {
                        Cow::Borrowed(b) => Body::from(b),
                        Cow::Owned(v) => Body::from(v),
                    };
                    return builder.body(body).unwrap();
                }
                let mut builder = Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Content-Type", "text/plain; charset=utf-8");
                for (key, value) in security_headers {
                    builder = builder.header(key.as_str(), value.as_str());
                }
                builder.body(Body::from("Not Found")).unwrap()
            }
        }
    }
}

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! spa {
    // 2 args: spa!(Spa, "assets")
    ($struct:ident, $path:expr $(,)?) => {
        spa!($struct, $path, "/", ["index.html"]);
    };

    // 2 args + config: spa!(Spa, "assets", { .with_xxx() })
    ($struct:ident, $path:expr, { $($config:tt)* } $(,)?) => {
        spa!($struct, $path, "/", ["index.html"], { $($config)* });
    };

    // 3 args: spa!(Spa, "assets", "/app")
    ($struct:ident, $path:expr, $base:expr $(,)?) => {
        spa!($struct, $path, $base, ["index.html"]);
    };

    // 3 args + config: spa!(Spa, "assets", "/app", { .with_xxx() })
    ($struct:ident, $path:expr, $base:expr, { $($config:tt)* } $(,)?) => {
        spa!($struct, $path, $base, ["index.html"], { $($config)* });
    };

    // 4 args: spa!(Spa, "assets", "/", ["index.html"])
    ($struct:ident, $path:expr, $base:expr, [$($index:expr),*] $(,)?) => {
        spa!($struct, $path, $base, [$($index),*], {});
    };

    // 4 args + config: spa!(Spa, "assets", "/", ["index.html"], { .with_xxx() })
    ($struct:ident, $path:expr, $base:expr, [$($index:expr),*], { $($config:tt)* } $(,)?) => {
        #[derive(rust_embed::RustEmbed)]
        #[folder = $path]
        pub struct $struct;
        paste::paste!{

            pub mod [<mod_ $struct:lower>] {
                pub static CONFIG: std::sync::OnceLock<anycms_spa::core::SpaConfig> = std::sync::OnceLock::new();
                pub static SPA: std::sync::OnceLock<anycms_spa::axum::AxumSpa<crate::$struct>> = std::sync::OnceLock::new();
            }

            impl $struct {

                pub fn spa_router() -> axum::Router {
                    use axum::{routing::get, Router};

                    [<mod_ $struct:lower>]::CONFIG.get_or_init(|| anycms_spa::core::SpaConfig::default()
                            .with_base_path($base)
                            .with_index_files(&[$($index),*])
                            $($config)*);
                    [<mod_ $struct:lower>]::SPA.get_or_init(|| anycms_spa::axum::AxumSpa::new([<mod_ $struct:lower>]::CONFIG.get().unwrap().clone()));

                    let base = $base.trim_matches('/');
                    let route_path = if base.is_empty() {
                        "/".to_string()
                    } else {
                        format!("/{}/", base)
                    };
                    let route_path_all = if base.is_empty() {
                        "/{*path}".to_string()
                    } else {
                        format!("/{}/{{*path}}", base)
                    };
                    let svr = |uri: axum::http::Uri, headers: axum::http::HeaderMap| async move {
                            [<mod_ $struct:lower>]::SPA.get().unwrap().handle_request(uri, &headers).await
                        };
                    Router::new()
                        .route(
                        &route_path_all,
                        get(svr))
                        .route(
                            &route_path,
                            get(svr))
                }
            }
        }
    };
}
