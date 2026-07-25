#!/usr/bin/env bash
# Check that a built javelin-steno-pico firmware image stops before the flash
# javelin reserves for on-device assets.
#
# javelin-steno-pico/config/main_flash_layout.h hands out fixed flash addresses
# for the config block, button-script byte code, system, word list and
# dictionaries. The lowest of them, STENO_CONFIG_BLOCK_ADDRESS, is the first
# byte the firmware image may not occupy. Nothing enforces that: the linker's
# FLASH region is the whole chip (see javelin-steno-pico.elf.map's "Memory
# Configuration"), and the layout addresses are plain C++ constants with no
# linker ASSERT behind them. So an image that has outgrown its budget links,
# builds, and flashes without a single warning — and then the .uf2 writes
# firmware bytes over the config block and the button script.
#
# That failure is close to silent on-device: StenoConfigBlock has no magic or
# version field (javelin-steno/config_block.h), so pico_bindings.cc reads the
# overwritten bytes as a real config and memcpy's them into the key map, while
# ButtonScriptManager's magic check fails and every button goes dead. The board
# still enumerates over USB, so it looks like a hang rather than a bad build.
# This check is the guard that failure mode never had.
#
# Missing toolchain or an absent build is a SKIP, following cpp_check.sh's
# conventions; an image that overruns is a FAIL. Not wired into ci.sh /
# ctx-verify (there is no firmware ELF in a Rust-only workspace, so it would
# always SKIP) — build_firmware.sh runs it, and it can be run by hand after a
# manual cmake/make (see docs/firmware-build.md).

set -uo pipefail

ROOT="${1:-.}"
cd "$ROOT" || { echo "FAIL: flash_extent_check: cannot cd to '$ROOT'" >&2; exit 1; }
ROOT="$(pwd)"

PICO="$ROOT/javelin-steno-pico"
ELF="$PICO/build/javelin-steno-pico.elf"
LAYOUT="$PICO/config/main_flash_layout.h"

skip() { echo "SKIP: flash_extent_check ($1)"; exit 0; }
fail() { echo "FAIL: flash_extent_check: $1" >&2; exit 1; }

[[ -d "$PICO" ]] || skip "javelin-steno-pico/ not present at $PICO"
[[ -f "$ELF" ]] || skip "no firmware ELF at $ELF; build it first (scripts/build_firmware.sh)"
[[ -f "$LAYOUT" ]] || skip "no flash layout header at $LAYOUT"
command -v arm-none-eabi-objdump >/dev/null 2>&1 ||
    skip "arm-none-eabi-objdump not on PATH"

# The limit is read from the vendored header rather than hard-coded here, so a
# re-sync that moves the layout moves this budget with it.
limit_hex="$(grep -A1 'STENO_CONFIG_BLOCK_ADDRESS' "$LAYOUT" |
    grep -o '0x[0-9a-fA-F]\+' | head -1)"
[[ -n "$limit_hex" ]] ||
    fail "could not read STENO_CONFIG_BLOCK_ADDRESS from $LAYOUT"
limit=$((limit_hex))

# Flash extent = the highest load address any loaded section reaches. objdump
# prints each section as a header line ("Idx Name Size VMA LMA File-off Algn")
# followed by a flags line; only sections whose flags carry LOAD occupy flash,
# which excludes .bss (ALLOC only) and the .debug_* sections. Empty sections are
# dropped too: an empty .tdata is still marked LOAD but reports its RAM VMA as
# its LMA, which would otherwise swamp the maximum.
image_end=0
image_start=-1
while read -r size lma; do
    start=$((16#$lma))
    end=$((start + 16#$size))
    if ((end > image_end)); then image_end=$end; fi
    if ((image_start < 0 || start < image_start)); then image_start=$start; fi
done < <(arm-none-eabi-objdump -h "$ELF" |
    awk '/^ *[0-9]+ / { size = $3; lma = $5; next }
         /LOAD/ { if (size !~ /^0+$/) print size, lma }')

((image_end > 0)) || fail "no loadable sections found in $ELF"

if ((image_end > limit)); then
    printf 'FAIL: flash_extent_check: firmware image overruns the reserved flash layout\n' >&2
    printf '  image      0x%08x .. 0x%08x  (%d bytes)\n' \
        "$image_start" "$image_end" "$((image_end - image_start))" >&2
    printf '  budget ends at STENO_CONFIG_BLOCK_ADDRESS 0x%08x (%d bytes)\n' \
        "$limit" "$((limit - image_start))" >&2
    printf '  overrun    %d bytes\n' "$((image_end - limit))" >&2
    printf '  Flashing this .uf2 destroys the on-device config block and button\n' >&2
    printf '  script. Shrink the image (the generated construct op tables in\n' >&2
    printf '  out/steno_generated_dictionary_data.h are the largest contributor;\n' >&2
    printf '  arm-none-eabi-nm -S --size-sort on the ELF ranks the rest), or move\n' >&2
    printf '  the layout in %s via a tracked\n' "${LAYOUT#"$ROOT"/}" >&2
    printf '  patch — and re-upload the assets afterwards either way.\n' >&2
    exit 1
fi

printf 'OK: flash_extent_check: image ends 0x%08x, %d bytes clear of 0x%08x\n' \
    "$image_end" "$((limit - image_end))" "$limit"
exit 0
