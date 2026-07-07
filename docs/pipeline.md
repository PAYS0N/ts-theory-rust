# Pipeline

How [`dict.steno`](../dict.steno) becomes the four `out/` artifacts. Data flows
strictly forward through a chain of one-way passes; each pass has its own error
type and nothing is swallowed. This document covers the passes, the module
dataflow, the error taxonomy, and the editor model the renderers share.

## The passes

```
dict.steno
   │  parse            (parse/)      source text → Vec<Entry>, each template → Vec<Chunk>
   ▼
 Vec<Entry>
   │  Pass A: counts   (expand/counts.rs)   @count fan-out; resolve %d, %[…%], %(…)
   ▼
 Vec<ExpandedEntry>    (no Repeat / Dcount / Computed chunks remain)
   │  Pass B: types    (expand/append.rs, types.rs)   append type strokes into %t slots
   ▼
 Vec<TypedEntry>
   │  line-flag        (expand/lineflag.rs)   add the one-liner (U) variant
   ▼
 Vec<TypedEntry>  ──┬── render plain  (render/plain.rs)  ─→ out/plain-ts.json
                    ├── render smart  (render/smart.rs)  ─→ out/smart-ts.json
                    └── snippets      (snippet.rs)       ─→ out/vim-snippets.json,
                                                            out/snippets.json
```

`expand_dict` (in [`expand/mod.rs`](../crates/steno/src/expand/mod.rs)) runs
Pass A, Pass B, and the line-flag pass in one call; the three binaries each call
it, then hand the `TypedEntry` list to a renderer.

### parse — `parse/`

Turns source text into a typed AST and nothing more (no expansion, no movement,
no JSON). [`source.rs`](../crates/steno/src/parse/source.rs) walks the file line
by line into `Vec<Entry>`; [`template.rs`](../crates/steno/src/parse/template.rs)
parses each fence body into `Vec<Chunk>`;
[`expr.rs`](../crates/steno/src/parse/expr.rs) parses a `%(…)` expression into
the linear form `a·d + b`. Syntax is specified in
[steno-format.md](steno-format.md).

### Pass A — count expansion — `expand/counts.rs`

Each `@count` entry fans out to one `ExpandedEntry` per count value `0..=max`:
the bank keys for that value are merged into the stroke's last sub-stroke, the
`%[ sep | body %]` repeat is run `count` times, and `%d` / `%(EXPR)` are
resolved. The scope of `d` is the total count outside a repeat and the (0-based)
iteration index inside one. Entries without `@count` pass through unchanged. A
`@count` with no count operator — or a count operator with no `@count` — is an
error. After this pass, no `Repeat`, `Dcount`, or `Computed` chunks survive.

### Pass B — type append — `expand/append.rs`, `types.rs`

`@type` entries are collected into the append set (consumed here, never emitted
on their own). Each `%t` type slot is filled by appending a type stroke, so
`function %0(%1): %t {…}` fans out over the available types. Generic arguments
draw from the arity-0 type pool, optionally restricted by the caller (the
`measure` binary compares pool sizes). `@fuse` merges the shape's last stroke
segment into the first appended type stroke so the type-less intermediate is
never a required stroke. The output `TypedEntry` records whether it is
**terminal** (a complete construct) or a non-terminal append step, and carries
its source `Entry` for flag lookups downstream.

A second, unrelated source of non-terminal-ness is caught only after every
entry is known: a "family root" stroke (e.g. `STKWR-PBGS`, a placeholder with
no `%t` of its own) can be a strict `/`-segment-prefix of other, separately
authored entries later in `dict.steno` (`STKWR-PBGS/TPH-FLT`, ...).
`expand_types_one` sees one entry at a time and has no way to know this, so it
falls through to its one-stroke-leaf branch and marks it terminal. A
whole-batch post-pass, `fix_family_terminals`
(`expand/family.rs`, run in `expand_dict` right after the main Pass A/B loop
and before line-flag), forces `terminal = false` on any entry that is such a
prefix — the same invariant the nvim path already relies on for arity-chain
partials.

### line-flag — `expand/lineflag.rs`

Adds the one-liner variant (the U flag): a copy of each eligible entry in which
every `%b` body-break collapses instead of breaking onto a new line.
`@multiline` entries opt out.

### render — `render/`

Passes C+D+E. Each `TypedEntry` becomes a Plover dictionary value through the
editor model (below): tokenize the template into keystrokes, (smart only) drop
the trailing run of auto-supplied closers, coalesce into events, interpret them
through the profile's editor to find where `%0` lands, append the movement that
walks the cursor there, and serialize. The value is wrapped in Plover's `{^}…{^}`
affixes. Non-terminal (type-append intermediate) strokes are identical in both
profiles: brackets stripped, no newlines, no movement.
[`build_dict`](../crates/steno/src/render/mod.rs) collects the results into an
ordered map and flags any stroke that appears twice with a **different** value as
a collision.

### snippets — `snippet.rs`

The nvim path. An LSP snippet engine owns cursor placement via tabstops, so there
is no movement math and no plain/smart split. Landings are renumbered to tab
order: the lowest becomes `${1}` and ascends, the **highest** becomes `${0}` (the
LSP exit). Non-terminal strokes emit the same bracket-stripped partial as the
plain profile, with no tabstops — and, critically, **no sentinel**: they're typed
as plain `{^}…{^}` text Plover owns outright, exactly like the plain/smart
profiles. A non-terminal is always superseded by a longer chord, and Plover
corrects that by backspacing exactly what it last typed; if the nvim plugin had
already rewritten that text into a snippet body, Plover's backspace count would
no longer match the buffer and the correction would eat whatever precedes it.
Only a terminal — a completed construct, never itself extended by a later
stroke — is safe to hand to the nvim plugin. `build_snippets` emits two maps:
`plover-keys` (stroke → either a plain `{^}…{^}` partial for a non-terminal, or
a `{^}@@…@@{^}` sentinel-wrapped token for a terminal) and `snippets` (terminal
`key_id` → LSP body).

### `@literal` blocks — `blocks.rs`

An `@literal` entry is a complete pre-formatted multi-line block (a data
structure). Under the **smart** profile only, `emit_struct` drives the editor
structurally instead of typing literal tabs and closers: it types opening/content
lines, relies on Enter to block-expand after a line ending in `{`, and walks
`Down`/`End`/`Enter` to drop levels — so the result uses the editor's own indent
unit. The plain profile types such a block literally.

## Module dataflow

Strictly forward, no back-edges (an intent invariant):

```
stroke ◄── parse ◄── expand ◄── { editor, render, snippet, blocks }
              │                          ▲
              └── json_out ──────────────┘   (ordered-map JSON emission)
```

Read the arrows as "depends on": `render`, `snippet`, and `blocks` sit at the
top and depend on `expand`, which depends on `parse` and `stroke`; nothing lower
depends on anything higher. `json_out` is a leaf used only at emission. The
library exposes every stage through [`lib.rs`](../crates/steno/src/lib.rs); only
the three binaries write files.

## The editor model — `editor/`

Rendering is separated from the editor so each editor behavior is an independent,
testable knob. Three functions:

```text
interpret(events, behaviors) -> EditorState    how an editor reacts to keystrokes
movement_events(buffer, from, to) -> Event[]    indent-independent cursor moves
serialize(events) -> String                     Plover dictionary value
```

An `Event` is `Text(String)`, a repeated special `Key` (`Up`/`Down`/`Left`/
`Right`/`Home`/`End`/`BackSpace`/`Tab`/`Enter`), or a `Mark(n)` that records where
landing `%n` fell (types nothing). `interpret` returns the final `buffer`, the
resting cursor offset `rest`, and `target` (where `%0` landed).

A **profile is just a `Behaviors` preset** — four knobs:

| Preset | `auto_close` | `type_over` | `auto_indent` | Role |
|---|---|---|---|---|
| `PLAIN` | off | off | off | A dumb editor: every closer typed, cursor walks back from the document end. |
| `SMART` | on | on | off | Auto-close + type-over + block-expand: emit only what the editor won't supply; interior closers stay via type-over, the trailing run of auto-closers drops. |
| `SMART_INDENT` | on | on | on | `SMART` plus auto-indent: multi-level `@literal` blocks need no literal `\t` and no typed closers. |

`indent_unit` is one indentation level (four spaces, VS Code's default). The tests
assert that `interpret(emit(t, B), B)` reproduces the intended code with the
cursor on `%0`, for each profile `B` — the editor is simulated to *compile*
movement, never executed.

## Error taxonomy — `error.rs`

Every failure surfaces as one of five typed errors; nothing panics and nothing is
swallowed. Each pass owns its type:

| Error | Raised by | Carries |
|---|---|---|
| `StenoError` | parse | message **and the 1-based source line** (and column, in template errors) |
| `StrokeError` | stroke mechanics (parse/merge/count-bank arithmetic) | message |
| `ExpandError` | Pass A / Pass B | message; `From<StrokeError>` re-wraps stroke-arithmetic failures |
| `RenderError` | render pass | message (unresolved chunk reaching a renderer, or an impossible movement) |
| `SnippetError` | snippet renderer | message (unresolved chunk reaching it) |

A caller gets a complete result or an error — never partial silent output — and
output files are written only after the collision check passes.
