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

- The resource flow (`Config::production` minus a stockpile-scaled
  `Config::maintenance_pct`) is the anti-snowball mechanic at the empire
  scale: a big empire's tiles sit near the equilibrium and grow slowly,
  while a recovering player's poor tiles grow quickly — losing ground
  speeds your economy up instead of ending the game. The equilibrium
  (`Config::resource_equilibrium`) is emergent from two meaningful knobs
  rather than a hard-cap constant, but this is the same *kind* of curve as
  the original grow-toward-a-cap rule — tuned leaner — not a new pressure:
  resources never rise above the equilibrium, so there is no upkeep on a
  hoard and no bite on passivity. The lever against turtling is army-side
  (stack decay or garrison upkeep), noted in "Why turtling dominates"
  below.
- The command-point pool (`Config::command_points`)
  is the original design's anti-APM currency,
  kept per-army but stripped of accumulation:
  each turn a player gets a fresh pool
  and spends one CP per army moved or raised,
  so force projection — not click speed — is the limiting resource.
  Dropping the earn/accumulate/spend bookkeeping is the one simplification
  (see the table below);
  the per-army cost is what makes big offensives expensive and staged
  rather than free.

## Differences from the original (old/README.md) and why

| Original | v1 | Why |
|---|---|---|
| Move order targets any tile, engine paths one hex/turn | Move targets an adjacent tile only | Long moves are UI sugar over the same mechanic; per-step orders shrink the ML action space to `tiles × 6 × 2 + recruit` per order slot (passing = submitting fewer orders; there is no explicit pass order) |
| Command points: earned, spent per army per step, accumulated to a cap | Per-army `command_points` pool spent per step, no accumulation | Kept the per-army cost — it makes force projection the scarce resource and offensives staged rather than free. Dropped only accumulation, which existed to let idle players catch up — irrelevant for local/AI games (decision, 2026-07) |
| Multi-turn attrition combat with attack/defense efficiency constants | Single step, no defense — only mutual attack: a stationary garrison deals no damage, so a winning attacker keeps its full force | Fewer states in flight, no movement records to track, easier credit assignment for RL. A garrison is presence you must out-deliver, not an army that fights back; the defender bonus that shipped in early v1 went with it (see "Why turtling dominates") |
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
  The mapping: *strategy matters* → `greedy` dominates `random`, with
  conquest a common outcome rather than every game grinding to the turn
  limit (the per-army command-point pool makes a decisive overrun
  expensive, so a healthy share end by elimination and the rest on
  territory at `max_turns`); *fairness /
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

- `greedy` beats `random` 200/200, avg ~420 turns, about three-quarters by
  elimination and the rest decided on territory at `max_turns`.
- `tactician` beats `greedy` from either seat — 200/200 at radius 5, and a
  clean sweep at every radius tested from 4 up (4, 6, 7) — but never by
  elimination: like the `greedy` mirror, every game runs to `max_turns` and
  is decided on tile count. It wins the opening land grab (claiming toward
  the contested centre with pool-efficient moves, holding the frontier with
  garrisons the capped attacker cannot out-fund) and freezes ahead, rather
  than trading armies into a full-strength garrison. So the `greedy`-mirror freeze
  was partly a `greedy` limitation — a better land-grabber simply owns more
  of the board when the lead locks in — not purely structural; `tactician`
  still cannot break `greedy`'s turtle, only out-turtle it (see "Why
  turtling dominates" below). Its own mirror keeps a mild first-seat lean
  (~55/45 over 400 games, where `greedy`'s is even); the likeliest cause is
  combat's tie-break — `GameState::step` awards an exact-strength clash to
  the lower player id — which its centre-axis march runs into more often
  than `greedy`'s looser expansion, but that is unverified analysis, not a
  measured decomposition.
- `future` (the future-evaluating bot; see "A future-evaluating bot")
  tops the scripted ladder: at radius 4 in seeded series it sweeps
  `tactician` from either seat.
  It wins mostly by holding territory to the tile-count adjudication,
  but now that removing the defender bonus has loosened the turtle
  it also takes the occasional game by elimination.
  Its mirror is the *most* contested match measured —
  ~60 lead changes per game and comebacks in nearly half of games,
  against the `tactician` mirror's near-frozen ~1.6 —
  because two strong best-responders keep trading the frontier
  instead of freezing an early lead.
- Mirror matches are ~50/50 with no first-player advantage — the setup is
  symmetric and turns are truly simultaneous.
- *Both* mirrors always hit `max_turns` and get decided by tile-count
  adjudication, never elimination. For `random` mirrors the quarter-mark
  lead barely predicts the winner (comeback rate ~35%) and the lead
  changes hands constantly (~8 times per game) — noisy play keeps the
  race open. For `greedy` mirrors it nearly always predicts the winner (a
  handful of comebacks over ~190 decided games) and the lead barely moves
  (~1.6 changes per game over 500 turns): under a turtling stalemate
  whoever grabs the early tile lead holds it to the adjudicated finish.
  Lead volatility quantifies that turtling — a near-frozen lead over a
  full game, where the comeback rate only sees the endpoints.
  Why the freeze is structural rather than a bot limitation —
  and the levers against it —
  is the subject of "Why turtling dominates" below.
- `ml` (the learned vertical-slice policy, trained only at radius 3;
  see "ML plan") beats `random` from either seat
  at every radius tested (3, 4, 5), 50/50 in seeded CLI series —
  and, like the scripted bots, wins by holding territory
  to the `max_turns` adjudication rather than by elimination.
  That it generalizes to radii it never trained on
  is the encoding earning its keep:
  the policy reads only a tile's one-hop patch, player-relative,
  so map size is invisible to it.
  It has not been pitted against `greedy`/`tactician`
  as a measured series yet —
  beating `random` is the slice's bar;
  climbing the scripted ladder is the scaling work.
- Six-player free-for-alls (radius 6, one bot per corner) behave the same
  way at the top level: every game still runs to `max_turns` and is
  adjudicated, never won by elimination, for both mirrors — six `greedy`
  turtles no more break the stalemate than two do. The seats stay fair
  (wins spread evenly across the six corners, no positional advantage), and
  the turtling signature carries over: the `greedy` six-way barely moves
  its lead (~3 changes per game, comeback rate ~6%) while the `random`
  six-way is pure churn (~30 changes per game, comeback rate ~77%). Mixing
  three `greedy` against three `random` around the ring, the `greedy` seats
  take every decided game and the `random` seats win none. This measures
  the multi-player prediction in "Why turtling dominates" below: more
  attackers do not, in practice, out-deliver a turtle's defense.

## Why turtling dominates (v1 rules)

Analysis (2026-07): the greedy-mirror freeze is structural,
not a bot limitation —
under the v1 rules turtling is dominant in the two-player game,
so a stronger opponent (including a learned one)
would turtle harder, not break the stalemate.
Three legs:

- Offense is capped, presence is not.
  The command-point pool is charged per army moved,
  out of one shared per-turn pool,
  so a player delivers at most `Config::command_points` armies
  to any one tile in a turn
  no matter how long they stage,
  while a garrison persists indefinitely.
  Taking a tile means out-presenting its garrison in a single turn,
  so a garrison matching the pool (20 at defaults) cannot be taken:
  the most an attack can muster only matches it, which annihilates.
- Cheaper offense does not obviously help.
  A won attack keeps the attacker's full force —
  the garrison strikes for nothing —
  so offense costs less than it did under attrition,
  yet no single turn can still deliver past a pool-sized garrison,
  and whittling one down trades even
  while the defender re-recruits from its own equal pool.
  Both sides convert the same per-turn pool into board effect,
  so neither out-delivers the other's regeneration.
- Passivity is free and wins:
  resources grow whether or not a player acts,
  stacks never decay,
  and the `max_turns` tile-count adjudication rewards holding
  whatever the early land grab yielded.

This is analysis, not measurement,
and it has a cheap falsification test:
a scripted turtle-breaker bot
(stage stacks, converge on the thinnest border tile);
if it can beat `greedy`, the argument is wrong.
`tactician` beating `greedy` is not that falsification:
it wins the tile-count adjudication by out-grabbing the neutral land
and never by breaking a garrison,
so its sweep is over-turtling, not an attack out-delivering a garrison —
the leg the test targets still stands.

Each candidate anti-turtling lever attacks one leg:
stack decay or upkeep bounds defense;
letting staged force exceed the per-turn cap,
or attack-efficiency scaling, unbounds offense;
victory by resource share or frontier-based scoring prices passivity.
The planned experiment is a seeded variant tournament:
implement the levers as `Config` knobs,
run the gameplay-health metrics per variant,
and keep the one lever that restores elimination and comebacks —
one, because every surviving constant must earn its place.

Decision (2026-07-14): removed the notion of defense entirely.
A stationary garrison used to fight back for free —
the "auto-defense" that made holding cost the attacker armies —
and carried a 1.25x bonus (`defense_bonus_pct`) on top;
both are gone.
A garrison now strikes for nothing
(to inflict losses an army must be commanded to move),
and the bonus had no meaning once there is no defense to scale.

Whether this loosens the turtle is still open.
The headline matchups are unmoved —
`greedy` still beats `random` about three-quarters by elimination
(avg ~420 turns) and `tactician` still freezes `greedy`
to the turn limit — but that is weak evidence:
`greedy` and `tactician` were shaped under the old rules
and never attack a garrison they cannot overrun in one turn,
so neither tests whether cheaper offense now pays.
The structural reason to expect the freeze to survive is unchanged
(the per-turn pool caps delivery to any one tile,
so a pool-sized garrison still cannot be taken in a turn),
but the clean check is the turtle-breaker bot proposed above,
which has not been run.
The offense cap and free passivity remain the legs to target.

In principle more players weakens the first leg:
the offense cap is per player,
so several neighbours could jointly out-deliver
one defender's per-turn regeneration.
But free-for-all incentives push the other way —
in multi-party combat the winner pays
everyone else's effective strength,
so simultaneous attackers also fight each other
and the abstaining vulture profits,
and tile-count victory invites ganging up on the leader
and kingmaking.
Six-`greedy` runs land on the second reading:
they still never end by elimination (see "Observed behavior" above),
so in practice the extra attackers do not break the turtle —
they turtle their own corners.
That is political emergence,
not the two-player duel depth the ML plan targets,
so player count is deliberately not the anti-turtling fix.
It is a cheap experiment to run
(`--bots` takes 2–6 names, one per corner).

## A future-evaluating bot (`future`)

`future` (implemented in `bot/src/future_tree.rs`, which owns the mechanics)
is the skeleton of a classical game engine —
one-ply search plus a static evaluation —
adapted to a game that breaks the assumptions chess-style search relies on:
turns are simultaneous,
so the opponent is a fixed model rather than a minimising node;
and a turn is a whole *set* of orders whose power set is far too wide
to enumerate,
so the plan is built greedily, one order at a time by evaluated value,
in place of enumerating legal moves.
It is the strongest non-learned bot, sweeping `tactician` from either seat.

The edge over `tactician` is that it optimises the *resolved* position
directly —
its evaluation refuses any order that loses army on net and prices every tile —
so it grabs and holds neutral land at least as efficiently,
never issuing the under-funded overruns the scripted bots throw away.

Why one ply and not deeper — a measured, rules-dependent result.
A genuine multi-turn best-response search was built and tried:
each turn branches over a small candidate-plan menu,
our own continuation is *searched* rather than played out by a weak policy,
and `evaluate` is applied only at the leaf.
Under the earlier rules — a stationary garrison struck back
with a 1.25x defender bonus — it earned its cost:
depth two beat one ply roughly 15/9 and won some games by elimination,
because cracking a bonus-backed garrison needs a multi-turn build-up
that a single ply cannot see.
Removing defense (see "Why turtling dominates") erased that edge:
overrunning a garrison is now a one-turn threshold,
so there is nothing multi-turn left to discover,
and depth two comes out even-to-slightly-behind one ply
at roughly 11x the cost (each added turn re-runs the opponent model
for every candidate). So the shipped bot is one ply.

The lesson underneath is the one classical engines encode:
do not score a leaf by playing on with a weak policy —
an early attempt rolled the extra turns with `TacticianBot` and lost,
since it was really scoring "what if I then play like a weaker bot" —
but search, and score the leaf with a function you trust.
Deeper search pays only once that leaf evaluator is at least as strong
as the search itself: a learned value function, not a scripted playout,
which is the AlphaZero/NNUE direction the next section takes.

## ML plan

### Vertical slice (implemented): a learned bot, end to end, in Rust

The first ML milestone is deliberately narrow and entirely in-process:
a learned `ml` bot that beats `random`,
with no Python, no ONNX, and no external ML dependency.
It exists to prove the observation/action encoding is learnable
and to give the later, heavier work a working reference.
Three pieces, each documented where it lives:

- The RL-facing state/action encoding is `engine/src/encoding.rs`
  (feature planes, the discrete action space,
  and a legality mask derived from `GameState::legal_orders`
  so it can't drift from the rules).
- The policy and its trainer are `bot/src/ml/`:
  a tiny two-layer perceptron over one-hop hex patches (`ml::policy`),
  trained by episodic REINFORCE with discounted reward-to-go
  against a scripted opponent (`ml::train`).
  The reward is the per-turn change in the learner's tile lead;
  reward-to-go is what makes the credit assignment work,
  since a lone terminal reward split across a game's hundreds of
  per-tile decisions barely moves the policy.
- `MlBot` (`ml::bot`) plays a trained checkpoint
  through the same `Bot` trait as the scripted bots,
  so everything that runs them runs it.

**Decision (2026-07): hand-rolled, pure Rust, not a framework.**
The model is a ~900-weight MLP,
so its forward and backward passes are a few dozen lines
and a finite-difference gradient check pins their correctness
(`ml::policy` tests).
Against that, pulling a full framework (candle, burn, `tch`)
into the `bot` crate — which the `cli`, the tests,
and ultimately the WebAssembly/Android `app` all build —
is a heavy dependency on a workspace that otherwise has none,
and `tch` is not even pure Rust.
The engine's zero-dependency, every-target ethos wins at this size.
This is a slice, not the ceiling:
it uses argmax play,
a fixed scripted opponent rather than self-play,
and a local policy that cannot see beyond one hex.

### Scaling up (next)

When the model outgrows an MLP the trade flips, and the path is:

1. A larger policy (CNN/GNN over the full per-tile planes)
   trained by self-play (PPO),
   reusing the `engine::encoding` contract unchanged.
2. A training stack heavy enough to deserve a real framework —
   `candle` (pure Rust, trains and runs) is the natural fit;
   exposing the engine to Python via a `py/` PyO3 crate
   for the mature RL tooling remains an option but is no longer required.
3. In-app inference via `tract` (pure Rust, Android-friendly)
   once the model is too big to keep hand-rolling:
   export the trained policy to ONNX and run it under `tract`,
   keeping the shipped app light while the framework stays on the
   desktop trainer.
   A policy for these map sizes is a few MB,
   inference well under a millisecond.
4. Training checkpoints double as a difficulty ladder.

Engine speed today (release, single thread): ~800 full games/sec on
radius 5 (~70k turns/sec), before any optimization. Comfortable for RL.

## Open questions

- Fog of war? Generals.io-style visibility would add depth and is standard
  for the RL formulation, but complicates the UI and the observation space.
- Should `half` splits be richer (explicit amounts?) — richer play vs
  bigger action space.
- Anti-turtling lever — which one lever from "Why turtling dominates"
  survives the variant tournament.
