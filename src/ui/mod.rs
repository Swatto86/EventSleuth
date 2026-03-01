//! UI sub-module re-exports for EventSleuth.
//!
//! Each sub-module adds rendering methods to [`crate::app::EventSleuthApp`]
//! via `impl` blocks, keeping UI code cleanly separated from state management.

pub mod detail_panel;
pub mod event_table;
pub mod filter_panel;
pub mod stats_panel;
pub mod status_bar;
pub mod theme;
pub mod toolbar;
