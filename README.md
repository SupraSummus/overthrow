# Overthrow

A minimalist simultaneous-turn strategy game on a hex map, being rebuilt in
Rust as a single-player game against a machine-learned AI.

One unit type, no terrain, no randomness in the rules.
Each turn every player spends a small pool of command points —
one per army moved or raised, no accumulation between turns —
and all orders resolve at once.
Tiles passively accumulate resources
(quickly when poor, slowly when rich — growth works against snowballing),
and resources can be converted into armies.

The rules are documented where they are implemented, in
[`engine/`](engine/); their rationale and design history live in
[DESIGN.md](DESIGN.md). The original Django/Vue multiplayer prototype is
preserved under [`old/`](old/) as a reference implementation.

## Layout

- `engine/` — pure game rules. Deterministic, no I/O, no dependencies.
  The single source of truth, intended to be reused unchanged by the
  desktop/Android app, headless simulation, and RL training.
- `bot/` — opponents implementing the `Bot` trait: `random` (baseline),
  `greedy` (scripted heuristic), `tactician` (a stronger heuristic that
  beats `greedy`), and `ml` (a learned policy). The `ml` bot and its
  pure-Rust trainer live in `bot/src/ml/`; the state/action encoding they
  build on is `engine/src/encoding.rs`.
- `cli/` — headless runner for bot-vs-bot matches.
- `app/` — playable [macroquad](https://macroquad.rs) frontend
  (human vs bot, or spectating a bot-vs-bot match).
  The one crate builds to native desktop, web (WebAssembly) and Android;
  see [`app/README.md`](app/README.md) for the per-target build commands.

## Usage

    cargo test
    cargo run --release -p overthrow-cli -- match --games 200
    cargo run --release -p overthrow-cli -- match --games 1 --render
    cargo run --release -p overthrow-cli -- match --bots tactician,greedy --radius 6
    cargo run --release -p overthrow-cli -- match --bots ml,random
    cargo run --release -p overthrow-cli -- match --bots greedy,greedy,greedy,greedy,greedy,greedy --radius 6

    # Retrain the ml bot's policy (overwrites bot/src/ml/policy.txt; rebuild to bake it in)
    cargo run --release -p overthrow-bot --bin train

## Roadmap

1. ✅ Pure engine with simplified ruleset + scripted bots + headless CLI
2. ✅ Playable frontend (`app/`, macroquad) —
   the one crate targets native desktop, web (WebAssembly) and Android.
   macroquad was chosen over Bevy because the game is graphically trivial
   (colored hexes and numbers),
   so immediate-mode drawing fits and the ECS machinery would be overhead;
   it also covers web, which a native-only stack would not.
3. ✅ Learned bot, pure-Rust vertical slice:
   a state/action encoding in the engine,
   a tiny policy trained in-process by REINFORCE,
   playing through the same `Bot` trait as the scripted bots (`bot/src/ml/`).
4. Scale the learned policy up (larger net, self-play, richer training).
   See `DESIGN.md` ("ML plan") for the path —
   including where a framework like `candle` (training)
   and `tract` (in-app inference) would replace the hand-rolled slice.
