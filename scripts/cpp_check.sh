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

# --- Stack budget: large working buffers must live in static storage, and no
# generated function may recurse ---------------------------------------------
# The generated dictionary's Lookup runs deep in the firmware call stack on a
# device with only a few KB of it (PICO_STACK_SIZE defaults to 2KB and is
# unmodified for this board). An 8 KB arena or editor-sim buffer declared as an
# automatic local overflows that stack and the device emits nothing at all (a
# hard fault, not a wrong lookup). The differential above cannot catch this —
# it runs with a multi-megabyte host stack that hides the overflow — so guard
# the per-function frame size directly, cross-compiled for the real target: a
# host build's frame layout is a different ABI/register file and does not
# reflect the firmware's actual stack usage. Uses arm-none-eabi-g++
# specifically (the same cross toolchain build_firmware.sh requires): clang's
# -fstack-usage reports 0 for large frames and cannot be trusted, and a plain
# g++ build targets the host, not the board's Cortex-M0+. Absent toolchain =>
# the guard is skipped, not failed, matching this script's tolerance of a
# missing toolchain.
#
# A flat per-frame budget cannot see a second failure mode: a self-recursive
# function's stack cost multiplies with input depth, so a frame that looks
# small in isolation can still blow the stack many calls deep. Any function
# whose own disassembly calls back to its own entry point fails outright,
# regardless of its frame size — such logic must be rewritten as an explicit
# iterative walk over static storage (see ConsumeType in
# javelin-ext/steno_generated_types.h for the pattern).
STACK_BUDGET=2048
stack_note=", stack-budget SKIP (no arm-none-eabi-g++)"
if command -v arm-none-eabi-g++ >/dev/null 2>&1; then
    stack_obj="$BUILD_DIR/stack.o"
    stack_log="$BUILD_DIR/stack.log"
    if ! arm-none-eabi-g++ -std=c++20 -mcpu=cortex-m0plus -mthumb \
            -DJAVELIN_BOARD_CONFIG='<stddef.h>' -O2 -fstack-usage \
            -fno-exceptions -fno-rtti -c \
            -I "$JS" -I "$OUT" -I "$EXT" \
            "$EXT/steno_generated_dictionary.cc" -o "$stack_obj" >"$stack_log" 2>&1; then
        echo "FAIL: cpp_check: arm-none-eabi-g++ -fstack-usage build failed" >&2
        sed 's/^/  /' "$stack_log" >&2
        exit 1
    fi
    su_file="${stack_obj%.o}.su" # g++ writes <object-basename>.su beside the object
    [[ -f "$su_file" ]] || fail "expected stack-usage file $su_file not produced"
    max_frame="$(awk -F'\t' 'BEGIN{m=0} {if($2+0>m)m=$2+0} END{print m}' "$su_file")"
    worst="$(awk -F'\t' 'BEGIN{m=-1} {if($2+0>m){m=$2+0;f=$1}} END{print f}' "$su_file")"
    if (( max_frame > STACK_BUDGET )); then
        echo "FAIL: cpp_check: stack frame ${max_frame}B exceeds budget ${STACK_BUDGET}B" >&2
        echo "  worst: $worst" >&2
        echo "  a large working buffer regressed onto the stack; give it static" >&2
        echo "  storage instead (see Arena in javelin-ext/steno_generated_types.h)" >&2
        exit 1
    fi

    recursion_awk="$BUILD_DIR/recursion.awk"
    cat >"$recursion_awk" <<'AWK'
BEGIN { FS = "\t" }
NF == 1 {
    # Function label lines ("00000560 <sym>:") carry no tabs; instruction
    # lines ("  612:\tbytes \tmnemonic\toperand") always do.
    if ($0 ~ /^[0-9a-f]+ <.*>:$/) {
        s = index($0, "<"); e = index($0, ">")
        cur = substr($0, s + 1, e - s - 1)
    }
    next
}
$3 == "bl" || $3 == "blx" {
    # A genuine self-recursive call targets the callee's exact entry point,
    # so objdump prints a bare "<sym>" with no "+0xNN" offset. A "<sym+0xNN>"
    # target is a call into the middle of another, unlabeled local function
    # that objdump attributed to the nearest preceding symbol, not a call to
    # "sym" itself, and must not be flagged.
    if (cur != "" && $4 ~ ("^[0-9a-f]+ <" cur ">$")) {
        print cur
    }
}
AWK
    recursion_log="$BUILD_DIR/recursion.log"
    arm-none-eabi-objdump -d "$stack_obj" 2>/dev/null \
        | awk -f "$recursion_awk" | sort -u >"$recursion_log"
    if [[ -s "$recursion_log" ]]; then
        echo "FAIL: cpp_check: self-recursive function(s) on the firmware stack:" >&2
        sed 's/^/  /' "$recursion_log" >&2
        echo "  recursion depth multiplies stack usage in a way a flat frame" >&2
        echo "  budget cannot bound; convert to an explicit iterative walk over" >&2
        echo "  static storage instead (see ConsumeType in" >&2
        echo "  javelin-ext/steno_generated_types.h)" >&2
        exit 1
    fi

    stack_note=", max stack frame ${max_frame}B <= ${STACK_BUDGET}B, no recursion"
fi

echo "OK: cpp_check ($CXX): $(cat "$run_log")$stack_note"
exit 0
