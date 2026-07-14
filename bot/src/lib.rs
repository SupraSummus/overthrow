//! Bots: strategies that map a game state to a set of orders.
//!
//! `RandomBot` is the sanity baseline,
//! `GreedyBot` a scripted heuristic,
//! and `TacticianBot` a stronger heuristic that beats it.
//! `FutureTreeBot` drops the hand-tuned priorities and picks its turn by
//! simulating each candidate order forward and keeping the one whose
//! resulting position scores best, beating `TacticianBot`.
//! `MlBot` is a learned-policy vertical slice.
//! Everything that can run these can run a future learned policy too,
//! since they all implement the same `Bot` trait.
//! `make_bot` is the name registry,
//! and `BOT_NAMES` lists every name it accepts.

use std::collections::{HashMap, HashSet, VecDeque};

use overthrow_engine::rng::Rng;
use overthrow_engine::{Config, GameState, Hex, MoveAmount, Order, Outcome, PlayerId};

pub mod future_tree;
pub mod ml;
pub mod stats;
pub mod tactician;

pub use future_tree::FutureTreeBot;
pub use ml::MlBot;
pub use stats::{MatchRecord, SeriesStats};
pub use tactician::TacticianBot;

pub trait Bot {
    fn name(&self) -> &'static str;
    fn orders(&mut self, state: &GameState, me: PlayerId) -> Vec<Order>;
}

/// Drive a game to completion, one `Bot` per player, tracing what the
/// game-health metrics need (see `stats::MatchRecord`).
pub fn run_match(config: Config, bots: &mut [Box<dyn Bot>]) -> (GameState, MatchRecord) {
    assert_eq!(bots.len(), config.players as usize);
    let mut state = GameState::new(config);
    let mut leaders = vec![stats::strict_leader(&state)];
    loop {
        let orders: Vec<_> = bots
            .iter_mut()
            .enumerate()
            .map(|(p, bot)| bot.orders(&state, PlayerId(p as u8)))
            .collect();
        let outcome = state.step(&orders);
        leaders.push(stats::strict_leader(&state));
        if outcome != Outcome::Ongoing {
            let record = MatchRecord {
                outcome,
                turns: state.turn,
                max_turns: state.config.max_turns,
                leaders,
            };
            return (state, record);
        }
    }
}

/// Keep the first order per source tile,
/// funding them from the command-point pool in priority order
/// until it runs out — mirrors what `step` will spend
/// (the last order kept may only be partly paid,
/// exactly as the engine runs it partially).
fn take_budget(state: &GameState, candidates: impl IntoIterator<Item = Order>) -> Vec<Order> {
    take_budget_floored(state, candidates.into_iter().map(|order| (order, 1)))
}

/// `take_budget` with a per-order floor.
/// `min_useful` is the fewest armies an order must actually be funded for
/// to be worth keeping:
/// the pool clamps a move or recruit to `min(cost, remaining)`,
/// and an order that would land below its floor is skipped
/// rather than committed under-strength —
/// the leverage a bot needs to refuse an attack that loses beneath `needed`
/// instead of bleeding army into it.
/// A floor of `1` (anything the pool pays for is progress)
/// recovers plain `take_budget`.
pub(crate) fn take_budget_floored(
    state: &GameState,
    candidates: impl IntoIterator<Item = (Order, u32)>,
) -> Vec<Order> {
    let mut used_sources = HashSet::new();
    let mut remaining = state.config.command_points;
    let mut chosen = Vec::new();
    for (order, min_useful) in candidates {
        if remaining == 0 {
            break;
        }
        if used_sources.contains(&order.source()) {
            continue;
        }
        let cost = state.order_cost(&order);
        if cost == 0 {
            continue;
        }
        let funded = cost.min(remaining);
        if funded < min_useful {
            continue;
        }
        used_sources.insert(order.source());
        chosen.push(order);
        remaining -= funded;
    }
    chosen
}

/// BFS distance from every tile to the nearest tile not owned by `me`.
/// One O(tiles) pass per turn;
/// interior armies descend this gradient toward the fighting.
/// Shared by the scripted bots that route reserves forward.
pub(crate) fn frontier_distances(state: &GameState, me: PlayerId) -> HashMap<Hex, u32> {
    let mut dist = HashMap::new();
    let mut queue = VecDeque::new();
    for (hex, tile) in state.iter_tiles() {
        if tile.owner != Some(me) {
            dist.insert(hex, 0);
            queue.push_back(hex);
        }
    }
    while let Some(hex) = queue.pop_front() {
        let d = dist[&hex];
        for (_, neighbor) in hex.neighbors() {
            if state.tile(neighbor).is_some() && !dist.contains_key(&neighbor) {
                dist.insert(neighbor, d + 1);
                queue.push_back(neighbor);
            }
        }
    }
    dist
}

/// Picks uniformly random legal orders (one per source tile).
pub struct RandomBot {
    rng: Rng,
}

impl RandomBot {
    pub fn new(seed: u64) -> Self {
        RandomBot {
            rng: Rng::new(seed),
        }
    }
}

impl Bot for RandomBot {
    fn name(&self) -> &'static str {
        "random"
    }

    fn orders(&mut self, state: &GameState, me: PlayerId) -> Vec<Order> {
        let mut legal = state.legal_orders(me);
        self.rng.shuffle(&mut legal);
        take_budget(state, legal)
    }
}

/// Scripted heuristic: recruit on threatened or rich tiles, grab neutral
/// neighbors, attack enemies only with clear superiority, push interior
/// armies toward the frontier.
pub struct GreedyBot {
    rng: Rng,
}

impl GreedyBot {
    pub fn new(seed: u64) -> Self {
        GreedyBot {
            rng: Rng::new(seed),
        }
    }
}

impl Bot for GreedyBot {
    fn name(&self) -> &'static str {
        "greedy"
    }

    fn orders(&mut self, state: &GameState, me: PlayerId) -> Vec<Order> {
        let frontier = frontier_distances(state, me);
        // (priority, order); higher priority first.
        let mut scored: Vec<(i64, Order)> = Vec::new();

        for (hex, tile) in state.iter_tiles() {
            if tile.owner != Some(me) {
                continue;
            }

            let enemy_next_door = hex.neighbors().any(|(_, n)| {
                state
                    .tile(n)
                    .is_some_and(|t| t.owner.is_some_and(|o| o != me) && t.army > 0)
            });

            // Recruit when threatened, or whenever a tile has piled up
            // enough resources to be worth an order.
            if tile.resources > 0 && (enemy_next_door || tile.resources >= 30) {
                let priority = if enemy_next_door {
                    900 + tile.resources as i64
                } else {
                    100 + tile.resources as i64
                };
                scored.push((priority, Order::Recruit { at: hex }));
            }

            if tile.army == 0 {
                continue;
            }

            let mut moved_somewhere = false;
            for (dir, next_hex) in hex.neighbors() {
                let Some(next) = state.tile(next_hex) else {
                    continue;
                };
                match next.owner {
                    // Neutral tile: expand. Prefer rich tiles; keep half at
                    // home when an enemy is adjacent.
                    None => {
                        let amount = if enemy_next_door || tile.army >= 8 {
                            MoveAmount::Half
                        } else {
                            MoveAmount::All
                        };
                        scored.push((
                            500 + next.resources as i64,
                            Order::Move {
                                from: hex,
                                dir,
                                amount,
                            },
                        ));
                        moved_somewhere = true;
                    }
                    // Enemy tile: attack only with a margin that strictly
                    // outnumbers the enemy garrison.
                    Some(owner) if owner != me => {
                        let needed = next.army as i64 + 1;
                        if (tile.army as i64) >= needed {
                            scored.push((
                                800 + (tile.army as i64 - needed),
                                Order::Move {
                                    from: hex,
                                    dir,
                                    amount: MoveAmount::All,
                                },
                            ));
                            moved_somewhere = true;
                        }
                    }
                    _ => {}
                }
            }

            // Interior tile with an idle army: walk it down the frontier
            // gradient.
            if !moved_somewhere && !enemy_next_door && tile.army >= 4 {
                let towards_frontier = hex
                    .neighbors()
                    .filter(|(_, n)| state.tile(*n).is_some())
                    .min_by_key(|(_, n)| frontier.get(n).copied().unwrap_or(u32::MAX))
                    .map(|(dir, _)| dir);
                if let Some(dir) = towards_frontier {
                    scored.push((
                        200 + tile.army as i64,
                        Order::Move {
                            from: hex,
                            dir,
                            amount: MoveAmount::All,
                        },
                    ));
                }
            }
        }

        // Shuffle before the stable sort so equal-priority orders don't
        // always resolve in map order.
        self.rng.shuffle(&mut scored);
        scored.sort_by_key(|&(priority, _)| std::cmp::Reverse(priority));

        take_budget(state, scored.into_iter().map(|(_, order)| order))
    }
}

/// Every name `make_bot` accepts, for CLI usage and error strings so the
/// list is written once. Keep in sync with `make_bot`'s match arms.
pub const BOT_NAMES: &[&str] = &["random", "greedy", "tactician", "ml", "future"];

pub fn make_bot(name: &str, seed: u64) -> Option<Box<dyn Bot>> {
    match name {
        "random" => Some(Box::new(RandomBot::new(seed))),
        "greedy" => Some(Box::new(GreedyBot::new(seed))),
        "tactician" => Some(Box::new(TacticianBot::new(seed))),
        "ml" => Some(Box::new(MlBot::new(seed))),
        "future" => Some(Box::new(FutureTreeBot::new(seed))),
        _ => None,
    }
}
