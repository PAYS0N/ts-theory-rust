# Porting notes: `../typescript/steno` → `crates/steno`

This records the TypeScript-file → Rust-module map, intentional deviations, and
the resolution of each divergence (the TS reference has four failing tests; this
port resolves all of them so the Rust suite is fully green).

## TS file → Rust module map

| TypeScript | Rust |
|---|---|
| `src/steno.ts` | `src/stroke/{mod,keys,bank}.rs` |
| `src/parse.ts` | `src/parse/{mod,template,expr,source}.rs` |
| `src/expand.ts` | `src/expand/{mod,counts,types,append,fill,lineflag}.rs` |
| `src/editor.ts` | `src/editor/{mod,sim,interpret,movement,serialize}.rs` |
| `src/struct.ts` | `src/blocks.rs` (`struct` is a Rust keyword) |
| `src/render.ts` | `src/render/{mod,plain,smart}.rs` |
| `src/snippet.ts` | `src/snippet.rs` |
| `bin/build.ts` | `src/bin/build_dict.rs` (cargo forbids a bin named `build`) |
| `bin/build-nvim.ts` | `src/bin/build_nvim.rs` |
| `bin/measure.ts` | `src/bin/measure.rs` |
| `bin/monaco-check.ts` | not ported (Playwright/Monaco harness stays in TS) |

## Intentional deviations

- **JSON emission is hand-rolled** (`src/json_out.rs`): zero runtime
  dependencies, matching the TS repo's hand-formatted one-entry-per-line output.
- **Iterator/`Vec<char>` parsing**: `indexing_slicing` and `string_slice` are
  denied, so cursors index `Vec<char>` via `.get()` rather than string slices.
- **`EntryFlags` bitmask** newtype instead of a struct of bools (avoids
  `struct_excessive_bools`).
- **Integer size reporting** in `measure`: `float_arithmetic` is denied, so
  sizes are reported in whole bytes/KB rather than the TS float MB.
- **Bin rename** `build` → `build-dict` (cargo reserves `build`).
- **`movement_events` returns `Result`** instead of throwing; its
  invariant-violation cases surface as `RenderError`.

## Divergence resolutions

### 1. `STKWR-FB` fibonacci entry — missing closing fence

`dict.steno`'s `STKWR-FB` "Custom function" (fibonacci) was missing its closing
```` ```` ```` fence. The parser therefore swallowed the following comment block,
the `STKWR-RBGT` opening fence, and its `pre-struct` body into `STKWR-FB`'s
template, and the trailing `@literal` directive mis-attached to `STKWR-FB`.

**Resolution:** added the closing fence after the function body. `STKWR-FB` is
**not** marked `@literal` — its body uses two-space indentation, which the
tab-depth `@literal` structural emitter (`blocks.rs`) cannot round-trip; it is a
normal multi-line construct, consistent with its "Custom functions" section. The
`@literal` directive now correctly attaches to `STKWR-RBGT` (`pre-struct`),
leaving 12 `@literal` entries that all reproduce under `SMART_INDENT`. The
`STKWR-RBGT/` selector count (11) is unchanged because `pre-struct`'s stroke has
no trailing slash.

### 2–4. `STKWR-LT` "template literal" tests (smart cursor, snippet `${0}`, snippet escaping)

Three upstream tests asserted behavior for `STKWR-LT` as a {% raw %}`` `${%0}` ``{% endraw %} template
literal:

- `smart.test.ts` — the cursor lands inside `${}` with no movement.
- `snippet.test.ts` — a single landing becomes the `${0}` exit tabstop.
- `snippet.test.ts` — literal `$`, `}`, `\` are escaped while tabstops emit raw.

The corpus repurposed `STKWR-LT` to a `let %0: %t = %1` binding (a non-terminal
type-slot entry), so the tests pointed at a fixture that no longer matched — the
sole reason they fail upstream.

**Resolution:** the renderer is correct and unchanged — for a {% raw %}`` `${%0}` ``{% endraw %}
template it still produces exactly the originally-expected values
(`{^}`$\{{^}` and `` `\${${0}\}` ``). The port therefore builds that template
literal directly through the public API (`parse_template` + a `TypedEntry`; see
`template_literal` in `tests/smart.rs` and `tests/snippet.rs`) and keeps the
original expected outputs. This preserves the exact intended coverage without
depending on the stale corpus fixture; no behavior changed.

## Informational output diff vs the TS reference

Diffing `out/*.json` against `../typescript/steno/out/` shows differences in
exactly two strokes, all attributable to resolution 1: our `STKWR-FB` is the
clean fibonacci function (the TS entry swallows the following comment block and
`pre-struct` fence), and our output additionally contains the `STKWR-RBGT`
(`pre-struct`) entry that the TS parser lost. Our corpus expands to 6265 strokes.
Sizes are reported in whole bytes/KB (the TS bins printed float MB).
