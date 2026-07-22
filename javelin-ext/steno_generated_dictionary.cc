//---------------------------------------------------------------------------

#include "steno_generated_dictionary.h"
#include "steno_generated_dictionary_data.h"

#include "str.h"

//---------------------------------------------------------------------------

using namespace steno_generated;

namespace {

// Fixed working limits. A single lookup is bounded by the engine's maximum
// outline length, so the residual-stroke and slot fan-out are small; the text
// arena is generous enough for the deepest nesting reachable inside that stroke
// budget (see the of-javelin brief, D8: the walk is unbounded, one lookup is
// not). Overflow degrades to a truncated string, which the differential test
// against the Rust walker would catch, never memory corruption.
constexpr uint32_t MAX_STROKES = 64;
constexpr uint32_t MAX_SLOTS = 64;
constexpr uint32_t MAX_ARITY = 16;
constexpr uint32_t ARENA_BYTES = 8192;

// A bump arena that materialises rendered fragments as NUL-terminated C strings.
// Children are finished before their parent renders, so a parent copies child
// bytes forward into a strictly higher region (no overlap).
class Arena {
public:
  Arena() : cursor(data), limit(data + ARENA_BYTES) {}

  char *Mark() { return cursor; }

  void Push(const char *s) {
    while (*s && cursor < limit) {
      *cursor++ = *s++;
    }
  }

  void PushChar(char c) {
    if (cursor < limit) {
      *cursor++ = c;
    }
  }

  // Terminate the string that began at `start` and return it.
  char *Terminate(char *start) {
    PushChar('\0');
    return start;
  }

private:
  char data[ARENA_BYTES];
  char *cursor;
  char *limit;
};

// One consumed top-level type: its rendered text and whether every obligation
// was discharged.
struct Consumed {
  const char *text;
  bool complete;
};

const GenType *LookupType(uint32_t stroke) {
  for (uint32_t i = 0; i < TYPE_COUNT; ++i) {
    if (TYPES[i].stroke == stroke) {
      return &TYPES[i];
    }
  }
  return nullptr;
}

// Substitute `args` into the `%t` markers of a type's text (render_type).
const char *RenderType(const char *text, const char *const *args,
                       uint32_t argCount, Arena &arena) {
  char *start = arena.Mark();
  uint32_t argIndex = 0;
  for (const char *s = text; *s;) {
    if (s[0] == '%' && s[1] == 't') {
      if (argIndex < argCount) {
        arena.Push(args[argIndex]);
      }
      ++argIndex;
      s += 2;
    } else {
      arena.PushChar(*s++);
    }
  }
  return arena.Terminate(start);
}

// The bracketless partial form: `Array`, or `Map number` (partial()).
const char *RenderPartial(const GenType *type, const char *const *args,
                          uint32_t argCount, Arena &arena) {
  char *start = arena.Mark();
  for (const char *s = type->text; *s && *s != '<'; ++s) {
    arena.PushChar(*s);
  }
  for (uint32_t i = 0; i < argCount; ++i) {
    arena.PushChar(' ');
    arena.Push(args[i]);
  }
  return arena.Terminate(start);
}

// Consume one complete (possibly nested) type from `ts` starting at `*pos`,
// advancing `*pos`. `ok` is cleared iff a stroke is not a valid type; a type
// that runs out of strokes mid-obligation returns its partial with
// `complete = false` (consume_type).
Consumed ConsumeType(const uint32_t *ts, uint32_t length, uint32_t *pos,
                     Arena &arena, bool &ok) {
  const GenType *type = LookupType(ts[*pos]);
  if (!type) {
    ok = false;
    return {nullptr, false};
  }
  ++*pos;

  const char *args[MAX_ARITY];
  uint32_t argCount = 0;
  bool complete = true;
  const uint32_t arity = type->arity < MAX_ARITY ? type->arity : MAX_ARITY;
  for (uint32_t i = 0; i < arity; ++i) {
    if (*pos >= length) {
      complete = false;
      break;
    }
    const Consumed arg = ConsumeType(ts, length, pos, arena, ok);
    if (!ok) {
      return {nullptr, false};
    }
    args[argCount++] = arg.text;
    if (!arg.complete) {
      complete = false;
      break;
    }
  }

  if (!complete) {
    return {RenderPartial(type, args, argCount, arena), false};
  }
  return {RenderType(type->text, args, argCount, arena), true};
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
    for (uint32_t i = 0; i < restLength; ++i) {
      ts[i] = rest[i];
    }
    tsLength = restLength;
    return true;
  }

  // Fuse inversion: the return type was merged into the shape stroke. Recover it
  // by subtracting the shape's keys; it must be a subset and a valid type.
  if (restLength < 1 || (rest[0] & c.shape) != c.shape) {
    return false;
  }
  const uint32_t residual = rest[0] & ~c.shape;
  if (!LookupType(residual)) {
    return false;
  }
  score = c.baseLength * 2 + 1;
  ts[0] = residual;
  for (uint32_t i = 1; i < restLength; ++i) {
    ts[i] = rest[i];
  }
  tsLength = restLength;
  return true;
}

// Render a construct's template with its slots filled in template order
// (render_filled). Unfilled slots contribute their default empty string.
const char *RenderFilled(const GenConstruct &c, const char *const *slotText,
                         Arena &arena) {
  char *start = arena.Mark();
  arena.Push(c.fragments[0]);
  for (uint32_t i = 0; i < c.slotCount; ++i) {
    arena.Push(slotText[i]);
    arena.Push(c.fragments[i + 1]);
  }
  return arena.Terminate(start);
}

// Walk the residual type strokes against one construct, filling its slots in
// D12 order (walk_construct). Sets `terminal` false on a partial; returns
// nullptr iff a stroke is invalid or extra strokes remain unmatched.
const char *WalkConstruct(const GenConstruct &c, const uint32_t *ts,
                          uint32_t tsLength, Arena &arena, bool &terminal) {
  const char *empty = "";
  const char *slotText[MAX_SLOTS];
  const uint32_t slotCount = c.slotCount < MAX_SLOTS ? c.slotCount : MAX_SLOTS;
  for (uint32_t i = 0; i < slotCount; ++i) {
    slotText[i] = empty;
  }

  terminal = true;
  uint32_t pos = 0;
  bool ok = true;
  for (uint32_t k = 0; k < slotCount; ++k) {
    if (pos >= tsLength) {
      terminal = false;
      break;
    }
    const Consumed consumed = ConsumeType(ts, tsLength, &pos, arena, ok);
    if (!ok) {
      return nullptr;
    }
    if (!consumed.complete) {
      terminal = false;
    }
    slotText[c.fillOrder[k]] = consumed.text;
  }
  if (pos < tsLength) {
    return nullptr; // extra strokes matched no slot
  }
  return RenderFilled(c, slotText, arena);
}

// The walker (criterion 3, ported): pick the most specific construct whose base
// matches, then replay its type obligations. Enumerates nothing.
const char *Walk(const uint32_t *strokes, uint32_t length, Arena &arena,
                 bool &terminal) {
  const GenConstruct *best = nullptr;
  uint32_t bestScore = 0;
  uint32_t bestTs[MAX_STROKES];
  uint32_t bestTsLength = 0;

  for (uint32_t i = 0; i < CONSTRUCT_COUNT; ++i) {
    uint32_t ts[MAX_STROKES];
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
    return nullptr;
  }
  return WalkConstruct(*best, bestTs, bestTsLength, arena, terminal);
}

} // namespace

//---------------------------------------------------------------------------

StenoGeneratedDictionary::StenoGeneratedDictionary()
    : StenoDictionary(MAX_OUTLINE_LENGTH) {}

StenoDictionaryLookupResult
StenoGeneratedDictionary::Lookup(const StenoDictionaryLookup &lookup) const {
  if (lookup.length == 0 || lookup.length > MAX_STROKES) {
    return StenoDictionaryLookupResult::CreateInvalid();
  }

  uint32_t strokes[MAX_STROKES];
  for (size_t i = 0; i < lookup.length; ++i) {
    strokes[i] = lookup.strokes[i].GetKeyState();
  }

  Arena arena;
  bool terminal = false;
  const char *text =
      Walk(strokes, (uint32_t)lookup.length, arena, terminal);

  // Only a terminal walk is a complete definition; a partial outline is not a
  // match (its verdict mirrors the walker's `terminal` flag / golden validity).
  if (!text || !terminal) {
    return StenoDictionaryLookupResult::CreateInvalid();
  }
  return StenoDictionaryLookupResult::CreateDup(text);
}

const char *StenoGeneratedDictionary::GetName() const {
  return "generated";
}

//---------------------------------------------------------------------------
