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
        let security_headers = self.handler.security_headers();

        match self.handler.get_file(path) {
            Ok(spa_resp) => {
                let cache = if spa_resp.is_html { "no-cache" } else { "public, max-age=31536000" };

                // ETag / 304 check
                if let Some(if_none_match) = req.headers().get("If-None-Match").and_then(|v| v.to_str().ok()) {
                    if crate::core::etag_matches(if_none_match, &spa_resp.etag) {
                        let mut response = HttpResponse::NotModified();
                        response.insert_header(("ETag", spa_resp.etag.as_str()));
                        response.insert_header(("Cache-Control", cache));
                        #[cfg(any(feature = "gzip", feature = "brotli"))]
                        if spa_resp.has_compression() {
                            response.insert_header(("Vary", "Accept-Encoding"));
                        }
                        for (key, value) in security_headers {
                            response.insert_header((key.as_str(), value.as_str()));
                        }
                        return response.finish();
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
                        let mut response = HttpResponse::PartialContent();
                        response.content_type(crate::core::content_type_with_charset(spa_resp.mime).as_ref());
                        response.insert_header(("Content-Range", range.content_range(data_len)));
                        response.insert_header(("Content-Length", range.len().to_string()));
                        response.insert_header(("Accept-Ranges", "bytes"));
                        response.insert_header(("ETag", spa_resp.etag.as_str()));
                        for (key, value) in security_headers {
                            response.insert_header((key.as_str(), value.as_str()));
                        }
                        return response.body(spa_resp.data[range.start..=range.end].to_vec());
                    }
                }

                let mut response = HttpResponse::Ok();
                response.content_type(crate::core::content_type_with_charset(spa_resp.mime).as_ref());
                response.insert_header(("ETag", spa_resp.etag.as_str()));
                response.insert_header(("Cache-Control", cache));
                response.insert_header(("Accept-Ranges", "bytes"));

                #[cfg(any(feature = "gzip", feature = "brotli"))]
                if spa_resp.has_compression() {
                    response.insert_header(("Vary", "Accept-Encoding"));
                }

                for (key, value) in security_headers {
                    response.insert_header((key.as_str(), value.as_str()));
                }

                #[cfg(any(feature = "gzip", feature = "brotli"))]
                {
                    let accept_encoding = req.headers()
                        .get("Accept-Encoding")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    if let Some((encoding, data)) = spa_resp.select_encoding(accept_encoding) {
                        response.insert_header(("Content-Encoding", encoding));
                        return response.body(data.to_vec());
                    }
                }

                response.body(spa_resp.data)
            }
            Err(e) => {
                tracing::warn!("SPA resource not found: {} - {}", path, e);
                if let Some(error_resp) = self.handler.get_error_page(404) {
                    let mut response = HttpResponse::NotFound();
                    response.content_type(crate::core::content_type_with_charset(error_resp.mime).as_ref());
                    for (key, value) in security_headers {
                        response.insert_header((key.as_str(), value.as_str()));
                    }
                    #[cfg(any(feature = "gzip", feature = "brotli"))]
                    {
                        let accept_encoding = req.headers()
                            .get("Accept-Encoding")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");
                        if let Some((encoding, data)) = error_resp.select_encoding(accept_encoding) {
                            response.insert_header(("Content-Encoding", encoding));
                            return response.body(data.to_vec());
                        }
                    }
                    return response.body(error_resp.data);
                }
                let mut response = HttpResponse::NotFound();
                for (key, value) in security_headers {
                    response.insert_header((key.as_str(), value.as_str()));
                }
                response.finish()
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

        paste::paste!{

            pub mod [<mod_ $struct:lower>] {
                pub static CONFIG: std::sync::OnceLock<anycms_spa::core::SpaConfig> = std::sync::OnceLock::new();
                pub static SPA: std::sync::OnceLock<anycms_spa::actix::ActixSpa<crate::$struct>> = std::sync::OnceLock::new();
            }

            impl $struct {
                pub fn spa_service() -> actix_web::Resource {
                    use actix_web::web;

                [<mod_ $struct:lower>]::CONFIG.get_or_init(|| {
                    anycms_spa::core::SpaConfig::default()
                        .with_base_path($base)
                        .with_index_files(&[$($index),*])
                        .with_override_dir($path)
                        $($config)*
                });
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
