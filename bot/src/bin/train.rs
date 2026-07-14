//! Train the `ml` bot's policy and write the checkpoint the bot embeds.
//!
//! Usage:
//!   train [--radius R] [--opponent NAME] [--updates N] [--batch N]
//!         [--lr F] [--seed S] [--out PATH]
//!
//! Defaults train against `random` on a small map and overwrite
//! `bot/src/ml/policy.txt`; rebuild afterwards to bake the new weights into
//! `MlBot`.

use std::process::exit;

use overthrow_bot::ml::train::{train, TrainParams};

fn main() {
    let mut params = TrainParams::default();
    let mut out = "bot/src/ml/policy.txt".to_string();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .unwrap_or_else(|| {
                    eprintln!("missing value for {name}");
                    exit(2);
                })
                .clone()
        };
        let mut number = |name: &str| -> f64 {
            value(name).parse().unwrap_or_else(|_| {
                eprintln!("{name}: not a number");
                exit(2);
            })
        };
        match arg.as_str() {
            "--radius" => params.radius = number("--radius") as i32,
            "--max-turns" => params.max_turns = number("--max-turns") as u32,
            "--opponent" => params.opponent = value("--opponent"),
            "--updates" => params.updates = number("--updates") as u32,
            "--batch" => params.batch = number("--batch") as u32,
            "--lr" => params.lr = number("--lr") as f32,
            "--seed" => params.seed = number("--seed") as u64,
            "--eval-every" => params.eval_every = number("--eval-every") as u32,
            "--eval-games" => params.eval_games = number("--eval-games") as u32,
            "--out" => out = value("--out"),
            other => {
                eprintln!("unknown argument: {other}");
                exit(2);
            }
        }
    }

    let policy = train(&params, &mut |line| println!("{line}"));

    if let Err(e) = std::fs::write(&out, policy.serialize()) {
        eprintln!("failed to write {out}: {e}");
        exit(1);
    }
    println!("wrote checkpoint to {out}");
}
