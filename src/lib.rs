pub mod core;

#[cfg(feature="actix")]
pub mod actix;

#[cfg(feature="axum")]
pub mod axum;

// 通用错误类型
pub use core::{SpaError, SpaConfig};