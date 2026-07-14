//! The pure-Rust ML vertical slice: a learned bot, end to end, with no
//! external ML or linear-algebra dependency.
//!
//! The state/action *encoding* lives in the engine (`engine::encoding`) so
//! it is shared by anything that learns or plays. This module owns the rest:
//! [`policy::Policy`] (a tiny MLP over one-hop hex patches), [`train`] (an
//! episodic REINFORCE loop that plays real engine games), and [`MlBot`]
//! (which plays a trained policy through the standard `Bot` trait).
//!
//! `DESIGN.md` ("ML plan") records why this is hand-rolled rather than built
//! on an ML framework, and when that trade flips.

mod bot;
pub mod policy;
pub mod train;

pub use bot::MlBot;
pub use policy::Policy;
