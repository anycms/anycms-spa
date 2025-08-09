use std::sync::OnceLock;

use axum::{
    http::{HeaderValue, StatusCode, Uri},
    response::Response,
};
use crate::core::{SpaConfig, SpaHandler, SpaError};
use regex::Regex;

static PATH_RE: OnceLock<Regex> = OnceLock::new();


pub struct AxumSpa<E: rust_embed::RustEmbed> {
    handler: SpaHandler<E>,
}

impl<E: rust_embed::RustEmbed> AxumSpa<E> {
    pub fn new(config: SpaConfig) -> Self {
        PATH_RE.get_or_init(|| Regex::new(r"/+").unwrap());
        Self {
            handler: SpaHandler::new(config),
        }
    }
    
    pub async fn handle_request(&self, uri: Uri) -> Result<Response, SpaError> {
        let path = uri.path();
        let clean_path = PATH_RE.get().unwrap().replace_all(path, "/");
        let (content, mime) = self.handler.get_file(&clean_path)?;
        let content = content.to_vec();
        
        let mime = HeaderValue::from_str(mime)
            .map_err(|_| SpaError::MimeDetection)?;
        
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime.clone());
        
        // 缓存优化
        if !mime.to_str().unwrap_or("").starts_with("text/html") {
            response = response.header("Cache-Control", "public, max-age=31536000");
        }
        
        Ok(response.body(content.into()).unwrap())
    }
}
#[macro_export]
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
                            match [<mod_ $struct:lower>]::SPA.get().unwrap().handle_request(uri).await {
                                Ok(res) => res,
                                Err(e) => {
                                    tracing::error!("SPA error: {}", e);
                                    axum::http::Response::builder()
                                        .status(axum::http::StatusCode::NOT_FOUND)
                                        .body(axum::body::Body::empty()).unwrap()
                                }
                            }
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