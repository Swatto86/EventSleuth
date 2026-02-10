//! Core domain modules for EventSleuth.
//!
//! Contains the event data model, background reader logic, XML parsing,
//! channel enumeration, and in-memory filtering.

pub mod channel_enumerator;
pub mod event_reader;
pub mod event_record;
pub mod filter;
pub mod xml_parser;
