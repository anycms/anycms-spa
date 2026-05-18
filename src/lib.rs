pub mod core;

#[cfg(all(feature = "actix", feature = "axum"))]
compile_error!("Features `actix` and `axum` are mutually exclusive, enable only one.");

#[cfg(all(feature = "actix", not(feature = "axum")))]
pub mod actix;

#[cfg(all(feature = "axum", not(feature = "actix")))]
pub mod axum;

// 通用错误类型
pub use core::{SpaError, SpaConfig};