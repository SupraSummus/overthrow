//! `MlBot`: plays the trained [`Policy`] deterministically.
//!
//! The checkpoint is embedded at build time with `include_str!`, so a
//! release binary (desktop, web or Android) carries its brain with no
//! runtime file to ship or load. Retraining is a matter of overwriting
//! `policy.txt` (via the `train` binary) and rebuilding.

use overthrow_engine::{GameState, Order, PlayerId};

use crate::ml::policy::Policy;
use crate::Bot;

/// Weights produced by `cargo run -p overthrow-bot --bin train`.
const CHECKPOINT: &str = include_str!("policy.txt");

pub struct MlBot {
    policy: Policy,
}

impl MlBot {
    /// Load the embedded checkpoint. The seed is unused — play is a
    /// deterministic argmax over the policy — but kept for parity with the
    /// other bots' `new(seed)` shape so `make_bot` treats them alike.
    pub fn new(_seed: u64) -> Self {
        MlBot {
            policy: Policy::deserialize(CHECKPOINT).expect("embedded policy checkpoint is valid"),
        }
    }

    /// Build from an arbitrary checkpoint (used by the trainer to evaluate a
    /// freshly trained policy before it is committed).
    pub fn from_checkpoint(text: &str) -> Result<Self, String> {
        Ok(MlBot {
            policy: Policy::deserialize(text)?,
        })
    }
}

impl Bot for MlBot {
    fn name(&self) -> &'static str {
        "ml"
    }

    fn orders(&mut self, state: &GameState, me: PlayerId) -> Vec<Order> {
        // Deterministic argmax play (no sampling RNG, no decisions to keep).
        self.policy.orders(state, me, None).0
    }
}
