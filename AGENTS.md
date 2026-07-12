# AGENTS.md

Agent-facing project doc. `CLAUDE.md` in the repo root is a symlink to
this file, so both names resolve to the same content — different tools
look for different filenames. Edit either, you edit both.

## Docs must not repeat what the code already says

A doc paragraph that restates code behavior — a rule's mechanics, a
formula, a constant's default, resolution order — is a second copy that
goes stale silently, and the reader can't tell which copy is
authoritative. `engine/` is the single source of truth for the game
rules; split everything else by ownership:

- **Code (doc comments)** owns "what is implemented and how".
  Mechanics, formulas, and constants live as rustdoc on the items that
  implement them (`Config` documents every rule knob and its default,
  `GameState::step` documents turn resolution and combat), together
  with the local one-or-two-sentence rationale. If you're about to
  write a doc paragraph describing behavior, put it in a doc comment
  instead and have the doc point at the item by name.
- **Docs (`README.md`, `DESIGN.md`)** own what code cannot show:
  design goals, decisions and their rationale — especially departures
  from the original game (`old/README.md`) — what is deliberately
  *not* implemented and why, observed emergent behavior from
  bot-vs-bot runs, planned work, and open questions. The
  "Differences from the original" table in `DESIGN.md` is the
  reference example.

Cross-references go by name: docs name files, types, and functions
(`engine/src/game.rs`, `Config::growth_divisor`); a code comment whose
"why" spans modules points at a `DESIGN.md` section by heading, as the
`game.rs` module doc does. When renaming a doc heading or a named
item, grep the other side for references to it.

Don't paste code or values into docs — formulas, defaults, sample CLI
invocations excepted (`README.md` usage lines are commands to copy,
not a copy of source). Link to the item instead; a pasted copy is
guaranteed to drift.
