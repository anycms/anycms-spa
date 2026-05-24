use crate::core::{SpaConfig, SpaHandler};
use salvo::http::header::{HeaderValue, CACHE_CONTROL, CONTENT_TYPE};
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
        match self.handler.get_file(path) {
            Ok(spa_resp) => {
                let cache = if spa_resp.is_html { "no-cache" } else { "public, max-age=31536000" };

                // ETag / 304 check
                if let Some(if_none_match) = req.headers().get("If-None-Match").and_then(|v| v.to_str().ok()) {
                    if crate::core::etag_matches(if_none_match, &spa_resp.etag) {
                        res.status_code(StatusCode::NOT_MODIFIED);
                        res.headers_mut().insert("ETag", HeaderValue::from_str(&spa_resp.etag).unwrap());
                        res.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static(cache));
                        return;
                    }
                }

                if let Ok(mime_val) = HeaderValue::from_str(spa_resp.mime) {
                    res.headers_mut().insert(CONTENT_TYPE, mime_val);
                }
                res.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static(cache));
                res.headers_mut().insert("ETag", HeaderValue::from_str(&spa_resp.etag).unwrap());

                #[cfg(feature = "gzip")]
                {
                    let accept_encoding = req.headers()
                        .get("Accept-Encoding")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    if let Some(ref gzip_data) = spa_resp.gzip_data {
                        if crate::core::accepts_gzip(accept_encoding) {
                            res.headers_mut().insert("Content-Encoding", HeaderValue::from_static("gzip"));
                            res.write_body(gzip_data.clone()).ok();
                            return;
                        }
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
            }
        }
    }
}

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! spa {
    ($struct:ident, $path:expr) => {
        spa!($struct, $path, "/", ["index.html"]);
    };

    ($struct:ident, $path:expr, $base:expr) => {
        spa!($struct, $path, $base, ["index.html"]);
    };

    ($struct:ident, $path:expr, $base:expr, [$($index:expr),*]) => {
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
                    [<mod_ $struct:lower>]::CONFIG.get_or_init(|| anycms_spa::core::SpaConfig::default()
                                .with_base_path($base)
                                .with_index_files(&[$($index),*]));
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
