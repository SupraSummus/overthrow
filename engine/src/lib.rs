//! Overthrow game engine: pure rules, no I/O, fully deterministic.
//!
//! This crate is the single source of truth for the game rules. It is kept
//! free of rendering, networking, threading and clock dependencies so the
//! same code can drive the desktop/Android app, headless bot-vs-bot runs,
//! and (later) reinforcement-learning training loops.

pub mod coords;
pub mod encoding;
pub mod game;
pub mod rng;

pub use coords::{Direction, Hex};
pub use encoding::{ACTIONS_PER_TILE, NUM_PLANES};
pub use game::{Config, GameState, MoveAmount, Order, Outcome, PlayerId, Tile};
