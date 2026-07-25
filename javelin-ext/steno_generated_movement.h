//---------------------------------------------------------------------------
//
// Movement math for the terminal plain/smart profiles: converting a
// (rest cursor, landing0 target) pair over the simulated buffer into a
// `{#Up Up End Left ...}` nav-key group. Transcribed from the Rust
// reference's cursor-movement model.
//
//---------------------------------------------------------------------------

#pragma once
#include "steno_generated_types.h"

namespace {

constexpr uint32_t MAX_MOVEMENT_KEYS = 256;

struct Pos {
  uint32_t line;
  uint32_t col;
};

// Line = count of '\n' before `offset`; col = offset since the last one.
Pos ComputePos(const char *buffer, uint32_t length, uint32_t offset) {
  if (offset > length) {
    offset = length;
  }
  uint32_t line = 0;
  uint32_t lineStart = 0;
  for (uint32_t i = 0; i < offset; ++i) {
    if (buffer[i] == '\n') {
      ++line;
      lineStart = i + 1;
    }
  }
  return {line, offset - lineStart};
}

// The character length of line `line` in `buffer` (excluding its newline).
uint32_t LineLength(const char *buffer, uint32_t length, uint32_t line) {
  uint32_t start = 0;
  if (line > 0) {
    uint32_t seen = 0;
    for (uint32_t i = 0; i < length; ++i) {
      if (buffer[i] == '\n') {
        ++seen;
        if (seen == line) {
          start = i + 1;
          break;
        }
      }
    }
  }
  uint32_t end = start;
  while (end < length && buffer[end] != '\n') {
    ++end;
  }
  return end - start;
}

// Push `n` copies of `key` into `keys`, bounded by MAX_MOVEMENT_KEYS.
void PushKeys(const char *key, uint32_t n, const char **keys,
             uint32_t &keyCount) {
  for (uint32_t i = 0; i < n && keyCount < MAX_MOVEMENT_KEYS; ++i) {
    keys[keyCount++] = key;
  }
}

// Same line -> Left x delta; cross line -> Up x delta, End, Left x delta (End
// is always emitted on the cross-line branch; a zero-count key is omitted).
void CollectMovementKeys(const char *buffer, uint32_t bufLen, Pos f, Pos t,
                         const char **keys, uint32_t &keyCount) {
  keyCount = 0;
  if (f.line == t.line) {
    PushKeys("Left", f.col > t.col ? f.col - t.col : 0, keys, keyCount);
    return;
  }
  PushKeys("Up", f.line - t.line, keys, keyCount);
  if (keyCount < MAX_MOVEMENT_KEYS) {
    keys[keyCount++] = "End";
  }
  const uint32_t lineLen = LineLength(buffer, bufLen, t.line);
  PushKeys("Left", lineLen > t.col ? lineLen - t.col : 0, keys, keyCount);
}

// Movement from `rest` to `target` over `buffer`. Appends nothing if there is
// no movement.
void AppendMovementGroup(Arena &arena, const char *buffer, uint32_t bufLen,
                         uint32_t rest, uint32_t target) {
  const Pos f = ComputePos(buffer, bufLen, rest);
  const Pos t = ComputePos(buffer, bufLen, target);

  const char *keys[MAX_MOVEMENT_KEYS];
  uint32_t keyCount;
  CollectMovementKeys(buffer, bufLen, f, t, keys, keyCount);
  if (keyCount == 0) {
    return;
  }

  arena.Push("{#");
  for (uint32_t i = 0; i < keyCount; ++i) {
    if (i > 0) {
      arena.PushChar(' ');
    }
    arena.Push(keys[i]);
  }
  arena.Push("}");
}

} // namespace
