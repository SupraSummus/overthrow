//! Seeded gameplay-health suite: end-to-end indicators that the ruleset
//! works *as a game*, one test per design goal — see `DESIGN.md` ("How we
//! know it works as a game") for the approach and its limits. Every game
//! is seeded, so failures replay exactly. Thresholds are deliberately
//! loose: they catch a rule change that breaks a goal outright, not
//! balance drift.

use overthrow_bot::{make_bot, run_match, Bot, SeriesStats};
use overthrow_engine::{Config, PlayerId};

fn config() -> Config {
    Config {
        radius: 4,
        max_turns: 300,
        ..Config::default()
    }
}

/// Run `games` seeded matches of `bots.0` (P0) vs `bots.1` (P1).
fn run_series(bots: (&str, &str), games: u64) -> SeriesStats {
    let mut stats = SeriesStats::default();
    for seed in 0..games {
        let mut players: Vec<Box<dyn Bot>> = [bots.0, bots.1]
            .iter()
            .enumerate()
            .map(|(i, name)| make_bot(name, seed * 2 + i as u64).unwrap())
            .collect();
        let (_, record) = run_match(config(), &mut players);
        stats.record(&record);
    }
    stats
}

/// Strategy must matter: the scripted heuristic dominates the random
/// baseline, and does so by actually eliminating it, not by out-waiting
/// the turn limit.
#[test]
fn greedy_dominates_random_by_elimination() {
    let stats = run_series(("greedy", "random"), 20);
    let greedy_wins = stats.wins_of(PlayerId(0));
    assert!(
        greedy_wins >= 18,
        "greedy should dominate random, won {greedy_wins}/20"
    );
    assert!(
        stats.eliminations >= 18,
        "wins should come by elimination, got {}/20",
        stats.eliminations
    );
}

/// Fairness: strength must not depend on seat order. The setup is
/// symmetric and turns resolve simultaneously, so greedy must dominate
/// random equally as P0 and as P1.
#[test]
fn strength_is_seat_independent() {
    let as_p0 = run_series(("greedy", "random"), 10).wins_of(PlayerId(0));
    let as_p1 = run_series(("random", "greedy"), 10).wins_of(PlayerId(1));
    assert!(
        as_p0 >= 8 && as_p1 >= 8,
        "greedy should dominate from either seat, won {as_p0}/10 as P0, {as_p1}/10 as P1"
    );
}

/// Fairness: a mirror match must be roughly 50/50 — no seat holds a
/// structural advantage.
#[test]
fn mirror_match_has_no_seat_advantage() {
    let stats = run_series(("random", "random"), 40);
    let (p0, p1) = (stats.wins_of(PlayerId(0)), stats.wins_of(PlayerId(1)));
    let decided = p0 + p1;
    assert!(decided >= 20, "too few decided games ({decided}/40)");
    assert!(
        p0 * 4 <= decided * 3 && p1 * 4 <= decided * 3,
        "seat advantage: {p0} vs {p1} of {decided} decided games"
    );
}

/// Anti-snowball: leading at the quarter mark must not lock in the win.
/// If comebacks never happen, the growth curve isn't doing its job.
#[test]
fn early_lead_does_not_lock_the_win() {
    let stats = run_series(("random", "random"), 40);
    assert!(
        stats.comeback_eligible >= 10,
        "too few eligible games ({})",
        stats.comeback_eligible
    );
    assert!(
        stats.comebacks > 0,
        "no comebacks in {} eligible games: early lead looks decisive",
        stats.comeback_eligible
    );
}
