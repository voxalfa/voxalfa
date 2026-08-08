pub mod ast;
pub mod data_types;
pub mod diagnostics;
pub mod error;
pub mod ir;
pub mod output;
pub mod ts_utils;
pub mod validation;

#[cfg(test)]
mod tests;

pub const LIB_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SUPPORTED_VERSION: &str = "=0.1.0-alpha";
