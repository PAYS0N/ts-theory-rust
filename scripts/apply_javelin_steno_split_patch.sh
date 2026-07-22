#!/usr/bin/env bash
# Apply scripts/javelin_steno_split.patch and scripts/javelin_steno_pico_split.patch
# to the vendored javelin-steno and javelin-steno-pico checkouts, renaming
# SplitMetricId's RESET_COUNT enumerator to SPLIT_RESET_COUNT.
#
# Unqualified, split.h declares `enum SplitMetricId { RESET_COUNT, ... }` at
# global scope. pico-sdk's hardware/structs/resets.h does the same via
# `typedef enum reset_num_rp2040 { ... RESET_COUNT } reset_num_t;`. Any
# translation unit that transitively includes both (pico_bindings.cc,
# ssd1306.cc, st7789.cc) fails to compile with a duplicate global identifier.
# See docs/firmware-build.md's "Known blocker" section.
#
# Both javelin-steno/ and javelin-steno-pico/ are vendored and gitignored
# (re-synced from jthlim/javelin-steno and jthlim/javelin-steno-pico), so
# this rename can't live as a permanent in-place change without being
# silently lost on the next sync — this script is the reapply step, run
# after every fresh clone or re-sync of either tree.
#
# Idempotent: SKIPs (not FAILs) if a tree is already patched, so it's safe
# to run unconditionally as part of a build script. Follows cpp_check.sh's
# SKIP/FAIL conventions: missing prerequisites are a SKIP, a patch that no
# longer applies cleanly is a FAIL with guidance to regenerate it.

set -uo pipefail

ROOT="${1:-.}"
cd "$ROOT" || { echo "FAIL: apply_javelin_steno_split_patch: cannot cd to '$ROOT'" >&2; exit 1; }
ROOT="$(pwd)"

MARKER="SPLIT_RESET_COUNT"

skip() { echo "SKIP: apply_javelin_steno_split_patch ($1)"; }
fail() { echo "FAIL: apply_javelin_steno_split_patch: $1" >&2; exit 1; }

if ! command -v git >/dev/null 2>&1; then
    fail "git not on PATH; required to apply patches"
fi

check_log="$(mktemp)"
trap 'rm -f "$check_log"' EXIT

# apply_one <label> <tree-dir> <patch-file> <marker-file>
apply_one() {
    local label="$1" tree="$2" patch="$3" marker_file="$4"

    [[ -d "$tree" ]] || { skip "$label: $tree not present; clone the upstream repo there first (see README.md)"; return 0; }
    [[ -f "$patch" ]] || fail "$patch missing; this is a tracked file and should not be absent"

    if grep -q "$MARKER" "$marker_file" 2>/dev/null; then
        skip "$label: already applied ($MARKER found in $marker_file)"
        return 0
    fi

    if ! git -C "$tree" apply --check "$patch" >"$check_log" 2>&1; then
        echo "FAIL: apply_javelin_steno_split_patch: $patch does not apply cleanly to $tree" >&2
        echo "  This usually means $tree was re-synced from upstream and the surrounding" >&2
        echo "  lines shifted. Re-diff:" >&2
        echo "    1. Re-apply the rename by hand (RESET_COUNT -> SPLIT_RESET_COUNT in the" >&2
        echo "       SplitMetricId enum and its one use site; see docs/firmware-build.md)." >&2
        echo "    2. git -C $tree diff > $patch" >&2
        sed 's/^/  /' "$check_log" >&2
        exit 1
    fi

    if ! git -C "$tree" apply "$patch" >"$check_log" 2>&1; then
        echo "FAIL: apply_javelin_steno_split_patch: $patch passed --check but failed to apply" >&2
        sed 's/^/  /' "$check_log" >&2
        exit 1
    fi

    echo "OK: apply_javelin_steno_split_patch: applied $patch to $tree"
}

apply_one "javelin-steno" \
    "$ROOT/javelin-steno" \
    "$ROOT/scripts/javelin_steno_split.patch" \
    "$ROOT/javelin-steno/split/split.h"

apply_one "javelin-steno-pico" \
    "$ROOT/javelin-steno-pico" \
    "$ROOT/scripts/javelin_steno_pico_split.patch" \
    "$ROOT/javelin-steno-pico/pico_split.cc"

exit 0
