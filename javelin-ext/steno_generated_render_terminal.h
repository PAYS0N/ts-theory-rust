//---------------------------------------------------------------------------
//
// The third profile renderer (terminal plain/smart) and the top-level
// dispatcher all three `Lookup` implementations call. This is the
// editor-simulation port: build tokens, drop trailing closers (smart only),
// replay through `Sim`, then serialize the typed text plus any movement
// group computed from the simulated buffer.
//
//---------------------------------------------------------------------------

#pragma once
#include "steno_generated_movement.h"
#include "steno_generated_render.h"
#include "steno_generated_sim.h"

namespace {

// The typed text: concat of the token stream's Ch (as char) and Enter (as
// '\n'); Mark tokens are invisible. From `tokens[0..count)`, i.e. the
// closer-dropped stream for smart — never the simulated buffer, since
// auto-close only affects the buffer (used for movement math), not what was
// actually typed.
const char *TypedText(const Tok *tokens, uint32_t count, Arena &arena) {
  char *start = arena.Mark();
  for (uint32_t i = 0; i < count; ++i) {
    const Tok &tok = tokens[i];
    if (tok.kind == TokKind::Ch) {
      arena.PushChar(tok.ch);
    } else if (tok.kind == TokKind::Enter) {
      arena.PushChar('\n');
    }
  }
  return arena.Terminate(start);
}

// Replay the token stream through `sim` (Ch -> TypeChar, Enter -> Enter,
// Mark -> Mark).
void ReplayTokens(const Tok *tokens, uint32_t count, Sim &sim) {
  for (uint32_t i = 0; i < count; ++i) {
    const Tok &tok = tokens[i];
    switch (tok.kind) {
    case TokKind::Ch:
      sim.TypeChar(tok.ch);
      break;
    case TokKind::Enter:
      sim.Enter();
      break;
    case TokKind::Mark:
      sim.Mark(tok.index);
      break;
    }
  }
}

// The escaped serialized text: the typed characters (closer-dropped for
// smart, from `tokens`), Plover-escaped. Never the simulated buffer — that's
// only for movement math; auto-close affects the buffer, not what was typed.
const char *ComputeEscapedTypedText(const Tok *tokens, uint32_t count,
                                    Arena &arena) {
  const char *typedText = TypedText(tokens, count, arena);
  char *start = arena.Mark();
  AppendEscapeText(arena, typedText);
  return arena.Terminate(start);
}

// The `{#...}` movement group from rest -> landing0, or "" if no Mark(0)
// occurred.
const char *ComputeMovement(const Sim &sim, Arena &arena) {
  char *start = arena.Mark();
  if (sim.HasLanding0()) {
    AppendMovementGroup(arena, sim.Buffer(), sim.Length(), sim.Cursor(),
                        sim.Landing0());
  }
  return arena.Terminate(start);
}

// Terminal plain/smart render: build tokens, drop trailing closers (smart
// only), simulate through Sim for the buffer/cursor/landing0, then serialize
// the typed text plus any movement group.
const char *RenderTerminalPlainSmart(const WalkOut &out, Profile profile,
                                     Arena &arena) {
  static Tok tokens[MAX_TOKENS];
  uint32_t count = 0;
  BuildTokens(out, tokens, count);
  const bool smart = profile == Profile::Smart;
  if (smart) {
    DropTrailingClosers(tokens, count);
  }

  // Static, not stack: the editor buffer must not sit in this frame alongside
  // the arena (that ~19 KB frame overflowed the device stack). Non-reentrant,
  // like `tokens` above.
  static char simBuffer[MAX_BUFFER];
  Sim sim(simBuffer, /*autoClose=*/smart, /*typeOver=*/smart);
  ReplayTokens(tokens, count, sim);

  const char *escaped = ComputeEscapedTypedText(tokens, count, arena);
  const char *movement = ComputeMovement(sim, arena);

  char *start = arena.Mark();
  arena.Push("{^}");
  arena.Push(escaped);
  arena.Push(movement);
  arena.Push("{^}");
  return arena.Terminate(start);
}

// Dispatch to the right renderer for `profile`, terminal or non-terminal.
const char *RenderProfile(const WalkOut &out, Profile profile, Arena &arena) {
  if (!out.terminal) {
    return RenderNonTerminal(out, profile, arena);
  }
  if (profile == Profile::Nvim) {
    return RenderTerminalNvim(out, arena);
  }
  return RenderTerminalPlainSmart(out, profile, arena);
}

} // namespace
