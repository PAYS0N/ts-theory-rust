# steno

A compiler from steno-theory source ([`dict.steno`](dict.steno)) to the
dictionaries and snippet tables that let you write TypeScript/JavaScript at
chord speed. It is a zero-dependency Rust implementation. `dict.steno` here is the source of truth.

`dict.steno` is a fenced DSL that maps [Plover](https://www.openstenoproject.org/plover/)
strokes to code templates (`function %0(%1): %t {%b%2}` and the like). The
compiler expands those templates over count and type variants, simulates how an
editor reacts to the resulting keystrokes, and emits four JSON artifacts.

## Artifacts and their consumers

All four are written under `out/`. Nothing else in the pipeline writes files.

| Artifact | Built by | Consumed by | What it is |
|---|---|---|---|
| `out/plain-ts.json` | `build-dict` | Plover, on a dumb editor | stroke → Plover keystroke value; every closer typed, cursor walks back from the document end |
| `out/smart-ts.json` | `build-dict` | Plover, on an auto-closing editor | stroke → Plover keystroke value; only the keystrokes the editor won't auto-supply, movement computed against the resulting buffer |
| `out/vim-snippets.json` | `build-nvim` | Plover | stroke → a sentinel-wrapped token (`@@…@@`); Plover stays a dumb lookup table |
| `out/snippets.json` | `build-nvim` | the [`nvim/`](nvim/) plugin | token → LSP snippet body with tabstops; the editor owns cursor placement |

`plain-ts.json` and `smart-ts.json` are two profiles of the *same* mechanism (see
[docs/pipeline.md](docs/pipeline.md)); both load together on the device, so the
real size budget is their sum. The nvim path exists because an LSP snippet engine
places the cursor via native tabstops, sidestepping the `{#Up}{#Left}` movement
math the Plover-only dictionaries need.

## Build

Each binary reads `dict.steno` from the workspace root and writes only under `out/`.

```sh
cargo run --bin build-dict    # out/plain-ts.json, out/smart-ts.json
cargo run --bin build-nvim    # out/vim-snippets.json, out/snippets.json
cargo run --bin measure       # size matrix (generic-arg pool × param counts); writes nothing
```

A duplicate stroke that resolves to a *different* value is a collision: the
binary prints `ERROR (...): N stroke collision(s)`, exits non-zero, and writes
no file. The current corpus expands to 6265 strokes with no collisions.

## Checks

This repo carries a context/verification toolchain (`ctx-*` binaries under
`target/debug/`); see [CLAUDE.md](CLAUDE.md). The whole-workspace gate is:

```sh
bash scripts/ci.sh    # fmt, clippy, doc, deny, machete, rationale, cycle, no-allow
```

## Layout

```
dict.steno              the source of truth (fenced DSL) — see docs/steno-format.md
out/                    generated artifacts (the four JSON files above)
nvim/                   the Neovim plugin that consumes the snippet artifacts
docs/
  steno-format.md       normative .steno syntax reference
  pipeline.md           passes, module dataflow, error taxonomy, editor model
  porting-notes.md      TS → Rust file map and divergence resolutions
crates/steno/
  src/
    parse/              source & template → Entry AST
    expand/             count fan-out, type-append, one-liner line-flag
    stroke/             steno key mechanics + count-bank arithmetic
    editor/             editor model (interpret / movement / serialize)
    render/             plain & smart Plover dictionaries
    blocks.rs           @literal structural block emitter
    snippet.rs          LSP snippet artifacts
    json_out.rs         hand-rolled ordered-map JSON (no deps)
    bin/                build_dict, build_nvim, measure
  tests/                per-module integration tests over dict.steno
```

## Invariants

- Zero runtime dependencies; JSON is hand-rolled. No async, no `unsafe`, no network.
- One-way pass dataflow (`stroke ← parse ← expand ← {editor, render, snippet, blocks}`);
  no back-edges.
- Errors are typed and propagated, never swallowed; a caller gets a complete
  result or an error, never partial silent output.
- Output files are written only after collision checks pass.
- The editor is *simulated* to compile movement, never executed — this is not a
  Plover plugin or an editor. The Monaco/Playwright verification harness stays in
  the TypeScript reference repo.
