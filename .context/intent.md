---
intent_version: 1
---
# Project Intent

This project compiles steno-theory source (`dict.steno`, a fenced DSL mapping
Plover strokes to TypeScript/JavaScript code templates) into four dictionary
artifacts: `out/plain.json` and `out/smart.json` (Plover dictionaries for a
plain and an auto-closing editor), and `out/plover-keys.json` plus
`out/snippets.json` (stroke→token and token→LSP-snippet tables consumed by the
bundled Neovim plugin). It is a Rust reimplementation of the TypeScript
compiler at `../typescript/steno`; `dict.steno` here is the source of truth.

## Public surface

- The three binaries (`build`, `build-nvim`, `measure`) are the only entry
  points that write files, and they write only under `out/`.
- Plover (on-device) consumes the generated dictionaries; the `nvim/` Lua
  plugin consumes the snippet artifacts. Nothing consumes the library crate
  from outside this workspace.

## Non-goals

- No runtime dependencies, ever. JSON emission is hand-rolled.
- No network access, no async runtime, no unsafe code.
- Not a Plover plugin and not an editor: editor behavior (auto-close,
  auto-indent, block-expand) is *simulated* to compile movement, never
  executed. The Monaco/Playwright verification harness stays in the
  TypeScript reference repo.

## Architectural properties that must hold

- One-way pass dataflow: stroke ← parse ← expand ← {editor, render, snippet,
  blocks}; no back-edges between modules.
- Errors are typed and propagated, never swallowed; callers get a complete
  result or an error, never partial silent output.
- Output files are written only after collision checks pass.
- `dict.steno` is the single source of truth for stroke definitions and the
  integration-test fixture; tests read it from disk and never mutate sources.
