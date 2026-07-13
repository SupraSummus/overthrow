//! Headless runner: pit bots against each other, print stats, render maps.
//!
//! Usage:
//!   overthrow match [--games N] [--radius R] [--bots A,B,...] [--seed S] [--render]
//!
//! `--bots` takes 2 to 6 comma-separated names, one per player; the player
//! count follows from the list. Bots: greedy, random.

use std::env;
use std::process::exit;

use overthrow_bot::{make_bot, run_match, Bot, SeriesStats};
use overthrow_engine::{Config, GameState, Hex, PlayerId};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("match") {
        eprintln!(
            "usage: overthrow match [--games N] [--radius R] [--bots A,B,...] [--seed S] [--render]"
        );
        exit(2);
    }

    let mut games = 20u32;
    let mut radius = 5i32;
    let mut bots = vec!["greedy".to_string(), "random".to_string()];
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
                bots = v.split(',').map(str::to_string).collect();
                if !(2..=6).contains(&bots.len()) {
                    eprintln!(
                        "--bots wants 2 to 6 comma-separated names, one per player, e.g. greedy,random"
                    );
                    exit(2);
                }
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
        players: bots.len() as u8,
        ..Config::default()
    };

    let mut stats = SeriesStats::default();

    for game_index in 0..games {
        let game_seed = seed + game_index as u64;
        let mut players: Vec<Box<dyn Bot>> = bots
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
                "game {game_index}: {:?} after {} turns  ({})",
                record.outcome,
                state.turn,
                bots.join(" vs "),
            );
            print_map(&state);
        }
    }

    let standings: Vec<String> = bots
        .iter()
        .enumerate()
        .map(|(p, name)| format!("{name} [P{p}] {} wins", stats.wins_of(PlayerId(p as u8))))
        .collect();
    println!(
        "{} games, radius {}, {} players: {}, {} draws, avg {} turns",
        games,
        radius,
        bots.len(),
        standings.join(", "),
        stats.draws,
        stats.avg_turns(),
    );
    println!(
        "endings: {} by elimination, {} at the turn limit; comebacks: {} of {} decided games with an early leader",
        stats.eliminations, stats.turn_limit_endings, stats.comebacks, stats.comeback_eligible,
    );
    println!(
        "lead volatility: {:.1} lead changes per game ({} total)",
        stats.avg_lead_changes(),
        stats.total_lead_changes,
    );
}

/// ASCII map: each tile is `<owner><army>`, owner A-F by player id or
/// `.` (neutral). Rows
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
