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

## Observed behavior (bot-vs-bot, defaults)

- `greedy` beats `random` 200/200 on radius 5, avg ~86 turns.
- Mirror matches (`greedy,greedy`, `random,random`) are ~50/50 with no
  first-player advantage — the setup is symmetric and turns are truly
  simultaneous.
- `greedy` mirrors usually hit `max_turns` (turtling stalemate). Expected
  with a defense bonus and no economic pressure to attack; worth revisiting
  once a real frontend or smarter bots exist. Possible levers: victory by
  resource share, decay on huge stacks, or attack efficiency scaling.

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
