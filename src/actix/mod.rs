use std::sync::OnceLock;
use crate::core::{SpaConfig, SpaError, SpaHandler};
use actix_web::{HttpRequest, HttpResponse};
use regex::Regex;

static PATH_RE: OnceLock<Regex> = OnceLock::new();

pub struct ActixSpa<E: rust_embed::RustEmbed> {
    handler: SpaHandler<E>,
}

impl<E: rust_embed::RustEmbed> ActixSpa<E> {
    pub fn new(config: SpaConfig) -> Self {
        PATH_RE.get_or_init(|| Regex::new(r"/+").unwrap());
        Self {
            handler: SpaHandler::new(config),
        }
    }

    pub async fn handle_request(
        &self,
        req: HttpRequest,
    ) -> Result<impl actix_web::Responder, SpaError> {
        let path = req.path();
        let clean_path = PATH_RE.get().unwrap().replace_all(path, "/");
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
                pub static SPA: std::sync::OnceLock<anycms_spa::actix::ActixSpa<crate::$struct>> = std::sync::OnceLock::new();
            }
        
            impl $struct {
                pub fn spa_service() -> actix_web::Resource {
                    use actix_web::web;

                [<mod_ $struct:lower>]::CONFIG.get_or_init(|| anycms_spa::core::SpaConfig::default()
                            .with_base_path($base)
                            .with_index_files(&[$($index),*]));
                    [<mod_ $struct:lower>]::SPA.get_or_init(|| anycms_spa::actix::ActixSpa::new([<mod_ $struct:lower>]::CONFIG.get().unwrap().clone()));
                    let base = $base.trim_matches('/');
                    let pattern = if base.is_empty() {
                        "/{path:.*}".to_string()
                    } else {
                        format!("/{}/{{path:.*}}", base)
                    };

                    web::resource(&pattern)
                        .to(|req: actix_web::HttpRequest| async move {
                            let result = [<mod_ $struct:lower>]::SPA.get().unwrap().handle_request(req).await;
                            // TODO 修复这个 unrwap , 闭包情况下， match 需要返回完全一样的返回类型
                            result.unwrap()
                        })
                }
            }
        }
        
    };
}
