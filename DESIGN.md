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

World
- Hexagonal map of radius `R` (3R²+3R+1 tiles), cube coordinates.
- A tile has: owner (or neutral), army count, resource count.
- Every tile grows resources each turn:
  `growth = max(1, (resource_cap − resources) / growth_divisor)` —
  fast when poor, asymptotically slow near the cap. This is the
  anti-snowball mechanic: a big empire's tiles are rich and grow slowly,
  a recovering player's tiles grow quickly.

Turns
- Simultaneous: each player submits up to `orders_per_turn` orders
  (default 3); all resolve in the same tick. This is the command-point
  idea from the original design, discretized into an order budget.
  Orders are taken in submitted order until the budget is spent;
  illegal orders are dropped without consuming budget.
- Orders (**at most one order per source tile per turn**, of either kind):
  - **Move** `(tile, direction, all|half)` — send armies to an *adjacent*
    tile.
  - **Recruit** `(tile)` — convert all of a tile's resources into armies.
- Resolution within a tick: all orders apply "at once" (departures leave,
  recruits raise), then everything lands and fights, then resources grow.
  The one-order-per-tile rule means recruited armies defend the same turn
  but cannot move until the next turn.

Combat (single-step, deterministic)
- All armies arriving on a tile plus its garrison form per-player parties.
- The defender's party — garrison plus any same-owner arrivals — gets a
  defense bonus (default 1.25×).
- The strongest party survives, paying the summed effective strength of
  all other parties; ties annihilate everyone. Survivors are converted
  back from effective to actual units, rounding down.

Victory
- A player with no tiles is eliminated.
- At `max_turns`, most tiles wins; ties are draws.

Players
- 2 to 6, one map corner each (opposite corners for 2, evenly spread as
  the six corners allow otherwise). Neutral tiles never hold armies.

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
