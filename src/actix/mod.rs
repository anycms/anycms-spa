use crate::core::{SpaConfig, SpaError, SpaHandler};
use actix_web::{HttpRequest, HttpResponse, Responder};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref PATH_RE: Regex = Regex::new(r"/+").unwrap();
}

pub struct ActixSpa<E: rust_embed::RustEmbed> {
    handler: SpaHandler<E>,
}

impl<E: rust_embed::RustEmbed> ActixSpa<E> {
    pub fn new(config: SpaConfig) -> Self {
        Self {
            handler: SpaHandler::new(config),
        }
    }

    pub async fn handle_request(&self, req: HttpRequest) -> Result<impl Responder, SpaError> {
        let path = req.path();
        let clean_path = PATH_RE.replace_all(path, "/");

        let (content, mime) = self.handler.get_file(&clean_path)?;

        let mut response = HttpResponse::Ok();
        response.content_type(mime);

        // 缓存优化：非HTML资源长期缓存
        if !mime.starts_with("text/html") {
            response.insert_header(("Cache-Control", "public, max-age=31536000"));
        }

        Ok(response.body(content))
    }
}

#[macro_export]
macro_rules! create_actix_spa {
    ($struct:ident, $path:expr) => {
        create_actix_spa!($struct, $path, "/", ["index.html"]);
    };

    ($struct:ident, $path:expr, $base:expr) => {
        create_actix_spa!($struct, $path, $base, ["index.html"]);
    };

    ($struct:ident, $path:expr, $base:expr, [$($index:expr),*]) => {
        #[derive(rust_embed::RustEmbed)]
        #[folder = $path]
        pub struct $struct;

        pub fn spa_service() -> actix_web::Resource {
            use actix_web::web;
            use lazy_static::lazy_static;

            lazy_static! {
                static ref CONFIG: crate::core::SpaConfig = crate::core::SpaConfig::default()
                    .with_base_path($base)
                    .with_index_files(&[$($index),*]);
                static ref SPA: crate::actix::ActixSpa<$struct> =
                    crate::actix::ActixSpa::new(CONFIG.clone());
            }

            let base = $base.trim_matches('/');
            let pattern = if base.is_empty() {
                "/{path:.*}".to_string()
            } else {
                format!("/{}/{{path:.*}}", base)
            };

            web::resource(&pattern)
                .to(|req: actix_web::HttpRequest| async move {
                    match SPA.handle_request(req).await {
                        Ok(res) => res,
                        Err(e) => {
                            log::error!("SPA error: {}", e);
                            actix_web::HttpResponse::NotFound().body("Not Found")
                        }
                    }
                })
        }
    };
}

/// 环境感知的 SPA 创建宏
#[macro_export]
macro_rules! env_spa {
    ($struct:ident, $dev_path:expr, $prod_path:expr) => {
        spa!($struct, $dev_path, $prod_path, "/", ["index.html"]);
    };

    ($struct:ident, $dev_path:expr, $prod_path:expr, $base:expr) => {
        spa!($struct, $dev_path, $prod_path, $base, ["index.html"]);
    };

    ($struct:ident, $dev_path:expr, $prod_path:expr, $base:expr, [$($index:expr),*]) => {
        #[cfg(debug_assertions)]
        #[derive(rust_embed::RustEmbed)]
        #[folder = $dev_path]
        pub struct $struct;

        #[cfg(not(debug_assertions))]
        #[derive(rust_embed::RustEmbed)]
        #[folder = $prod_path]
        pub struct $struct;

        pub fn spa_service() -> actix_web::Resource {
            use actix_web::web;
            use lazy_static::lazy_static;

            lazy_static! {
                static ref CONFIG: crate::core::SpaConfig = crate::core::SpaConfig::default()
                    .with_base_path($base)
                    .with_index_files(&[$($index),*]);
                static ref SPA: crate::actix::ActixSpa<$struct> =
                    crate::actix::ActixSpa::new(CONFIG.clone());
            }

            let base = $base.trim_matches('/');
            let pattern = if base.is_empty() {
                "/{path:.*}".to_string()
            } else {
                format!("/{}/{{path:.*}}", base)
            };

            web::resource(&pattern)
                .to(|req: actix_web::HttpRequest| async move {
                    match SPA.handle_request(req).await {
                        Ok(res) => res,
                        Err(e) => {
                            log::error!("SPA error: {}", e);
                            actix_web::HttpResponse::NotFound().body("Not Found")
                        }
                    }
                })
        }
    };
}
