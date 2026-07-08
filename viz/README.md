# steno stroke viewer

Standalone page for browsing `dict.steno`'s strokes interactively: pick a
category (e.g. "functions" or "if"), see the next possible strokes, and drill
down to a complete outline with its rendered snippet text.

Categories come only from explicit `## label` markers in `dict.steno` — see
that file's own comments for the convention. A stroke that continues into
more strokes (e.g. `STKWR-PBGS` → `STKWR-PBGS/-FLT`) shows its next choices
as chips; a terminal stroke shows the full outline and its rendered text,
with a multi-line/one-liner toggle where both variants exist.

## Use

```sh
cargo run --bin viz-data   # writes out/viz-data.json
python3 -m http.server     # from the repo root — fetch can't read file://
```

Then open `http://localhost:8000/viz/`.
