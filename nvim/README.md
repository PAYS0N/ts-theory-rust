# steno-ts (Neovim)

Editor-side expander for the steno TypeScript theory. Plover transmits *which
chord was pressed*; for a terminal chord, the dictionary value carries the LSP
snippet body inline — so the cursor lands and tabs natively, with none of the
`{#Up}{#Left}` movement hacks the Plover-only `plain`/`smart` dictionaries
need.

## How it fits together

```
chord ─► Plover (out/plain.json or out/smart.json)
        types "{^}@@<escaped LSP body>@@{^}" into the editor
                         │
                         ▼
      steno-ts plugin: finds the complete @@…@@ token at the cursor,
        decodes it (INLINE_NEWLINE → "\n"), deletes the token,
        vim.snippet.expand(body)
```

The compiler in this repo produces the dictionary:

```sh
cargo run --bin build-nvim   # writes out/plain.json (or out/smart.json)
```

Load that file as a Plover dictionary — no second artifact is needed for the
plugin. (`out/snippets.json` is also emitted, but it's for debug/inspection
only per ADR-2 in `docs/DECISIONS.md`; the plugin never reads it and it must
not be treated as load-bearing.)

## Install & configure

Requires Neovim **0.10+** (built-in `vim.snippet`). With lazy.nvim:

```lua
{
  dir = "/path/to/steno/nvim",          -- or a published repo
  config = function()
    require("steno-ts").setup({
      -- filetypes = { "typescript", "javascript", ... },  -- default
    })
  end,
}
```

The plugin attaches a `TextChangedI` autocmd to matching buffers; when a
complete `@@…@@` token appears at the cursor it decodes the body, deletes the
token, and expands the snippet.

## Status & the chaining caveat

- **Single / fused strokes work cleanly** — functions, classes, control flow,
  declarations, ternary, the template literal, data structures: each is one
  terminal outline, so Plover types one inline token and the plugin expands
  it.
- **Type-append chains through a *defined* non-terminal head** (e.g.
  `Promise` → `Promise<string>`) are the open edge: the non-terminal expands
  to the "pre-function" partial as plain text, but Plover's retro-delete on
  the next stroke counts the *literal text* it typed, not any expansion — so
  it can leave stray characters. Reconciling Plover's delete/retype with
  editor-side expansion needs validation against a live setup; expect to
  iterate here.

Non-terminal partials are intentionally plain literal text, never
sentinel-wrapped — the caveat above is about the *chaining* runtime, not the
bodies themselves.
