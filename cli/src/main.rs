//! Headless runner: pit bots against each other, print stats, render maps.
//!
//! Usage:
//!   overthrow match [--games N] [--radius R] [--bots A,B] [--seed S] [--render]
//!
//! Bots: greedy, random.

use std::env;
use std::process::exit;

use overthrow_bot::{make_bot, run_match, Bot, SeriesStats};
use overthrow_engine::{Config, GameState, Hex, PlayerId};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("match") {
        eprintln!(
            "usage: overthrow match [--games N] [--radius R] [--bots A,B] [--seed S] [--render]"
        );
        exit(2);
    }

    let mut games = 20u32;
    let mut radius = 5i32;
    let mut bots = ("greedy".to_string(), "random".to_string());
    let mut seed = 1u64;
    let mut render = false;

    let mut it = args[1..].iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .unwrap_or_else(|| {
                    eprintln!("missing value for {name}");
                    exit(2);
                })
                .clone()
        };
        match arg.as_str() {
            "--games" => games = value("--games").parse().expect("--games: not a number"),
            "--radius" => radius = value("--radius").parse().expect("--radius: not a number"),
            "--seed" => seed = value("--seed").parse().expect("--seed: not a number"),
            "--bots" => {
                let v = value("--bots");
                let (a, b) = v.split_once(',').unwrap_or_else(|| {
                    eprintln!("--bots wants two comma-separated names, e.g. greedy,random");
                    exit(2);
                });
                bots = (a.to_string(), b.to_string());
            }
            "--render" => render = true,
            other => {
                eprintln!("unknown argument: {other}");
                exit(2);
            }
        }
    }

    let config = Config {
        radius,
        ..Config::default()
    };

    let mut stats = SeriesStats::default();

    for game_index in 0..games {
        let game_seed = seed + game_index as u64;
        let mut players: Vec<Box<dyn Bot>> = [&bots.0, &bots.1]
            .iter()
            .enumerate()
            .map(|(i, name)| {
                make_bot(name, game_seed.wrapping_mul(2).wrapping_add(i as u64)).unwrap_or_else(
                    || {
                        eprintln!("unknown bot: {name} (available: greedy, random)");
                        exit(2);
                    },
                )
            })
            .collect();

        let (state, record) = run_match(config.clone(), &mut players);
        stats.record(&record);

        if render {
            println!(
                "game {game_index}: {:?} after {} turns  ({} vs {})",
                record.outcome, state.turn, bots.0, bots.1
            );
            print_map(&state);
        }
    }

    println!(
        "{} games, radius {}: {} [P0] {} wins, {} [P1] {} wins, {} draws, avg {} turns",
        games,
        radius,
        bots.0,
        stats.wins_of(PlayerId(0)),
        bots.1,
        stats.wins_of(PlayerId(1)),
        stats.draws,
        stats.avg_turns(),
    );
    println!(
        "endings: {} by elimination, {} at the turn limit; comebacks: {} of {} decided games with an early leader",
        stats.eliminations, stats.turn_limit_endings, stats.comebacks, stats.comeback_eligible,
    );
}

/// ASCII map: each tile is `<owner><army>`, owner A/B/. (neutral). Rows
/// follow the z axis; each row is half a cell narrower per step away from
/// the center, giving the classic hexagon silhouette.
fn print_map(state: &GameState) {
    let r = state.config.radius;
    for z in -r..=r {
        print!("{}", "  ".repeat(z.unsigned_abs() as usize));
        for x in (-r).max(-r - z)..=r.min(r - z) {
            let hex = Hex::new(x, -x - z);
            let tile = state.tile(hex).expect("in-map hex");
            let owner = match tile.owner {
                Some(PlayerId(p)) => (b'A' + p) as char,
                None => '.',
            };
            print!("{owner}{:<3}", tile.army.min(999));
        }
        println!();
    }
}
