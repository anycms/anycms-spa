pub mod core;

#[cfg(all(feature = "actix", feature = "axum"))]
compile_error!("Features `actix` and `axum` are mutually exclusive, enable only one.");

#[cfg(all(feature = "actix", feature = "salvo"))]
compile_error!("Features `actix` and `salvo` are mutually exclusive, enable only one.");

#[cfg(all(feature = "axum", feature = "salvo"))]
compile_error!("Features `axum` and `salvo` are mutually exclusive, enable only one.");

#[cfg(all(feature = "actix", not(feature = "axum"), not(feature = "salvo")))]
pub mod actix;

#[cfg(all(feature = "axum", not(feature = "actix"), not(feature = "salvo")))]
pub mod axum;

#[cfg(all(feature = "salvo", not(feature = "actix"), not(feature = "axum")))]
pub mod salvo;

// 通用错误类型
pub use core::{SpaError, SpaConfig};
/// Internal dependencies referenced by the `spa!` macro's expansion. The
/// macro body must not rely on `paste` being visible in the *invoking* crate
/// (serde's `__private` pattern), so it is re-exported here and the macro
/// refers to it as `$crate::__macro_dependencies::paste`.
#[doc(hidden)]
pub mod __macro_dependencies {
    pub use paste;
}
