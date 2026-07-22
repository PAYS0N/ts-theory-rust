#!/usr/bin/env bash
# Apply scripts/javelin_ext_pico.patch to the vendored javelin-steno-pico
# checkout, wiring StenoGeneratedDictionary (javelin-ext/) into the real
# firmware dictionary list built in pico_bindings.cc's InitJavelinMaster().
#
# javelin-steno-pico/ is vendored and gitignored (re-synced from
# jthlim/javelin-steno-pico), so the two edits captured in the patch can't
# live as permanent in-place changes without being silently lost on the next
# sync — this script is the reapply step, run after every fresh clone or
# re-sync of javelin-steno-pico/.
#
# Idempotent: SKIPs (not FAILs) if the tree is already patched, so it's safe
# to run unconditionally as part of a build script. Follows cpp_check.sh's
# SKIP/FAIL conventions: missing prerequisites are a SKIP, a patch that no
# longer applies cleanly is a FAIL with guidance to regenerate it.

set -uo pipefail

ROOT="${1:-.}"
cd "$ROOT" || { echo "FAIL: apply_javelin_ext_patch: cannot cd to '$ROOT'" >&2; exit 1; }
ROOT="$(pwd)"

PICO="$ROOT/javelin-steno-pico"
PATCH="$ROOT/scripts/javelin_ext_pico.patch"
MARKER="steno_generated_dictionary"

skip() { echo "SKIP: apply_javelin_ext_patch ($1)"; exit 0; }
fail() { echo "FAIL: apply_javelin_ext_patch: $1" >&2; exit 1; }

[[ -d "$PICO" ]] || skip "javelin-steno-pico/ not present at $PICO; clone jthlim/javelin-steno-pico there first (see README.md)"
[[ -f "$PATCH" ]] || fail "$PATCH missing; this is a tracked file and should not be absent"

if grep -q "$MARKER" "$PICO/CMakeLists.txt" 2>/dev/null; then
    skip "already applied ($MARKER found in $PICO/CMakeLists.txt)"
fi

if ! command -v git >/dev/null 2>&1; then
    fail "git not on PATH; required to apply $PATCH"
fi

check_log="$(mktemp)"
trap 'rm -f "$check_log"' EXIT

if ! git -C "$PICO" apply --check "$PATCH" >"$check_log" 2>&1; then
    echo "FAIL: apply_javelin_ext_patch: $PATCH does not apply cleanly to $PICO" >&2
    echo "  This usually means javelin-steno-pico/ was re-synced from upstream and the" >&2
    echo "  surrounding lines in CMakeLists.txt / pico_bindings.cc shifted. Re-diff:" >&2
    echo "    1. Re-apply the two edits by hand (see scripts/javelin_ext_pico.patch for" >&2
    echo "       the intended hunks, or the README.md 'Firmware' section for context)." >&2
    echo "    2. git -C javelin-steno-pico diff CMakeLists.txt pico_bindings.cc > $PATCH" >&2
    sed 's/^/  /' "$check_log" >&2
    exit 1
fi

if ! git -C "$PICO" apply "$PATCH" >"$check_log" 2>&1; then
    echo "FAIL: apply_javelin_ext_patch: $PATCH passed --check but failed to apply" >&2
    sed 's/^/  /' "$check_log" >&2
    exit 1
fi

echo "OK: apply_javelin_ext_patch: applied $PATCH to $PICO"
exit 0
