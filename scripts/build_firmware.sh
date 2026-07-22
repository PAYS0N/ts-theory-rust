#!/usr/bin/env bash
# Full firmware build: dict.infinite.steno -> javelin-steno-pico.uf2, for
# JAVELIN_BOARD=polyglot.
#
# Runs docs/firmware-build.md's "Build steps" end to end: generates the
# dictionary header, reapplies the two vendored-tree patches (dictionary
# wiring + RESET_COUNT rename), then configures and builds
# javelin-steno-pico. This crosses out of the Rust workspace (CMake, an ARM
# cross toolchain, two vendored C++ trees) so it is not exercised by
# ctx-verify / scripts/ci.sh — see docs/firmware-build.md for why.
#
# Prerequisites (see docs/firmware-build.md's "Prerequisites" list; all
# missing/unmet cases FAIL here, since this script's job is the actual
# build, not a soft check):
#   - Arm GNU Toolchain >= 12 on PATH (validated against 14.2.1)
#   - CMake >= 4 on PATH
#   - PICO_SDK_PATH pointing at a cloned pico-sdk
#   - javelin-steno-pico/ cloned, with javelin-steno-pico/javelin symlinked
#     to a cloned javelin-steno
#   - picotool: by default CMake fetches it over the network at configure
#     time; on a machine without that access, set
#     BUILD_FIRMWARE_LOCAL_PICOTOOL to a local picotool clone (and
#     optionally BUILD_FIRMWARE_LOCAL_PICOTOOL_BRANCH to a tag matching
#     picotool_VERSION_REQUIRED) to build offline.

set -uo pipefail

ROOT="${1:-.}"
cd "$ROOT" || { echo "FAIL: build_firmware: cannot cd to '$ROOT'" >&2; exit 1; }
ROOT="$(pwd)"

PICO_REPO="$ROOT/javelin-steno-pico"
BOARD="polyglot"

fail() { echo "FAIL: build_firmware: $1" >&2; exit 1; }

# --- Step 1: generate the dictionary header --------------------------------
echo "==> generating out/steno_generated_dictionary_data.h (cargo run --bin build-javelin)"
( cd "$ROOT" && cargo run -q --bin build-javelin ) || fail "build-javelin failed"

# --- Step 2: wire the generated dictionary into javelin-steno-pico ---------
echo "==> applying javelin-ext patch"
bash "$ROOT/scripts/apply_javelin_ext_patch.sh" "$ROOT" || fail "apply_javelin_ext_patch.sh failed"

# --- Step 3: RESET_COUNT rename workaround ----------------------------------
echo "==> applying javelin-steno split patch"
bash "$ROOT/scripts/apply_javelin_steno_split_patch.sh" "$ROOT" || fail "apply_javelin_steno_split_patch.sh failed"

# --- Step 4: prerequisite checks --------------------------------------------
command -v arm-none-eabi-gcc >/dev/null 2>&1 || fail "arm-none-eabi-gcc not on PATH (Arm GNU Toolchain >= 12 required)"
command -v cmake >/dev/null 2>&1 || fail "cmake not on PATH (CMake >= 4 required)"
[[ -n "${PICO_SDK_PATH:-}" ]] || fail "PICO_SDK_PATH not set; clone pico-sdk and export PICO_SDK_PATH"
[[ -d "${PICO_SDK_PATH}" ]] || fail "PICO_SDK_PATH ('${PICO_SDK_PATH}') is not a directory"
[[ -d "$PICO_REPO" ]] || fail "$PICO_REPO not present; clone jthlim/javelin-steno-pico there first (see README.md)"
[[ -e "$PICO_REPO/javelin" ]] || fail "$PICO_REPO/javelin symlink missing; symlink it to a cloned javelin-steno"

# --- Step 5: configure and build --------------------------------------------
BUILD_DIR="$PICO_REPO/build"
mkdir -p "$BUILD_DIR"

CMAKE_ARGS=(-D "JAVELIN_BOARD=${BOARD}")
if [[ -n "${BUILD_FIRMWARE_LOCAL_PICOTOOL:-}" ]]; then
    CMAKE_ARGS+=(-D "PICOTOOL_GIT_REPOSITORY_URL=${BUILD_FIRMWARE_LOCAL_PICOTOOL}")
    if [[ -n "${BUILD_FIRMWARE_LOCAL_PICOTOOL_BRANCH:-}" ]]; then
        CMAKE_ARGS+=(-D "PICOTOOL_GIT_BRANCH=${BUILD_FIRMWARE_LOCAL_PICOTOOL_BRANCH}")
    fi
fi

echo "==> cmake .. ${CMAKE_ARGS[*]}"
( cd "$BUILD_DIR" && cmake .. "${CMAKE_ARGS[@]}" ) || fail "cmake configure failed"

echo "==> make"
( cd "$BUILD_DIR" && make ) || fail "make failed"

UF2="$BUILD_DIR/javelin-steno-pico.uf2"
[[ -f "$UF2" ]] || fail "build finished but $UF2 was not produced"

echo "OK: build_firmware: $UF2"
exit 0
