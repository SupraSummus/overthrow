//! Whole-game invariant and determinism checks, driven through the public
//! API only. Where the unit tests in `game.rs` pin individual mechanics,
//! these tests assert properties that must hold across *every* reachable
//! state of *any* game — the engine-level half of the game-health
//! indicators (the gameplay half lives in `bot/tests/health.rs`).

use std::collections::HashSet;

use overthrow_engine::rng::Rng;
use overthrow_engine::{Config, Direction, GameState, Hex, MoveAmount, Order, Outcome, PlayerId};

fn test_config() -> Config {
    Config {
        radius: 3,
        max_turns: 200,
        ..Config::default()
    }
}

/// Random legal orders for every player: a candidate list roughly the size
/// of the CP pool (the engine funds as many as the pool covers).
fn random_legal_orders(state: &GameState, rng: &mut Rng) -> Vec<Vec<Order>> {
    (0..state.config.players)
        .map(|p| {
            let legal = state.legal_orders(PlayerId(p));
            (0..state.config.command_points)
                .filter_map(|_| {
                    if legal.is_empty() {
                        None
                    } else {
                        Some(legal[rng.below(legal.len())])
                    }
                })
                .collect()
        })
        .collect()
}

/// Random orders drawn from the full order space — off-map hexes, tiles the
/// player doesn't own, empty sources. The engine must drop these silently.
fn random_garbage_orders(state: &GameState, rng: &mut Rng) -> Vec<Vec<Order>> {
    let r = state.config.radius;
    let random_hex = |rng: &mut Rng| {
        // Sample from a box larger than the map so off-map hexes occur.
        Hex::new(
            rng.below((4 * r + 1) as usize) as i32 - 2 * r,
            rng.below((4 * r + 1) as usize) as i32 - 2 * r,
        )
    };
    (0..state.config.players)
        .map(|_| {
            (0..state.config.command_points + 2)
                .map(|_| {
                    let source = random_hex(rng);
                    if rng.below(4) == 0 {
                        Order::Recruit { at: source }
                    } else {
                        Order::Move {
                            from: source,
                            dir: Direction::ALL[rng.below(6)],
                            amount: if rng.below(2) == 0 {
                                MoveAmount::All
                            } else {
                                MoveAmount::Half
                            },
                        }
                    }
                })
                .collect()
        })
        .collect()
}

/// Panics if any cross-state invariant is violated.
fn check_invariants(state: &GameState, ever_owned: &mut HashSet<Hex>) {
    for (hex, tile) in state.iter_tiles() {
        match tile.owner {
            None => {
                assert_eq!(tile.army, 0, "neutral tile {hex:?} holds an army");
                assert!(
                    !ever_owned.contains(&hex),
                    "tile {hex:?} reverted to neutral"
                );
            }
            Some(PlayerId(p)) => {
                assert!(
                    p < state.config.players,
                    "tile {hex:?} owned by out-of-range player {p}"
                );
                ever_owned.insert(hex);
            }
        }
        // Resources grow up to the production/maintenance equilibrium and
        // stop; nothing raises them past it (recruit only lowers them), so
        // it is a hard ceiling.
        assert!(
            tile.resources <= state.config.resource_equilibrium(),
            "tile {hex:?} exceeds its resource equilibrium"
        );
    }
    assert!(
        state.turn <= state.config.max_turns,
        "game ran past max_turns"
    );
}

/// Drive seeded games to completion, checking the invariants after every
/// step. `orders_for` picks the order stream (legal or garbage).
fn run_checked_games(
    seed_base: u64,
    games: u64,
    orders_for: fn(&GameState, &mut Rng) -> Vec<Vec<Order>>,
) {
    for seed in seed_base..seed_base + games {
        let mut rng = Rng::new(seed);
        let mut state = GameState::new(test_config());
        let mut ever_owned = HashSet::new();
        check_invariants(&state, &mut ever_owned);
        loop {
            let expected_turn = state.turn + 1;
            let outcome = state.step(&orders_for(&state, &mut rng));
            assert_eq!(state.turn, expected_turn, "turn must advance by one");
            check_invariants(&state, &mut ever_owned);
            match outcome {
                Outcome::Ongoing => assert!(
                    state.turn < state.config.max_turns,
                    "game still ongoing at max_turns"
                ),
                _ => break,
            }
        }
    }
}

#[test]
fn invariants_hold_through_random_legal_games() {
    run_checked_games(0, 10, random_legal_orders);
}

#[test]
fn garbage_orders_never_panic_or_break_invariants() {
    run_checked_games(100, 10, random_garbage_orders);
}

#[test]
fn same_seed_replays_to_identical_states() {
    let play = |seed: u64| -> Vec<u64> {
        let mut rng = Rng::new(seed);
        let mut state = GameState::new(test_config());
        let mut hashes = vec![state.state_hash()];
        while state.step(&random_legal_orders(&state, &mut rng)) == Outcome::Ongoing {
            hashes.push(state.state_hash());
        }
        hashes.push(state.state_hash());
        hashes
    };
    for seed in 0..3 {
        assert_eq!(
            play(seed),
            play(seed),
            "same seed must replay to the same per-turn state hashes"
        );
    }
}

/// Change detector: the exact final state of one fixed seeded game. Any
/// change to the rules, `legal_orders` ordering, or the RNG moves this
/// value — that's the point. Update the constant deliberately when a rule
/// change is intended, never to silence an unexpected failure.
#[test]
fn golden_game_final_state_is_pinned() {
    let mut rng = Rng::new(42);
    let mut state = GameState::new(test_config());
    while state.step(&random_legal_orders(&state, &mut rng)) == Outcome::Ongoing {}
    assert_eq!(
        (state.turn, state.state_hash()),
        (GOLDEN_TURNS, GOLDEN_HASH),
        "engine behavior changed; if intended, update the golden values"
    );
}

const GOLDEN_TURNS: u32 = 200;
const GOLDEN_HASH: u64 = 1061877323026431320;
