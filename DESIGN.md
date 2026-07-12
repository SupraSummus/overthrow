# Design

## Goals (unchanged from the original project)

- No positive feedback loop: growing bigger must not make winning easier.
- No races, no action-per-minute pressure: order quality over click speed.
- Simple rules, emergent gameplay: one unit type, one map, few constants.
- Little randomness: the rules themselves are fully deterministic.

New goal driving the rewrite: the game must be **learnable and playable by a
locally-running ML agent**, which means a fast pure simulation core and a
small, discrete action space.

## Ruleset (v1, implemented in `engine/`)

`engine/src/game.rs` is the source of truth for the mechanics and their
constants: `Config` documents every rule knob and its default, `Order`
the two order kinds and the one-order-per-source-tile rule,
`GameState::step` turn resolution and combat, `GameState::outcome`
elimination and victory. Map geometry (hexagonal, cube coordinates) is
`engine/src/coords.rs`. This doc doesn't repeat any of that; it records
the two cross-cutting intents no single item can show:

- The resource growth curve (`Config::growth_divisor`) is the
  anti-snowball mechanic at the empire scale: a big empire's tiles sit
  near the cap and grow slowly, while a recovering player's poor tiles
  grow quickly — losing ground speeds your economy up instead of
  ending the game.
- The flat order budget (`Config::orders_per_turn`) is the original
  design's command-point idea discretized: order quality over click
  speed, without the earn/accumulate/spend bookkeeping (see the table
  below).

## Differences from the original (old/README.md) and why

| Original | v1 | Why |
|---|---|---|
| Move order targets any tile, engine paths one hex/turn | Move targets an adjacent tile only | Long moves are UI sugar over the same mechanic; per-step orders shrink the ML action space to `tiles × 6 × 2 + recruit` per order slot (passing = submitting fewer orders; there is no explicit pass order) |
| Command points: earned, spent per army per step, accumulated to a cap | Flat `orders_per_turn` budget, no accumulation | Same anti-APM intent, drastically simpler to learn and reason about. Accumulation existed to let idle players catch up — irrelevant for local/AI games, so it stays out (decision, 2026-07) |
| Multi-turn attrition combat with attack/defense efficiency constants | Single-step strongest-party-wins with defense bonus | Fewer states in flight, no movement records to track, easier credit assignment for RL. Attrition combat can return if fights feel too binary |
| Real-time ticks, no end | Discrete turns, `max_turns` limit, tile-count victory | RL needs episodes; a single-player game needs an ending |
| Corporations, boss trees, transfers | Omitted | Out of scope for single-player vs AI (deliberate decision, 2026-07) |
| Army cap tied to CP cap | No cap | Keep constants minimal; revisit if unbounded stacks distort play |

## How we know it works as a game

The natural way to judge "this really plays like a game" is to watch it —
render a bot match and eyeball the map evolving. That judgment is real but
it doesn't reproduce: it isn't seeded, isn't quantified, can't be compared
across commits, and nobody re-watches after every rule tweak, so it drifts
silently. The goal here is to decompose that eyeball judgment into
indicators that are **deterministic** (everything is seeded — a failure
replays exactly), **quantified**, and **tied to the design goals** above,
so a rule change that breaks a goal fails a named test instead of a vibe.
(Only the measurable goals get a proxy — "no APM pressure" and "simple
rules" are structural properties of the ruleset, not test subjects.)

Two layers, split by what they can claim:

- **Engine correctness** — necessary but not sufficient; a game can be
  bug-free and still be a bad game. `engine/tests/invariants.rs` checks
  properties that must hold in *every* reachable state of *any* game
  (including under garbage orders), that equal seeds replay to identical
  states (`GameState::state_hash`), and pins one golden game as a change
  detector.
- **Gameplay health** — proxies for the design goals, measured over
  seeded bot series. Metric definitions live in `bot/src/stats.rs`
  (`MatchRecord`, `SeriesStats`); `bot/tests/health.rs` asserts one loose
  threshold per goal; the CLI prints the same metrics for ad-hoc runs.
  The mapping: *strategy matters* → `greedy` dominates `random`, by
  elimination rather than turn-limit adjudication; *fairness /
  simultaneity* → seat-swap invariance and ~50/50 mirrors; *no
  snowball* → the comeback rate (`MatchRecord::comeback`: how often the
  quarter-mark tile leader loses anyway) is the thresholded proxy, backed
  by lead volatility (`MatchRecord::lead_changes`), an observed-only
  companion that reads the whole trajectory instead of one point and so
  catches a lead that stays locked in even when the finish is close.

Known limits, so the numbers aren't over-trusted: these are proxies, not
proof the game is *fun*; the bots bound what the metrics can see (`random`
can't punish anything, `greedy` turtles), so the indicators sharpen as the
opponents do — the RL phase feeds directly back into this suite.
Thresholds are deliberately loose change-detectors, not balance pins.
Eyeballing rendered games remains a legitimate tool — just run it from a
fixed `--seed` so what was seen can be seen again.

## Observed behavior (bot-vs-bot, radius 5, defaults)

- `greedy` beats `random` 200/200, avg ~89 turns, every win by
  elimination.
- Mirror matches are ~50/50 with no first-player advantage — the setup is
  symmetric and turns are truly simultaneous.
- *Both* mirrors always hit `max_turns` and get decided by tile-count
  adjudication, never elimination. For `random` mirrors the quarter-mark
  lead barely predicts the winner (comeback rate ~42%) and the lead
  changes hands constantly (~17 times per game) — noisy play keeps the
  race open. For `greedy` mirrors it predicted all 186 decided games (0
  comebacks) and the lead barely moves (~0.7 changes per game over 500
  turns): under a turtling stalemate whoever grabs the early tile lead
  holds it to the adjudicated finish. Lead volatility quantifies that
  turtling — a near-frozen lead over a full game, where the comeback rate
  only sees the endpoints. Expected with a defense bonus and no economic
  pressure to attack; worth revisiting once a real frontend or smarter
  bots exist. Possible levers: victory by resource share, decay on huge
  stacks, or attack efficiency scaling.

## ML plan (next phases)

1. Expose the engine to Python via PyO3 (`py/` crate) as a vectorized
   environment; train PPO self-play (small CNN/GNN over per-tile feature
   planes: own army, enemy army, resources, ownership).
2. Export the policy to ONNX; run it in-app with `tract` (pure Rust,
   Android-friendly). A policy for these map sizes is a few MB, inference
   well under a millisecond.
3. Training checkpoints double as a difficulty ladder.

Engine speed today (release, single thread): ~800 full games/sec on
radius 5 (~70k turns/sec), before any optimization. Comfortable for RL.

## Open questions

- Fog of war? Generals.io-style visibility would add depth and is standard
  for the RL formulation, but complicates the UI and the observation space.
- Should `half` splits be richer (explicit amounts?) — richer play vs
  bigger action space.
- Anti-turtling lever (see stalemate note above).
