#!/usr/bin/env bash
# Assemble the canonical "tool contracts" doc block from each binary's own
# `--use`/`--audience` output, and either write it into the docs
# (`--write`) or verify the committed docs still match (`--check`, used by
# ctx-verify).
#
# The single source of truth is the binary: each agent-audience tool
# (checked via `--audience`) prints its resident one-line dispatch entry
# under `--use`. A contract/use/audience change that doesn't regenerate
# the docs fails the `contracts` battery check — the doc block cannot
# drift from the code, symmetric with how the repo treats every other
# "must". Never hand-edit the block between the markers.
#
# Usage: gen_tool_contracts.sh [--write|--check] [ROOT]
#   --write  (default) rewrite the block in each target doc in place
#   --check  emit `FAIL:` lines (ctx-verify format) if any doc is stale

set -euo pipefail

MODE="--write"
ROOT="."
for arg in "$@"; do
    case "$arg" in
    --write | --check) MODE="$arg" ;;
    *) ROOT="$arg" ;;
    esac
done
cd "$ROOT"

BEGIN='<!-- BEGIN GENERATED tool-contracts (scripts/gen_tool_contracts.sh --write) -->'
END='<!-- END GENERATED tool-contracts -->'

# Docs carrying the block. This is the template/scaffolded-project copy
# of this script, which intentionally diverges from the CTX repo's own
# scripts/gen_tool_contracts.sh here: a cloned project has a single
# CLAUDE.md and no template/ dir, and its README.md documents the
# project itself, not the CTX tooling — it should never carry this
# block, so there is no full-contract block here at all, only the
# agent-audience dispatch lines. Keep this DOCS line as the one
# deliberate difference between the two copies; update-template.sh still
# overwrites everything else in this file unconditionally on every sync.
DOCS=(CLAUDE.md)

BIN_DIR="target/debug"
FAIL=0
BINS=(ctx-context ctx-verify ctx-scan ctx-cage ctx-brief ctx-status)

# Emit a FAIL line (check mode) or die (write mode) with a message.
fail_or_die() {
    if [[ "$MODE" == "--check" ]]; then
        echo "FAIL: gen_tool_contracts.sh: $1" >&2
        FAIL=1
    else
        echo "gen_tool_contracts.sh: $1" >&2
        exit 1
    fi
}

# The six contract-bearing binaries must already be built and present at
# $BIN_DIR. This script never invokes cargo itself — install-tools.sh is
# the sole build/install step (see its own header comment), so this stays
# accurate whether these binaries are local workspace members (the CTX
# repo checking itself) or artifacts copied in from elsewhere (a
# scaffolded project, where they aren't buildable from $ROOT at all).
for bin in "${BINS[@]}"; do
    if [[ ! -x "$BIN_DIR/$bin" ]]; then
        fail_or_die "$BIN_DIR/$bin: not found or not executable — run install-tools.sh (or, in the CTX repo itself, cargo build) first"
    fi
done
if [[ "$FAIL" -ne 0 ]]; then
    exit $FAIL
fi

# Render one `- **name** — <probe output>` line for `$bin`'s `$flag` probe.
one_line() {
    local bin="$1" flag="$2"
    printf -- '- **%s** — %s\n' "$bin" "$("$BIN_DIR/$bin" "$flag")"
}

BLOCK="$BEGIN"$'\n'
for bin in "${BINS[@]}"; do
    audience="$("$BIN_DIR/$bin" --audience)"
    [[ "$audience" == "human" ]] && continue
    BLOCK+="$(one_line "$bin" --use)"$'\n'
done
BLOCK+="$END"

# Extract the committed block (markers inclusive) from a doc, or empty.
committed_block() {
    awk -v b="$BEGIN" -v e="$END" '
        index($0, b) { f = 1 }
        f { print }
        index($0, e) { f = 0 }
    ' "$1"
}

for doc in "${DOCS[@]}"; do
    if [[ ! -f "$doc" ]]; then
        fail_or_die "$doc: missing"
        continue
    fi
    if ! grep -qF "$BEGIN" "$doc" || ! grep -qF "$END" "$doc"; then
        fail_or_die "$doc: tool-contract markers not found"
        continue
    fi
    if [[ "$MODE" == "--check" ]]; then
        if [[ "$(committed_block "$doc")" != "$BLOCK" ]]; then
            echo "FAIL: $doc:1: tool-contracts block is stale — run scripts/gen_tool_contracts.sh --write" >&2
            FAIL=1
        fi
        continue
    fi
    # --write: replace the marked region with the fresh block.
    tmp="$(mktemp)"
    printf '%s\n' "$BLOCK" >"$tmp.block"
    awk -v bf="$tmp.block" -v b="$BEGIN" -v e="$END" '
        BEGIN { while ((getline line < bf) > 0) blk = blk line "\n"; sub(/\n$/, "", blk) }
        index($0, b) { print blk; skip = 1; next }
        index($0, e) { skip = 0; next }
        !skip { print }
    ' "$doc" >"$tmp"
    mv "$tmp" "$doc"
    rm -f "$tmp.block"
done

exit $FAIL
