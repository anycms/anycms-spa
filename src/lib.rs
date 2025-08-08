pub mod core;

// #[cfg(feature="actix-web")]
pub mod actix;



// 重新导出宏
pub use crate::{create_actix_spa as spa};


// 通用错误类型
pub use core::{SpaError, SpaConfig};