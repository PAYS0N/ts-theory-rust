//---------------------------------------------------------------------------
//
// The two body escapers shared by all three profile renderers, and the
// `Profile` tag that selects between them. Transcribed byte-for-byte from
// the Rust reference (`escape_text`, the LSP body `esc`) — see the
// parity-2-cpp-three-profile-walker brief for the full derivation.
//
//---------------------------------------------------------------------------

#pragma once
#include "steno_generated_types.h"

#include <string.h>

namespace {

enum class Profile { Plain, Smart, Nvim };

// LSP body escaper (nvim terminal + non-terminal): `\`, `$`, `}` are
// backslash-escaped; `{` and tabs pass through verbatim.
void AppendEsc(Arena &arena, const char *s) {
  for (; *s; ++s) {
    const char ch = *s;
    if (ch == '\\' || ch == '$' || ch == '}') {
      arena.PushChar('\\');
    }
    arena.PushChar(ch);
  }
}

// Push one non-space char, escaping `{`/`}`/newline/tab; anything else passes
// through verbatim.
void AppendEscapedChar(Arena &arena, char c) {
  switch (c) {
  case '{':
    arena.Push("\\{");
    break;
  case '}':
    arena.Push("\\}");
    break;
  case '\n':
    arena.Push("\\n");
    break;
  case '\t':
    arena.Push("\\t");
    break;
  default:
    arena.PushChar(c);
    break;
  }
}

// Push the space run s[i..j) as a single literal space (a lone interior space
// with real characters on both sides) or one `{^ ^}` per space otherwise.
void AppendSpaceRun(Arena &arena, const char *s, uint32_t i, uint32_t j,
                    uint32_t len) {
  const uint32_t run = j - i;
  const bool hasBefore = i > 0;
  const char before = hasBefore ? s[i - 1] : '\0';
  const bool hasAfter = j < len;
  const char after = hasAfter ? s[j] : '\0';
  const bool safe =
      run == 1 && hasBefore && before != '\n' && hasAfter && after != '\n';
  if (safe) {
    arena.PushChar(' ');
    return;
  }
  for (uint32_t k = 0; k < run; ++k) {
    arena.Push("{^ ^}");
  }
}

// Plover value escaper (plain/smart terminal + non-terminal), transcribed
// byte-for-byte from the Rust reference's `escape_text`. A lone interior space
// with real characters on both sides stays a literal space; every other space
// run becomes one `{^ ^}` per space. Newlines/tabs/braces are backslash-escaped.
void AppendEscapeText(Arena &arena, const char *s) {
  const uint32_t len = (uint32_t)strlen(s);
  uint32_t i = 0;
  while (i < len) {
    if (s[i] == ' ') {
      uint32_t j = i;
      while (j < len && s[j] == ' ') {
        ++j;
      }
      AppendSpaceRun(arena, s, i, j, len);
      i = j;
      continue;
    }
    AppendEscapedChar(arena, s[i]);
    ++i;
  }
}

// Append the base-10 digits of `n` (no sign; landing indices are unsigned).
void AppendUint(Arena &arena, uint32_t n) {
  char digits[10];
  uint32_t count = 0;
  do {
    digits[count++] = char('0' + (n % 10));
    n /= 10;
  } while (n != 0 && count < sizeof(digits));
  while (count > 0) {
    arena.PushChar(digits[--count]);
  }
}

} // namespace
