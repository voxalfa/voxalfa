pub mod server;
pub mod services;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

mod actions;
mod builtin;
mod completion;
mod definition;
mod diagnostics;
mod docs;
mod parameters;
mod rename;
mod state;
mod symbols;
mod utils;

pub const SERVER_NAME: &str = "voxalfa-ls";
