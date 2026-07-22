#!/usr/bin/env bash
# Build the javelin-ext replay dictionary against javelin-steno and run its
# differential test (of-javelin brief, criterion 8): every golden stroke
# sequence emitted by build-javelin is replayed through StenoGeneratedDictionary
# and checked against the Rust reference walker's recorded verdict.
#
# Missing toolchain is a SKIP, not a FAIL — deliberately, and unlike
# cycle_check.sh / machete_check.sh. Those enforce a policy that has no other
# guardian, so a silent skip would make ctx-verify's pass misleading. This check
# is a *cross-language differential*: the walker's logic is already pinned by the
# Rust unit/integration tests (crates/steno/tests/infinite.rs). A host C++
# compiler or the upstream javelin-steno tree may be absent in a given
# environment; when they are, the Rust side still fully covers correctness, so
# SKIP is honest rather than a coverage gap.
#
# The generated headers in out/ are a build product (brief D4, gitignored). This
# script does NOT run cargo (it is invoked from within `cargo test`, where a
# nested cargo would deadlock on the target lock); the caller must have written
# the headers first (crates/steno/tests/cpp_check.rs does, via the same emit
# functions build-javelin uses).

set -uo pipefail

ROOT="${1:-.}"
cd "$ROOT" || { echo "FAIL: cpp_check: cannot cd to $ROOT" >&2; exit 1; }
ROOT="$(pwd)"

JS="$ROOT/javelin-steno"
EXT="$ROOT/javelin-ext"
OUT="$ROOT/out"
DATA_HEADER="$OUT/steno_generated_dictionary_data.h"
TEST_HEADER="$OUT/steno_generated_testdata.h"

skip() { echo "SKIP: cpp_check ($1)"; exit 0; }
fail() { echo "FAIL: cpp_check: $1" >&2; exit 1; }

# --- Toolchain / source availability (absence => SKIP) --------------------
CXX=""
for candidate in clang++ g++; do
    if command -v "$candidate" >/dev/null 2>&1; then
        CXX="$candidate"
        break
    fi
done
[[ -n "$CXX" ]] || skip "no C++ compiler (clang++/g++) on PATH"
[[ -d "$JS" ]] || skip "upstream javelin-steno tree not present"

# --- Generated headers must exist (absence => FAIL: caller's contract) -----
[[ -f "$DATA_HEADER" ]] || fail "missing $DATA_HEADER; run build-javelin first"
[[ -f "$TEST_HEADER" ]] || fail "missing $TEST_HEADER; run build-javelin first"

# --- Anti-cheat: the unbounded nesting dimension must not be enumerated -----
# The data header carries rules (e.g. the type "Array<%t>", a marker), never
# instantiated nested answers ("Array<Array<number>>"). Such a string appearing
# here would mean nesting was pre-expanded — exactly what the walk exists to
# avoid. Those answers live only in the golden test header.
if grep -q 'Array<Array' "$DATA_HEADER"; then
    fail "data header contains enumerated nested answers (anti-cheat: nesting must be walked, not enumerated)"
fi

# --- Compile the dictionary + differential harness, link, and run ----------
BUILD_DIR="$(mktemp -d)"
trap 'rm -rf "$BUILD_DIR"' EXIT
BIN="$BUILD_DIR/gentest"

# Full javelin-steno does not host-compile; link a curated minimal set that the
# dictionary actually needs (stroke/str/dictionary base + their deps) and let
# --gc-sections drop the Console-bound remainder.
LINK_SET=(
    "$JS/stroke.cc"
    "$JS/str.cc"
    "$JS/dictionary/dictionary.cc"
    "$JS/crc32.cc"
    "$JS/container/list.cc"
)
for f in "${LINK_SET[@]}"; do
    [[ -f "$f" ]] || skip "upstream source $f not present"
done

build_log="$BUILD_DIR/build.log"
# JAVELIN_EXT_RUN_TESTS (not javelin's own RUN_TESTS, which would compile that
# project's in-source UnitTest blocks) activates our differential main().
if ! "$CXX" -std=c++20 -DJAVELIN_BOARD_CONFIG='<stddef.h>' -DJAVELIN_EXT_RUN_TESTS=1 \
        -ffunction-sections -fdata-sections -Wl,--gc-sections \
        -I "$JS" -I "$OUT" -I "$EXT" \
        "$EXT/steno_generated_dictionary.cc" "$EXT/test_main.cc" \
        "${LINK_SET[@]}" \
        -o "$BIN" >"$build_log" 2>&1; then
    echo "FAIL: cpp_check: $CXX build failed" >&2
    sed 's/^/  /' "$build_log" >&2
    exit 1
fi

run_log="$BUILD_DIR/run.log"
if ! "$BIN" >"$run_log" 2>&1; then
    echo "FAIL: cpp_check: differential test failed" >&2
    sed 's/^/  /' "$run_log" >&2
    exit 1
fi

echo "OK: cpp_check ($CXX): $(cat "$run_log")"
exit 0
