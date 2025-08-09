pub mod core;

// #[cfg(feature="actix-web")]
pub mod actix;


// 通用错误类型
pub use core::{SpaError, SpaConfig};