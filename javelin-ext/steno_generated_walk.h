//---------------------------------------------------------------------------
//
// The walker (criterion 3, ported): match a construct's base, recover a
// fused return type if any, then replay the construct's type obligations
// against the residual strokes. Unchanged logic from the pre-three-profile
// walker; only its output changed — it now returns the matched construct
// plus filled slot texts (`WalkOut`) instead of a single rendered string, so
// the profile renderers (steno_generated_render*.h) can run on top of it.
//
//---------------------------------------------------------------------------

#pragma once
#include "steno_generated_types.h"

namespace {

using steno_generated::CONSTRUCT_COUNT;
using steno_generated::CONSTRUCTS;
using steno_generated::GenConstruct;

// Copy rest[tsFrom..restLength) into ts[tsFrom..restLength); the caller has
// already placed any leading recovered strokes (e.g. a fused residual type).
void CopyResidual(const uint32_t *rest, uint32_t restLength, uint32_t tsFrom,
                  uint32_t *ts) {
  for (uint32_t i = tsFrom; i < restLength; ++i) {
    ts[i] = rest[i];
  }
}

// Fuse inversion: the return type was merged into the shape stroke. Recover
// it by subtracting the shape's keys; it must be a subset and a valid type.
bool MatchFusedShape(const GenConstruct &c, const uint32_t *rest,
                     uint32_t restLength, uint32_t *ts, uint32_t &tsLength,
                     uint32_t &score) {
  if (restLength < 1 || (rest[0] & c.shape) != c.shape) {
    return false;
  }
  const uint32_t residual = rest[0] & ~c.shape;
  if (!LookupType(residual)) {
    return false;
  }
  score = c.baseLength * 2 + 1;
  ts[0] = residual;
  CopyResidual(rest, restLength, 1, ts);
  tsLength = restLength;
  return true;
}

// Match a construct's base against the head of `strokes`. On success, writes the
// residual type strokes to `ts`/`tsLength`, sets `score` (longer/fused = more
// specific), and returns true (match_base).
bool MatchBase(const GenConstruct &c, const uint32_t *strokes, uint32_t length,
               uint32_t *ts, uint32_t &tsLength, uint32_t &score) {
  if (length < c.baseLength) {
    return false;
  }
  for (uint32_t i = 0; i < c.baseLength; ++i) {
    if (strokes[i] != c.base[i]) {
      return false;
    }
  }
  const uint32_t *rest = strokes + c.baseLength;
  const uint32_t restLength = length - c.baseLength;

  if (!c.hasShape) {
    score = c.baseLength * 2;
    CopyResidual(rest, restLength, 0, ts);
    tsLength = restLength;
    return true;
  }
  return MatchFusedShape(c, rest, restLength, ts, tsLength, score);
}

// The outcome of a walk: the matched construct, its filled slot texts (in
// template-slot order), and whether the match was terminal (every obligation
// discharged) or partial. `matched` is false iff no construct's base matched,
// or a matched base's residual strokes didn't validly fill its slots.
struct WalkOut {
  const GenConstruct *c = nullptr;
  const char *slotText[MAX_SLOTS];
  uint32_t slotCount = 0;
  bool matched = false;
  bool terminal = false;
};

// Initialize each slot to "" (unfilled slots stay empty in a partial match).
void InitSlotText(uint32_t slotCount, WalkOut &out) {
  for (uint32_t i = 0; i < slotCount; ++i) {
    out.slotText[i] = "";
  }
}

// Fill slots in D12 order from `ts`. Returns false iff a stroke is invalid or
// extra strokes remain unmatched; sets `out.terminal` false on a partial.
bool FillSlots(const GenConstruct &c, const uint32_t *ts, uint32_t tsLength,
               uint32_t slotCount, Arena &arena, WalkOut &out) {
  out.terminal = true;
  uint32_t pos = 0;
  bool ok = true;
  for (uint32_t k = 0; k < slotCount; ++k) {
    if (pos >= tsLength) {
      out.terminal = false;
      break;
    }
    const Consumed consumed = ConsumeType(ts, tsLength, &pos, arena, ok);
    if (!ok) {
      return false;
    }
    if (!consumed.complete) {
      out.terminal = false;
    }
    out.slotText[c.fillOrder[k]] = consumed.text;
  }
  return pos >= tsLength; // false iff extra strokes matched no slot
}

// Walk the residual type strokes against one construct, filling its slots in
// D12 order (walk_construct).
bool WalkConstruct(const GenConstruct &c, const uint32_t *ts, uint32_t tsLength,
                   Arena &arena, WalkOut &out) {
  const uint32_t slotCount = c.slotCount < MAX_SLOTS ? c.slotCount : MAX_SLOTS;
  InitSlotText(slotCount, out);
  if (!FillSlots(c, ts, tsLength, slotCount, arena, out)) {
    return false;
  }
  out.slotCount = slotCount;
  return true;
}

// Pick the most specific construct whose base matches, then replay its type
// obligations. Enumerates nothing.
bool Walk(const uint32_t *strokes, uint32_t length, Arena &arena, WalkOut &out) {
  const GenConstruct *best = nullptr;
  uint32_t bestScore = 0;
  // Static, not stack: same non-reentrancy rationale as Arena/WalkOut (see
  // steno_generated_types.h / steno_generated_dictionary.cc).
  static uint32_t bestTs[MAX_STROKES];
  uint32_t bestTsLength = 0;

  for (uint32_t i = 0; i < CONSTRUCT_COUNT; ++i) {
    static uint32_t ts[MAX_STROKES];
    uint32_t tsLength = 0;
    uint32_t score = 0;
    if (MatchBase(CONSTRUCTS[i], strokes, length, ts, tsLength, score) &&
        (!best || score > bestScore)) {
      best = &CONSTRUCTS[i];
      bestScore = score;
      bestTsLength = tsLength;
      for (uint32_t j = 0; j < tsLength; ++j) {
        bestTs[j] = ts[j];
      }
    }
  }

  if (!best) {
    out.matched = false;
    return false;
  }
  out.c = best;
  out.matched = WalkConstruct(*best, bestTs, bestTsLength, arena, out);
  return out.matched;
}

} // namespace
