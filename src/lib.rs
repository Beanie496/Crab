//! Crab, a UCI-compatible chess engine for my A-level project written in Rust.

/// For unit testing.
mod bench;
/// Items related to the board. Mainly [`Board`](board::Board).
mod board;
/// Definitions and enumerations.
mod defs;
/// A container for [`Engine`](engine::Engine).
pub mod engine;
/// Handles UCI input
pub mod uci;
