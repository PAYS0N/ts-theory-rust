//---------------------------------------------------------------------------
//
// The terminal plain/smart editor simulation: a construct's ops become a
// token stream (Ch/Enter/Mark), smart drops trailing closers/quotes from
// that stream, and `Sim` replays it to produce the buffer text and cursor
// positions movement is computed from. Transcribed from the Rust reference's
// editor model — see the parity-2-cpp-three-profile-walker brief.
//
//---------------------------------------------------------------------------

#pragma once
#include "steno_generated_render.h"

namespace {

// A token is one typed character, one Enter, or one landing mark; the buffer
// is the simulated document text. Both generous multiples of ARENA_BYTES,
// which already bounds the slot text a construct's ops can reference.
constexpr uint32_t MAX_TOKENS = 4096;
constexpr uint32_t MAX_BUFFER = 8192;

enum class TokKind { Ch, Enter, Mark };

struct Tok {
  TokKind kind;
  char ch;        // Ch only
  uint32_t index; // Mark only
};

// A closer or quote, for smart's trailing-closer drop and type-over.
bool IsCloserOrQuote(char c) {
  switch (c) {
  case ')':
  case ']':
  case '}':
  case '`':
  case '"':
  case '\'':
    return true;
  default:
    return false;
  }
}

// Push one Ch token per character of `text`, bounded by MAX_TOKENS.
void PushCharTokens(const char *text, Tok *tokens, uint32_t &count) {
  for (const char *s = text; *s; ++s) {
    if (count < MAX_TOKENS) {
      tokens[count++] = {TokKind::Ch, *s, 0};
    }
  }
}

// Build the token stream from a construct's ops (OP_TEXT/OP_SLOT -> one Ch per
// character, OP_NEWLINE -> Enter, OP_LANDING(n) -> Mark(n)).
void BuildTokens(const WalkOut &out, Tok *tokens, uint32_t &count) {
  const GenConstruct &c = *out.c;
  uint32_t slotIndex = 0;
  count = 0;
  for (uint32_t i = 0; i < c.opCount; ++i) {
    const GenOp &op = c.ops[i];
    if (op.kind == OP_TEXT || op.kind == OP_SLOT) {
      const char *text =
          op.kind == OP_TEXT ? op.text : SlotTextAt(out, slotIndex);
      if (op.kind == OP_SLOT) {
        ++slotIndex;
      }
      PushCharTokens(text, tokens, count);
    } else if (op.kind == OP_NEWLINE) {
      if (count < MAX_TOKENS) {
        tokens[count++] = {TokKind::Enter, '\0', 0};
      }
    } else if (op.kind == OP_LANDING && count < MAX_TOKENS) {
      tokens[count++] = {TokKind::Mark, '\0', op.landing};
    }
  }
}

// smart only: from the end, remove each trailing Ch whose char is a closer or
// quote, skipping Marks, stopping at the first non-closer/non-quote token.
// The trailing run of Marks-and-closers is found first, then compacted in
// place: closer/quote Ch tokens in that run are dropped, Marks are kept in
// their original relative order.
void DropTrailingClosers(Tok *tokens, uint32_t &count) {
  uint32_t i = count;
  while (i > 0) {
    const Tok &t = tokens[i - 1];
    if (t.kind == TokKind::Mark ||
        (t.kind == TokKind::Ch && IsCloserOrQuote(t.ch))) {
      --i;
      continue;
    }
    break;
  }
  uint32_t w = i;
  for (uint32_t r = i; r < count; ++r) {
    if (tokens[r].kind == TokKind::Mark) {
      tokens[w++] = tokens[r];
    }
  }
  count = w;
}

// The Sim's editor behaviors: PLAIN is bare typing, SMART auto-closes
// brackets/quotes and types over an existing matching closer/quote.
class Sim {
public:
  // `buffer` is caller-owned static storage (MAX_BUFFER bytes), never an
  // automatic local — see the Arena comment in steno_generated_types.h: an 8 KB
  // on-stack editor buffer overflows the device stack and the device emits
  // nothing. Non-reentrant, matching the single steno engine.
  Sim(char *buffer, bool autoClose, bool typeOver)
      : buffer(buffer), autoClose(autoClose), typeOver(typeOver) {}

  void TypeChar(char ch) {
    if (autoClose) {
      if (ch == '(' || ch == '[' || ch == '{') {
        Insert1(ch);
        Insert1(MatchingCloser(ch));
        --cursor;
        return;
      }
      if (ch == '`' || ch == '"' || ch == '\'') {
        if (cursor < length && buffer[cursor] == ch) {
          ++cursor;
          return;
        }
        Insert1(ch);
        Insert1(ch);
        --cursor;
        return;
      }
    }
    if (typeOver && IsCloserOrQuote(ch) && cursor < length &&
        buffer[cursor] == ch) {
      ++cursor;
      return;
    }
    Insert1(ch);
  }

  void Enter() { Insert1('\n'); }

  void Mark(uint32_t n) {
    if (n == 0) {
      hasLanding0 = true;
      landing0 = cursor;
    }
  }

  const char *Buffer() const { return buffer; }
  uint32_t Length() const { return length; }
  uint32_t Cursor() const { return cursor; }
  bool HasLanding0() const { return hasLanding0; }
  uint32_t Landing0() const { return landing0; }

private:
  static char MatchingCloser(char ch) {
    switch (ch) {
    case '(':
      return ')';
    case '[':
      return ']';
    default:
      return '}';
    }
  }

  void Insert1(char ch) {
    if (length >= MAX_BUFFER) {
      return; // truncate rather than overflow; caught by the differential test
    }
    for (uint32_t i = length; i > cursor; --i) {
      buffer[i] = buffer[i - 1];
    }
    buffer[cursor] = ch;
    ++length;
    ++cursor;
  }

  char *buffer;
  uint32_t length = 0;
  uint32_t cursor = 0;
  bool autoClose;
  bool typeOver;
  bool hasLanding0 = false;
  uint32_t landing0 = 0;
};

} // namespace
