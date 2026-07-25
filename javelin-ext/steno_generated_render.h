//---------------------------------------------------------------------------
//
// Two of the three profile renderers: the non-terminal shape shared by all
// profiles, and the terminal nvim renderer. (The terminal plain/smart
// renderer needs the editor simulation and lives in
// steno_generated_render_terminal.h.)
//
//---------------------------------------------------------------------------

#pragma once
#include "steno_generated_escape.h"
#include "steno_generated_walk.h"

namespace {

using steno_generated::GenOp;
using steno_generated::OP_LANDING;
using steno_generated::OP_NEWLINE;
using steno_generated::OP_SLOT;
using steno_generated::OP_TEXT;

// The k-th OP_SLOT in template order fills from `out.slotText[k]`.
const char *SlotTextAt(const WalkOut &out, uint32_t slotIndex) {
  return slotIndex < out.slotCount ? out.slotText[slotIndex] : "";
}

// Non-terminal body shared by all three profiles: bracket-stripped, tabs kept,
// newlines & landings dropped (non_terminal_text).
const char *BuildStrippedBody(const WalkOut &out, Arena &arena) {
  char *start = arena.Mark();
  const GenConstruct &c = *out.c;
  uint32_t slotIndex = 0;
  for (uint32_t i = 0; i < c.opCount; ++i) {
    const GenOp &op = c.ops[i];
    const char *text;
    switch (op.kind) {
    case OP_TEXT:
      text = op.text;
      break;
    case OP_SLOT:
      text = SlotTextAt(out, slotIndex);
      ++slotIndex;
      break;
    default:
      continue; // OP_NEWLINE / OP_LANDING dropped entirely
    }
    for (const char *s = text; *s; ++s) {
      if (!IsBracket(*s)) {
        arena.PushChar(*s);
      }
    }
  }
  return arena.Terminate(start);
}

// Non-terminal render: identical for plain/smart (escape_text), distinct only
// for nvim (esc, no `@@` sentinel — plain `{^}...{^}` glue).
const char *RenderNonTerminal(const WalkOut &out, Profile profile, Arena &arena) {
  const char *body = BuildStrippedBody(out, arena);
  char *start = arena.Mark();
  arena.Push("{^}");
  if (profile == Profile::Nvim) {
    AppendEsc(arena, body);
  } else {
    AppendEscapeText(arena, body);
  }
  arena.Push("{^}");
  return arena.Terminate(start);
}

// Collect the distinct OP_LANDING values in `c.ops`, in first-seen order.
void CollectDistinctLandings(const GenConstruct &c, uint32_t *distinct,
                             uint32_t &count) {
  count = 0;
  for (uint32_t i = 0; i < c.opCount; ++i) {
    if (c.ops[i].kind != OP_LANDING) {
      continue;
    }
    const uint32_t v = c.ops[i].landing;
    bool found = false;
    for (uint32_t j = 0; j < count; ++j) {
      if (distinct[j] == v) {
        found = true;
        break;
      }
    }
    if (!found && count < MAX_SLOTS) {
      distinct[count++] = v;
    }
  }
}

// Insertion sort in place; the small, bounded (<= MAX_SLOTS) landing count
// makes this cheaper than pulling in <algorithm>.
void InsertionSortAscending(uint32_t *values, uint32_t count) {
  for (uint32_t i = 1; i < count; ++i) {
    const uint32_t key = values[i];
    uint32_t j = i;
    while (j > 0 && values[j - 1] > key) {
      values[j] = values[j - 1];
      --j;
    }
    values[j] = key;
  }
}

// The largest landing is the LSP exit (`0`); otherwise 1 + its 0-based
// position in the ascending list of a construct's distinct landings.
uint32_t TabIndex(const GenConstruct &c, uint32_t n) {
  uint32_t distinct[MAX_SLOTS];
  uint32_t count;
  CollectDistinctLandings(c, distinct, count);
  InsertionSortAscending(distinct, count);

  if (count == 0 || n == distinct[count - 1]) {
    return 0;
  }
  for (uint32_t i = 0; i < count; ++i) {
    if (distinct[i] == n) {
      return i + 1;
    }
  }
  return 0; // unreachable: n is always one of this construct's own landings
}

// The 3 UTF-8 bytes of U+2424 (SYMBOL FOR NEWLINE), standing in for a real
// newline inside an inline LSP body so the sentinel span survives Plover
// typing the value on one buffer line (ADR-2).
const char *INLINE_NEWLINE = "\xE2\x90\xA4";

// wrapInlineBody's char-level escape: `\`, `{`, `}` are backslash-escaped;
// a real newline becomes INLINE_NEWLINE; everything else verbatim.
void AppendWrappedInlineBody(Arena &arena, const char *body) {
  for (const char *s = body; *s; ++s) {
    const char ch = *s;
    if (ch == '\\') {
      arena.Push("\\\\");
    } else if (ch == '{') {
      arena.Push("\\{");
    } else if (ch == '}') {
      arena.Push("\\}");
    } else if (ch == '\n') {
      arena.Push(INLINE_NEWLINE);
    } else {
      arena.PushChar(ch);
    }
  }
}

// Walk ops directly (no bracket-stripping): esc-escape text/slots, real
// newlines, `${I}` landings from TabIndex.
void AppendNvimBody(const WalkOut &out, Arena &arena) {
  const GenConstruct &c = *out.c;
  uint32_t slotIndex = 0;
  for (uint32_t i = 0; i < c.opCount; ++i) {
    const GenOp &op = c.ops[i];
    switch (op.kind) {
    case OP_TEXT:
      AppendEsc(arena, op.text);
      break;
    case OP_SLOT:
      AppendEsc(arena, SlotTextAt(out, slotIndex));
      ++slotIndex;
      break;
    case OP_NEWLINE:
      arena.PushChar('\n');
      break;
    case OP_LANDING:
      arena.Push("${");
      AppendUint(arena, TabIndex(c, op.landing));
      arena.Push("}");
      break;
    }
  }
}

// Terminal nvim render: the op-walked body wrapped as one inline LSP sentinel
// span (ADR-2).
const char *RenderTerminalNvim(const WalkOut &out, Arena &arena) {
  char *bodyStart = arena.Mark();
  AppendNvimBody(out, arena);
  const char *body = arena.Terminate(bodyStart);

  char *start = arena.Mark();
  arena.Push("{^}@@");
  AppendWrappedInlineBody(arena, body);
  arena.Push("@@{^}");
  return arena.Terminate(start);
}

} // namespace
