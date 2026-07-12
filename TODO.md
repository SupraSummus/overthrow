# TODO

A running registry of open technical debt — things worth improving but outside the scope of whatever is currently being worked on. Spot a rough edge while working on something else — a sketchy pattern, a dead branch, drifted duplication, a missing test? Log it here instead of fixing it inline (scope creep) or burying a `// TODO` in code (invisible outside that file). Glance at this file before starting new work; it doubles as a map of where the rough edges are.

Registry, not changelog: when an entry is resolved — or turns out to be wrong or outdated — delete it in the same commit. Never strike it through or mark it "done"; git history is the changelog. The file only ever contains open items.

One paragraph per entry, separated by blank lines — no bullets, no numbering, no headings. Adding or removing an entry then yields a clean, minimal diff that doesn't reflow its neighbors. Write each entry concretely enough that someone can pick it up cold, and name a concrete next move — what the fix would actually look like. "Verify someday" is a hope, not a next move.

Belongs here: refactors, dead code, inconsistencies, missing tests, sketchy patterns. Does not: game-design questions, balance levers, and planned features — those are design work and live in `DESIGN.md` (planned work and open questions), next to their rationale. Prefer behavior-preserving noticings; when an entry implies a behavior change, say so, since it will need sign-off.

---

`cli/src/main.rs` handles bad flag values inconsistently: a non-numeric `--games`, `--radius`, or `--seed` panics through `expect` (a raw Rust panic message with a backtrace hint), while every other user error in the same parser — missing value, unknown flag, malformed `--bots` — prints a one-line message and exits with code 2. Next move: replace the three `expect` calls with the `eprintln!` + `exit(2)` shape their neighbors already use, and pin the exit code with a case in `cli/tests/cli.rs` alongside `unknown_arguments_are_rejected`.

The bot-name list lives in three places: `make_bot` in `bot/src/lib.rs` is the actual registry, and `cli/src/main.rs` repeats it twice as prose — the module doc's "Bots: greedy, random." and the unknown-bot error's "(available: greedy, random)". The usage line is likewise written out twice (module doc and the `eprintln!`). Adding a bot updates one match arm and silently strands the strings. Next move: export a name list next to `make_bot` (e.g. `pub const BOT_NAMES: &[&str]`), build the CLI error message from it, and have the doc comments point at `make_bot` instead of enumerating.

The engine supports 2–6 players (`Config::players`) and the surrounding code is already player-count-generic (`SeriesStats::wins` is per-id, `print_map` letters owners from `A`), but the CLI hardcodes two: `--bots` insists on exactly two comma-separated names, `config.players` stays at the default, and the summary line prints only P0 and P1. Next move: parse `--bots` as N names, set `config.players` from the count, and loop the summary over players. Behavior change (new CLI surface), but additive.

`app/` is keyboard-and-mouse only:
every non-map action is a key press —
ending a turn (Enter), recruiting (R), starting a new game (N),
switching mode (H play, B watch bots),
and the spectate pace controls (Space pause, `.` step, `[` / `]` speed) —
with no pointer affordance,
so on the Android target — where miniquad maps touches to mouse clicks —
those actions are unreachable.
Tile selection and moves work by touch, but a turn can't be ended.
Next move: add on-screen buttons
(an "End turn" and "New game" tap target,
a recruit button shown while a tile is selected,
and mode/pace buttons for the spectator)
drawn in the HUD and hit-tested like tiles,
so every action has a pointer path;
keep the key bindings as desktop shortcuts.

`.cargo/config.toml` passes `--allow-undefined` to `wasm-ld` so the
macroquad web build links under Rust 1.96+,
which stopped importing undefined symbols by default.
It is a workaround for miniquad not yet annotating its JS/GL imports with
`#[link(wasm_import_module = ...)]`.
Next move: once a released miniquad (via `macroquad`) carries those
annotations, bump the dependency and delete `.cargo/config.toml` —
the flag masks genuinely missing symbols, so drop it as soon as upstream
makes it unnecessary.

`GreedyBot` in `bot/src/lib.rs` scores an attack whenever
`tile.army >= needed` and then issues `MoveAmount::All`,
but under the per-army command-point pool a move ships only
`min(army, remaining CP)` armies (`GameState::order_cost` / `step`),
so an attack scored as winning can land under-strength and lose —
greedy throws armies and CP at overruns it can't actually fund this turn.
The scoring pass is blind to the pool because `take_budget` allocates CP
only afterwards.
Next move: fold the pool into scoring —
walk candidates in priority order tracking remaining CP,
and only commit an attack whose funded strength (`min(army, remaining)`)
still clears `needed`, deferring or downgrading the rest.
Behavior change (greedy plays differently), so it needs sign-off and a
fresh `bot/tests/health.rs` check.

`Rng::below` in `engine/src/rng.rs` documents "uniform value in `0..bound`" but computes `next_u64() % bound`, which is modulo-biased for bounds that don't divide 2^64 — negligible at the tiny bounds the bot shuffles use, but the doc overpromises. Next move: either soften the doc to say the bias is accepted, or debias (rejection sampling / Lemire); prefer the latter before anything RL-side starts drawing from this RNG, since it would silently skew exploration.
