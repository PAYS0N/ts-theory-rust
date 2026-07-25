//---------------------------------------------------------------------------
//
// Arena + the type-obligation consumer: the part of the walker that recovers
// a nested type's rendered text from a residual stroke run (consume_type,
// render_type, partial()). Unchanged logic from the pre-three-profile walker;
// only the renderers built on top of it changed.
//
//---------------------------------------------------------------------------

#pragma once
#include "steno_generated_dictionary_data.h"

#include <stdint.h>

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
//
// Storage is caller-owned (a static buffer), never an automatic local: a single
// lookup runs deep in the firmware call stack on a device with only a few KB of
// it, so an 8 KB on-stack arena overflows that stack and the device emits
// nothing. The host differential runs with a multi-megabyte stack and so cannot
// see the overflow; the stack-budget check in cpp_check.sh guards it instead.
// Non-reentrant, matching the single steno engine (one lookup at a time).
class Arena {
public:
  explicit Arena(char *storage) : cursor(storage), limit(storage + ARENA_BYTES) {}

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
  char *cursor;
  char *limit;
};

// One consumed top-level type: its rendered text and whether every obligation
// was discharged.
struct Consumed {
  const char *text;
  bool complete;
};

// True for any delimiter stripped from a non-terminal walk's body.
bool IsBracket(char c) {
  switch (c) {
  case '(':
  case ')':
  case '[':
  case ']':
  case '<':
  case '>':
  case '{':
  case '}':
    return true;
  default:
    return false;
  }
}

using steno_generated::GenType;
using steno_generated::TYPES;
using steno_generated::TYPE_COUNT;

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

// One in-progress type consumption: the type being filled, its target arity,
// how many child args have been rendered so far, and their rendered text.
// Depth is bounded by MAX_STROKES: a frame is only ever pushed immediately
// after consuming one stroke of `ts` (`++*pos` below), and `*pos` cannot
// exceed `length <= MAX_STROKES`.
struct ConsumeFrame {
  const GenType *type;
  uint32_t arity;
  uint32_t argCount = 0;
  const char *args[MAX_ARITY];
};

// Consume one complete (possibly nested) type from `ts` starting at `*pos`,
// advancing `*pos`. `ok` is cleared iff a stroke is not a valid type; a type
// that runs out of strokes mid-obligation returns its partial with
// `complete = false` (consume_type).
//
// Iterative equivalent of the original recursive ConsumeType/ConsumeArgs
// pair: a type's arity children are its stack obligations, so the call
// stack is replaced by an explicit frame stack in static storage (a single
// lookup's recursion depth was otherwise unbounded stack growth on a
// several-KB firmware stack). `stack` is reused across calls exactly like
// Arena/WalkOut (non-reentrant, single lookup at a time); every field of a
// frame is written before being read, so reuse is safe.
Consumed ConsumeType(const uint32_t *ts, uint32_t length, uint32_t *pos,
                     Arena &arena, bool &ok) {
  static ConsumeFrame stack[MAX_STROKES];
  ok = true;

  const GenType *rootType = LookupType(ts[*pos]);
  if (!rootType) {
    ok = false;
    return {nullptr, false};
  }
  ++*pos;
  uint32_t depth = 1;
  stack[0] = {rootType, rootType->arity < MAX_ARITY ? rootType->arity
                                                     : MAX_ARITY};

  for (;;) {
    ConsumeFrame &f = stack[depth - 1];

    if (f.argCount >= f.arity) {
      // Every child obligation discharged: render in full and pop, feeding
      // the rendered text to the parent as its next arg. A fully-completed
      // child never forces its parent to stop taking further children.
      const char *text = RenderType(f.type->text, f.args, f.argCount, arena);
      --depth;
      if (depth == 0) {
        return {text, true};
      }
      ConsumeFrame &parent = stack[depth - 1];
      parent.args[parent.argCount++] = text;
      continue;
    }

    if (*pos >= length) {
      // Out of strokes: every open frame becomes incomplete at once. This
      // mirrors the recursive cascade exactly — once one level runs out,
      // each ancestor in turn appends that partial text as its next arg,
      // marks itself incomplete, and stops taking further children too.
      const char *text = RenderPartial(f.type, f.args, f.argCount, arena);
      for (;;) {
        --depth;
        if (depth == 0) {
          return {text, false};
        }
        ConsumeFrame &parent = stack[depth - 1];
        parent.args[parent.argCount++] = text;
        text = RenderPartial(parent.type, parent.args, parent.argCount,
                             arena);
      }
    }

    const GenType *childType = LookupType(ts[*pos]);
    if (!childType) {
      ok = false;
      return {nullptr, false};
    }
    ++*pos;
    stack[depth] = {childType,
                    childType->arity < MAX_ARITY ? childType->arity
                                                  : MAX_ARITY};
    ++depth;
  }
}

} // namespace
