//! EventSleuth library crate.
//!
//! Re-exports the core modules so that integration tests and benchmarks
//! can access them. The binary entry point is in `main.rs`.

pub mod core;
pub mod export;
pub mod util;
