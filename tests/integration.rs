//! Integration tests for EventSleuth.
//!
//! These tests exercise the real runtime on a Windows system, validating
//! that the core modules work together correctly with real Windows APIs.
//! They run as part of `cargo test` and require a Windows environment.

mod constants_validation;
mod error_types;
mod export_validation;
mod filter_roundtrip;
mod time_utils;
