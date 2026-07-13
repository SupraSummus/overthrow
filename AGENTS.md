# AGENTS.md

Agent-facing project doc. `CLAUDE.md` in the repo root is a symlink to
this file, so both names resolve to the same content — different tools
look for different filenames. Edit either, you edit both.

## Prose uses semantic line breaks

Write natural-language text — this file, `README.md`, `DESIGN.md`,
`TODO.md`, commit bodies, doc comments — using
[Semantic Line Breaks](https://sembr.org/):
break the source after each sentence,
and after an independent clause where it helps,
instead of wrapping at a fixed column.
The breaks render as spaces in Markdown, so the output is unchanged;
what they buy is diffs that land on the clause that changed
rather than reflowing a whole paragraph.
As a rule of thumb keep lines under 80 characters
and never break inside a hyphenated word —
the link has the full MUST/SHOULD/MAY rules.

Leave prose that predates this convention alone unless you are already
editing it;
when you touch a paragraph, reflow just that paragraph.

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
(`engine/src/game.rs`, `Config::maintenance_pct`); a code comment whose
"why" spans modules points at a `DESIGN.md` section by heading, as the
`game.rs` module doc does. When renaming a doc heading or a named
item, grep the other side for references to it.

Don't paste code or values into docs — formulas, defaults, sample CLI
invocations excepted (`README.md` usage lines are commands to copy,
not a copy of source). Link to the item instead; a pasted copy is
guaranteed to drift.

## Tracking debt you notice in passing

When you spot a rough edge while working on something else — a refactor,
a dead branch, drifted duplication, a missing test — log it in `TODO.md`
at the repo root instead of fixing it now (scope creep) or burying an
inline `// TODO` (invisible outside that file). Glance at `TODO.md`
before starting new work; its header has the convention. Game-design
questions are not debt: they go to `DESIGN.md`, next to their rationale.
