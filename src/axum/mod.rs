use axum::{
    http::{HeaderValue, StatusCode, Uri},
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

    pub async fn handle_request(&self, uri: Uri) -> Response {
        let path = uri.path();
        match self.handler.get_file(path) {
            Ok((content, mime)) => {
                let mime = match HeaderValue::from_str(mime) {
                    Ok(v) => v,
                    Err(_) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, "MIME error").into_response();
                    }
                };

                let is_html = mime.to_str().unwrap_or("").starts_with("text/html");
                let cache = if is_html {
                    "no-cache"
                } else {
                    "public, max-age=31536000"
                };

                let body = match content {
                    Cow::Borrowed(b) => Body::from(b),
                    Cow::Owned(v) => Body::from(v),
                };

                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", mime)
                    .header("Cache-Control", cache)
                    .body(body)
                    .unwrap()
            }
            Err(e) => {
                tracing::warn!("SPA resource not found: {} - {}", path, e);
                (StatusCode::NOT_FOUND, "Not Found").into_response()
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
                            .with_index_files(&[$($index),*]));
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
                    let svr = |uri: axum::http::Uri| async move {
                            [<mod_ $struct:lower>]::SPA.get().unwrap().handle_request(uri).await
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
