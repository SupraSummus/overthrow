//! `TacticianBot`: a scripted heuristic that beats `GreedyBot`.
//!
//! Under the v1 rules a careful player wins the two-player game
//! without ever breaking a defense:
//! games are decided on tile count at `Config::max_turns`,
//! and a frontier garrison the command-point-capped attacker cannot
//! out-fund holds indefinitely
//! (why this favours the defender: `DESIGN.md`, "Why turtling dominates";
//! the combat that makes a held tile stick is `GameState::step`).
//! So the tactician wins the neutral land grab and holds it,
//! instead of trading armies into the defense bonus like `GreedyBot`.
//!
//! Three things distinguish it from `GreedyBot`:
//!
//! - Command-point-aware funding (`crate::take_budget_floored`):
//!   an attack is kept only when the pool will actually pay for a force
//!   that still beats the defender,
//!   never the under-funded overrun `GreedyBot` throws away.
//! - It claims neutral tiles toward the map centre first
//!   — the ground both players race for —
//!   to own more than half the board before contact.
//! - It garrisons only threatened tiles,
//!   and only to what survives one maximal enemy turn,
//!   spending the rest of the pool on expansion rather than over-recruiting.

use overthrow_engine::rng::Rng;
use overthrow_engine::{GameState, Hex, MoveAmount, Order, PlayerId};

use crate::{frontier_distances, take_budget_floored, Bot};

/// A scored candidate order.
/// `min_useful` is the smallest number of armies
/// the order must actually move or raise to be worth issuing —
/// `needed` for an attack (below it the attack loses and only bleeds army),
/// `1` otherwise — which the funding pass enforces.
struct Candidate {
    priority: i64,
    order: Order,
    min_useful: u32,
}

pub struct TacticianBot {
    rng: Rng,
}

impl TacticianBot {
    pub fn new(seed: u64) -> Self {
        TacticianBot {
            rng: Rng::new(seed),
        }
    }

    /// The most army an enemy could land on `hex` in one turn:
    /// the sum of enemy armies on adjacent enemy tiles,
    /// capped by the command-point pool
    /// (no player ships more than the pool a turn).
    /// An over-estimate on the funnelling side
    /// and an under-estimate against several enemies at once,
    /// but a safe garrison target for the two-player duel the ML plan targets.
    fn incoming_threat(state: &GameState, me: PlayerId, hex: Hex) -> u64 {
        let adjacent: u64 = hex
            .neighbors()
            .filter_map(|(_, n)| state.tile(n))
            .filter(|t| t.owner.is_some_and(|o| o != me))
            .map(|t| t.army as u64)
            .sum();
        adjacent.min(state.config.command_points as u64)
    }

    /// The garrison that survives `threat` incoming armies with a unit to
    /// spare: the smallest `g` with `g * defense_bonus_pct / 100 > threat`.
    /// A tile held this strongly keeps both its ownership
    /// and a defending army through one maximal enemy turn.
    fn garrison_to_hold(state: &GameState, threat: u64) -> u64 {
        let bonus = state.config.defense_bonus_pct as u64;
        // Smallest g with g*bonus/100 > threat  <=>  g*bonus > threat*100.
        (threat * 100) / bonus + 1
    }
}

impl Bot for TacticianBot {
    fn name(&self) -> &'static str {
        "tactician"
    }

    fn orders(&mut self, state: &GameState, me: PlayerId) -> Vec<Order> {
        let frontier = frontier_distances(state, me);
        let bonus = state.config.defense_bonus_pct as i64;
        let radius = state.config.radius;
        let center = Hex::new(0, 0);
        let mut candidates: Vec<Candidate> = Vec::new();

        for (hex, tile) in state.iter_tiles() {
            if tile.owner != Some(me) {
                continue;
            }

            let threat = Self::incoming_threat(state, me, hex);
            let threatened = threat > 0;

            // Reinforce a threatened tile up to a garrison that survives the
            // worst incoming turn, by converting its own resources; the more
            // exposed the tile, the earlier it is funded.
            if threatened
                && tile.resources > 0
                && (tile.army as u64) < Self::garrison_to_hold(state, threat)
            {
                candidates.push(Candidate {
                    priority: 1000 + threat as i64,
                    order: Order::Recruit { at: hex },
                    min_useful: 1,
                });
            }

            if tile.army == 0 {
                continue;
            }

            let mut acted = false;
            for (dir, next_hex) in hex.neighbors() {
                let Some(next) = state.tile(next_hex) else {
                    continue;
                };
                match next.owner {
                    // Neutral tile: claim it. Race toward the centre (the
                    // contested ground) and prefer rich tiles. Send half of a
                    // stack so the source keeps a footing, all of a lone
                    // pioneer so it keeps advancing.
                    None => {
                        let toward_center = (radius - next_hex.distance(center)) as i64;
                        let amount = if tile.army >= 2 {
                            MoveAmount::Half
                        } else {
                            MoveAmount::All
                        };
                        candidates.push(Candidate {
                            priority: 400 + 4 * toward_center + next.resources as i64,
                            order: Order::Move {
                                from: hex,
                                dir,
                                amount,
                            },
                            min_useful: 1,
                        });
                        acted = true;
                    }
                    // Enemy tile: rank a capture (a two-tile swing) by how
                    // cheap the kill is. The `needed` floor is what makes the
                    // attack safe — the funding pass drops it unless the pool
                    // pays for a force that strictly beats the bonus-scaled
                    // defender, so an under-funded attack is never issued.
                    Some(owner) if owner != me => {
                        let needed = (next.army as i64 * bonus) / 100 + 1;
                        if (tile.army as i64) >= needed {
                            candidates.push(Candidate {
                                priority: 800 - needed,
                                order: Order::Move {
                                    from: hex,
                                    dir,
                                    amount: MoveAmount::All,
                                },
                                min_useful: needed as u32,
                            });
                            acted = true;
                        }
                    }
                    _ => {}
                }
            }

            // Idle interior army: walk it toward the frontier so force reaches
            // the contested edge instead of sitting in the rear. Emit one
            // candidate per direction that strictly descends the frontier
            // gradient and let the pre-sort shuffle break ties — a
            // deterministic pick would bake a fixed compass direction into the
            // play and hand one seat the centre.
            if !acted && !threatened && tile.army >= 2 {
                let here = frontier.get(&hex).copied().unwrap_or(0);
                for (dir, next_hex) in hex.neighbors() {
                    let owned = state.tile(next_hex).is_some_and(|t| t.owner == Some(me));
                    let closer = frontier.get(&next_hex).copied().unwrap_or(u32::MAX) < here;
                    if owned && closer {
                        candidates.push(Candidate {
                            priority: 200 + tile.army as i64,
                            order: Order::Move {
                                from: hex,
                                dir,
                                amount: MoveAmount::All,
                            },
                            min_useful: 1,
                        });
                    }
                }
            }
        }

        // Shuffle before the stable sort so equal-priority orders don't always
        // resolve in map order, then fund in priority order.
        self.rng.shuffle(&mut candidates);
        candidates.sort_by_key(|c| std::cmp::Reverse(c.priority));
        take_budget_floored(
            state,
            candidates.into_iter().map(|c| (c.order, c.min_useful)),
        )
    }
}
