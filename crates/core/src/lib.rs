//! Lattice Lane domain logic.
//!
//! Pure functions only: no I/O, no clock, no randomness. Compiled into the
//! UI's WASM for live totals and into the native app for saves and exports,
//! so the screen and the saved record are always worked out by the same code.
#![cfg_attr(test, allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod api;
pub mod bin_expiry;
pub mod canvas;
pub mod colors;
pub mod cost_tree;
pub mod costing;
pub mod csv;
pub mod dimensions;
pub mod gstin;
pub mod images;
pub mod import;
pub mod js;
pub mod keybindings;
pub mod lenient;
pub mod list_return;
pub mod live;
pub mod money;
pub mod mrp;
pub mod numbers;
pub mod pricing;
pub mod product_code;
pub mod records;
pub mod reprice;
pub mod settings;
pub mod sheets;
pub mod undo;
pub mod variants;

pub use rust_decimal::Decimal;
