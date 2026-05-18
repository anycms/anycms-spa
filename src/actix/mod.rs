use crate::core::{SpaConfig, SpaHandler};
use actix_web::{HttpRequest, HttpResponse};

pub struct ActixSpa<E: rust_embed::RustEmbed> {
    handler: SpaHandler<E>,
}

impl<E: rust_embed::RustEmbed> ActixSpa<E> {
    pub fn new(config: SpaConfig) -> Self {
        Self {
            handler: SpaHandler::new(config),
        }
    }

    pub async fn handle_request(&self, req: HttpRequest) -> HttpResponse {
        let path = req.path();
        match self.handler.get_file(path) {
            Ok((content, mime)) => {
                let mut response = HttpResponse::Ok();
                response.content_type(mime);

                if mime.starts_with("text/html") {
                    response.insert_header(("Cache-Control", "no-cache"));
                } else {
                    response.insert_header(("Cache-Control", "public, max-age=31536000"));
                }

                response.body(content)
            }
            Err(e) => {
                tracing::warn!("SPA resource not found: {} - {}", path, e);
                HttpResponse::NotFound().finish()
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
                            [<mod_ $struct:lower>]::SPA.get().unwrap().handle_request(req).await
                        })
                }
            }
        }

    };
}
