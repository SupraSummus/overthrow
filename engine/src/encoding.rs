//! The reinforcement-learning-facing projection of a `GameState`:
//! per-tile feature vectors (the *observation* a policy reads) and a
//! fixed-size discrete *action space* that maps one-to-one onto `Order`.
//!
//! This is not a game rule — it is a lossless-enough view of the state a
//! policy reads, and an index scheme for the orders a policy emits — so it
//! lives beside the rules rather than inside them. The rules stay the
//! single source of truth: the legality mask is derived from
//! `GameState::legal_orders`, never re-derived here, so it cannot drift from
//! what `step` will actually accept. See `DESIGN.md` ("ML plan") for how the
//! action-space size falls out of the per-step order design.
//!
//! Everything is player-relative: pass the acting player as `me` and the
//! observation is encoded from their seat (own vs. enemy), so a single set
//! of policy weights serves every seat.

use crate::coords::Direction;
use crate::game::{GameState, MoveAmount, Order, PlayerId};
use crate::Hex;

/// Feature planes per tile in an observation, in this fixed order:
/// `[own_army, enemy_army, resources, is_own, is_enemy, is_neutral]`.
/// The three army/resource planes are lightly normalized (see
/// `GameState::tile_features`); the three ownership planes are a one-hot of
/// the tile's allegiance relative to `me`, which lets a policy tell an empty
/// owned tile from a neutral one that the army planes alone cannot.
pub const NUM_PLANES: usize = 6;

/// Discrete actions rooted at a single tile:
/// six directions times two [`MoveAmount`]s (12 moves), plus one recruit.
/// A whole-board action space is this times the tile count; passing is
/// modeled as emitting no action for a tile, not as an explicit index (the
/// engine treats fewer orders as a partial turn — see `DESIGN.md`).
pub const ACTIONS_PER_TILE: usize = Direction::ALL.len() * 2 + 1;

/// Index within a tile's [`ACTIONS_PER_TILE`] block reserved for recruit;
/// the moves occupy the block below it as `dir * 2 + amount`.
const RECRUIT_SUB: usize = ACTIONS_PER_TILE - 1;

impl GameState {
    /// The canonical index of `hex` in observation and action order
    /// (map order, matching `iter_tiles`), or `None` if it is off-map.
    pub fn tile_index(&self, hex: Hex) -> Option<usize> {
        self.index.get(&hex).copied()
    }

    /// This player's normalized feature vector for a single tile, in the
    /// [`NUM_PLANES`] order documented on that constant. Armies are scaled by
    /// the starting army and resources by the resource equilibrium, so a
    /// typical value sits near `1.0`; both can exceed it (a stack larger than
    /// the opening, resources briefly above equilibrium) — the normalization
    /// is a learning convenience, not a clamp. An off-map hex is all zeros,
    /// which is also what a real neutral empty tile reads as on the army
    /// planes, so callers gathering a patch treat the two alike.
    pub fn tile_features(&self, hex: Hex, me: PlayerId) -> [f32; NUM_PLANES] {
        let Some(tile) = self.tile(hex) else {
            return [0.0; NUM_PLANES];
        };
        let army_scale = self.config.initial_army.max(1) as f32;
        let resource_scale = self.config.resource_equilibrium().max(1) as f32;
        let is_own = tile.owner == Some(me);
        let is_enemy = tile.owner.is_some() && !is_own;
        [
            if is_own {
                tile.army as f32 / army_scale
            } else {
                0.0
            },
            if is_enemy {
                tile.army as f32 / army_scale
            } else {
                0.0
            },
            tile.resources as f32 / resource_scale,
            is_own as u8 as f32,
            is_enemy as u8 as f32,
            tile.owner.is_none() as u8 as f32,
        ]
    }

    /// Number of tiles on the map — the multiplier of `action_space_size`.
    pub fn tiles_len(&self) -> usize {
        self.index.len()
    }

    /// Size of the flat discrete action space: [`ACTIONS_PER_TILE`] per tile.
    pub fn action_space_size(&self) -> usize {
        self.tiles_len() * ACTIONS_PER_TILE
    }

    /// Decode a flat action index into the `Order` it denotes, or `None` if
    /// the index is out of range. This is a pure naming of the action — it
    /// says nothing about whether the order is legal now; use
    /// `legal_action_mask` for that.
    pub fn action_to_order(&self, index: usize) -> Option<Order> {
        if index >= self.action_space_size() {
            return None;
        }
        let hex = self.hex_at(index / ACTIONS_PER_TILE)?;
        let sub = index % ACTIONS_PER_TILE;
        if sub == RECRUIT_SUB {
            return Some(Order::Recruit { at: hex });
        }
        let dir = Direction::ALL[sub / 2];
        let amount = if sub.is_multiple_of(2) {
            MoveAmount::All
        } else {
            MoveAmount::Half
        };
        Some(Order::Move {
            from: hex,
            dir,
            amount,
        })
    }

    /// Encode an `Order` as its flat action index, the inverse of
    /// `action_to_order`, or `None` if the order's source tile is off-map.
    pub fn order_to_action(&self, order: &Order) -> Option<usize> {
        let base = self.tile_index(order.source())? * ACTIONS_PER_TILE;
        let sub = match *order {
            Order::Recruit { .. } => RECRUIT_SUB,
            Order::Move { dir, amount, .. } => {
                let d = Direction::ALL.iter().position(|&x| x == dir).unwrap();
                d * 2 + (amount == MoveAmount::Half) as usize
            }
        };
        Some(base + sub)
    }

    /// A boolean mask over the whole action space: `true` where the action is
    /// a legal order for `me` right now. Built by walking
    /// `GameState::legal_orders` and lighting up each order's index, so the
    /// mask is exactly the set of orders the engine would accept — the rules
    /// stay the one source of truth for legality.
    pub fn legal_action_mask(&self, me: PlayerId) -> Vec<bool> {
        let mut mask = vec![false; self.action_space_size()];
        for order in self.legal_orders(me) {
            if let Some(i) = self.order_to_action(&order) {
                mask[i] = true;
            }
        }
        mask
    }

    /// The canonical hex at index `t` in map order, or `None` if out of range.
    fn hex_at(&self, t: usize) -> Option<Hex> {
        self.hexes.get(t).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Config;

    fn state() -> GameState {
        GameState::new(Config {
            radius: 3,
            ..Config::default()
        })
    }

    #[test]
    fn action_index_round_trips_for_every_index() {
        let s = state();
        for i in 0..s.action_space_size() {
            let order = s.action_to_order(i).unwrap();
            assert_eq!(s.order_to_action(&order), Some(i), "index {i}");
        }
    }

    #[test]
    fn tile_index_and_hex_are_inverse_over_the_whole_map() {
        let s = state();
        for (t, (hex, _)) in s.iter_tiles().enumerate() {
            assert_eq!(s.tile_index(hex), Some(t));
        }
        assert_eq!(s.tiles_len(), s.iter_tiles().count());
    }

    #[test]
    fn mask_matches_legal_orders_exactly() {
        let s = state();
        let me = PlayerId(0);
        let mask = s.legal_action_mask(me);
        // Every legal order is masked in.
        for order in s.legal_orders(me) {
            let i = s.order_to_action(&order).unwrap();
            assert!(mask[i], "legal order {order:?} not in mask");
        }
        // The counts agree: legal_orders has no duplicate index, so the
        // number of set bits equals the number of legal orders.
        let set_bits = mask.iter().filter(|&&b| b).count();
        assert_eq!(set_bits, s.legal_orders(me).len());
    }

    #[test]
    fn tile_features_are_player_relative() {
        let s = state();
        // A tile player 0 owns reads as "own" from seat 0 and "enemy" from
        // seat 1: the two seats see one board from opposite sides, which is
        // what lets one set of weights play either seat.
        let (hex, _) = s
            .iter_tiles()
            .find(|(_, t)| t.owner == Some(PlayerId(0)))
            .unwrap();
        let f0 = s.tile_features(hex, PlayerId(0));
        let f1 = s.tile_features(hex, PlayerId(1));
        assert!(f0[0] > 0.0, "owner's own-army plane should be non-zero");
        assert_eq!(f0[0], f1[1], "own(p0) army == enemy(p1) army");
        assert_eq!(f0[3], f1[4], "is_own(p0) == is_enemy(p1)");
    }

    #[test]
    fn ownership_planes_are_one_hot() {
        let s = state();
        // Planes 3,4,5 (own/enemy/neutral) partition every tile into exactly
        // one class, which is the empty-owned-vs-neutral distinction the army
        // planes alone can't make.
        for (hex, _) in s.iter_tiles() {
            let f = s.tile_features(hex, PlayerId(0));
            assert_eq!(f[3] + f[4] + f[5], 1.0, "ownership not one-hot");
        }
    }

    #[test]
    fn off_map_source_has_no_action_index() {
        let s = state();
        let off_map = Hex::new(s.config.radius + 5, 0);
        assert_eq!(s.tile_index(off_map), None);
        assert_eq!(s.order_to_action(&Order::Recruit { at: off_map }), None);
    }
}
