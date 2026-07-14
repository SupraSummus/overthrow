//! `FutureTreeBot`: picks its turn by evaluating the futures
//! each candidate order leads to,
//! instead of scoring orders by a hand-tuned priority
//! like `GreedyBot` and `TacticianBot`.
//!
//! It is the skeleton of a classical game engine —
//! search plus a static evaluation —
//! for a game that breaks the assumptions chess-style search leans on.
//! Two pieces:
//!
//! - `evaluate` scores a position from our seat,
//!   dominated by tile count (the victory metric, `GameState::outcome`)
//!   and broken by army then resources.
//!   Everything the bot wants lives here;
//!   the search is otherwise policy-free,
//!   so tuning play means tuning this one function —
//!   contrast the per-order priority constants the scripted bots carry.
//! - The turn is built by a greedy walk over the tree of order combinations:
//!   from the empty (pass) plan it repeatedly appends the order
//!   whose one-turn-ahead position evaluates highest,
//!   until nothing beats passing or the command-point pool is spent.
//!   Each candidate costs a `clone` + `step` + `evaluate`,
//!   so a turn is `O(orders^2)` of those — cheap at these map sizes.
//!
//! Two properties of the game force that shape.
//! A turn is a *set* of orders (one per tile, funded from a shared pool),
//! so its branching factor is the power set of ~one order per tile,
//! far too wide to enumerate — hence greedy, not a full tree.
//! And turns are simultaneous, so no opponent can see ours:
//! we model every opponent once with `TacticianBot`, hold it fixed,
//! and search a one-ply best response to it.
//!
//! Why one ply rather than deeper is a measured, rules-dependent call;
//! see `DESIGN.md`, "A future-evaluating bot".

use overthrow_engine::rng::Rng;
use overthrow_engine::{GameState, Order, Outcome, PlayerId};

use crate::tactician::TacticianBot;
use crate::Bot;

/// A won position, dwarfing any tile-count differential (a full board is a
/// few hundred tiles, so `W_TILES` times that stays well under this).
const WIN: i64 = 1_000_000_000;

/// Position-value weights, most valuable first. Tiles decide the game
/// (`GameState::outcome`), so a single tile outweighs any army or resource
/// edge; armies (which take and hold tiles) outweigh the resources that
/// merely become armies. The gaps are wide enough that a lower term never
/// overturns a higher one at the scales these maps reach.
const W_TILES: i64 = 10_000;
const W_ARMY: i64 = 10;
const W_RES: i64 = 1;

pub struct FutureTreeBot {
    rng: Rng,
}

impl FutureTreeBot {
    pub fn new(seed: u64) -> Self {
        FutureTreeBot {
            rng: Rng::new(seed),
        }
    }

    /// Static value of `state` from `me`'s seat: positive is good for `me`.
    /// A decided game returns `±WIN` (a draw is a failure to win, so it is
    /// negative but far above a loss); otherwise it is a weighted lead over
    /// the single strongest opponent — the seat actually being raced — in
    /// tiles, then army, then resources (see `W_TILES`).
    fn evaluate(state: &GameState, me: PlayerId) -> i64 {
        match state.outcome() {
            Outcome::Winner(p) => return if p == me { WIN } else { -WIN },
            // A draw is only reachable at the turn limit on a tie; count it
            // as a narrow loss so any winning line is preferred to it.
            Outcome::Draw => return -W_TILES,
            Outcome::Ongoing => {}
        }

        let n = state.config.players as usize;
        let mut tiles = vec![0i64; n];
        let mut army = vec![0i64; n];
        let mut res = vec![0i64; n];
        for (_, tile) in state.iter_tiles() {
            if let Some(PlayerId(p)) = tile.owner {
                let p = p as usize;
                tiles[p] += 1;
                army[p] += tile.army as i64;
                res[p] += tile.resources as i64;
            }
        }

        let me = me.0 as usize;
        // Race the leading opponent: the one whose tile count (the win
        // condition) is highest, army then resources breaking ties so the
        // comparison is against a single, well-defined rival for any player
        // count.
        let opp = (0..n)
            .filter(|&p| p != me)
            .max_by_key(|&p| (tiles[p], army[p], res[p]))
            .expect("at least two players");

        W_TILES * (tiles[me] - tiles[opp])
            + W_ARMY * (army[me] - army[opp])
            + W_RES * (res[me] - res[opp])
    }

    /// Each player's modelled orders for the current `state`, our own seat
    /// included (it is overwritten by the plan under test). Simultaneous
    /// turns mean an opponent cannot see ours, so this is computed once and
    /// held fixed while we search our reply. `TacticianBot` is the strongest
    /// scripted policy available, seeded from the state hash so the model is
    /// a deterministic function of the position (no dependence on our RNG).
    fn model(state: &GameState) -> Vec<Vec<Order>> {
        (0..state.config.players)
            .map(|p| {
                TacticianBot::new(state.state_hash().wrapping_add(p as u64))
                    .orders(state, PlayerId(p))
            })
            .collect()
    }

    /// Resolve one turn on a copy of the board with `my_plan` in our seat and
    /// the fixed `model` in the others, and return the resulting position's
    /// value to us. The lookahead behind every candidate comparison.
    fn value_after(
        state: &GameState,
        me: PlayerId,
        model: &[Vec<Order>],
        my_plan: &[Order],
    ) -> i64 {
        let mut next = state.clone();
        let mut orders = model.to_vec();
        orders[me.0 as usize] = my_plan.to_vec();
        next.step(&orders);
        Self::evaluate(&next, me)
    }
}

impl Bot for FutureTreeBot {
    fn name(&self) -> &'static str {
        "future"
    }

    fn orders(&mut self, state: &GameState, me: PlayerId) -> Vec<Order> {
        let model = Self::model(state);
        let mut pool = state.legal_orders(me);
        // Shuffle so equal-value candidates don't resolve in a fixed map
        // order (which would bias one seat's expansion direction).
        self.rng.shuffle(&mut pool);

        let mut plan: Vec<Order> = Vec::new();
        let mut best = Self::value_after(state, me, &model, &plan);
        let mut used_sources = std::collections::HashSet::new();
        let mut remaining = state.config.command_points;

        // Greedily grow the plan: on each pass, add the untried order whose
        // one-turn-ahead position evaluates highest, stopping when none beats
        // the plan we already have or the command-point pool is spent. The
        // engine funds orders in submission order, so appending in
        // marginal-value order also fixes the funding priority.
        loop {
            if remaining == 0 {
                break;
            }
            let mut chosen: Option<(i64, usize, u32)> = None;
            for (i, order) in pool.iter().enumerate() {
                if used_sources.contains(&order.source()) {
                    continue;
                }
                let cost = state.order_cost(order);
                if cost == 0 {
                    continue;
                }
                plan.push(*order);
                let value = Self::value_after(state, me, &model, &plan);
                plan.pop();
                if chosen.is_none_or(|(v, _, _)| value > v) {
                    chosen = Some((value, i, cost));
                }
            }
            match chosen {
                Some((value, i, cost)) if value > best => {
                    let order = pool[i];
                    used_sources.insert(order.source());
                    plan.push(order);
                    best = value;
                    remaining = remaining.saturating_sub(cost.min(remaining));
                }
                _ => break,
            }
        }

        plan
    }
}
