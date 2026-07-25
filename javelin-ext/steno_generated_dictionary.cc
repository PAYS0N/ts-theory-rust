//---------------------------------------------------------------------------

#include "steno_generated_dictionary.h"
#include "steno_generated_dictionary_data.h"

#include "steno_generated_render_terminal.h"
#include "steno_generated_walk.h"

//---------------------------------------------------------------------------

using namespace steno_generated;

namespace {

// Shared by all three subclasses: walk the outline, then render it in the
// given profile. `CreateInvalid()` iff no construct's base matched at all.
StenoDictionaryLookupResult LookupProfile(const StenoDictionaryLookup &lookup,
                                          Profile profile) {
  if (lookup.length == 0 || lookup.length > MAX_STROKES) {
    return StenoDictionaryLookupResult::CreateInvalid();
  }

  // Static, not stack: see Arena's constructor comment below. Non-reentrant,
  // matching the single steno engine (one lookup at a time).
  static uint32_t strokes[MAX_STROKES];
  for (size_t i = 0; i < lookup.length; ++i) {
    strokes[i] = lookup.strokes[i].GetKeyState();
  }

  // Static, not stack: see Arena's constructor comment in
  // steno_generated_types.h. Non-reentrant, matching the single steno engine.
  static char arenaStorage[ARENA_BYTES];
  Arena arena(arenaStorage);
  // Static for the same reason as arenaStorage/strokes above: every field
  // read below is unconditionally (re)written whenever Walk() returns true,
  // and out is never read after a false return.
  static WalkOut out;
  if (!Walk(strokes, (uint32_t)lookup.length, arena, out)) {
    return StenoDictionaryLookupResult::CreateInvalid();
  }
  return StenoDictionaryLookupResult::CreateDup(RenderProfile(out, profile, arena));
}

} // namespace

//---------------------------------------------------------------------------

StenoGeneratedPlainDictionary::StenoGeneratedPlainDictionary()
    : StenoDictionary(MAX_OUTLINE_LENGTH) {}

StenoDictionaryLookupResult StenoGeneratedPlainDictionary::Lookup(
    const StenoDictionaryLookup &lookup) const {
  return LookupProfile(lookup, Profile::Plain);
}

const char *StenoGeneratedPlainDictionary::GetName() const {
  return "generated-plain";
}

//---------------------------------------------------------------------------

StenoGeneratedSmartDictionary::StenoGeneratedSmartDictionary()
    : StenoDictionary(MAX_OUTLINE_LENGTH) {}

StenoDictionaryLookupResult StenoGeneratedSmartDictionary::Lookup(
    const StenoDictionaryLookup &lookup) const {
  return LookupProfile(lookup, Profile::Smart);
}

const char *StenoGeneratedSmartDictionary::GetName() const {
  return "generated-smart";
}

//---------------------------------------------------------------------------

StenoGeneratedSnippetDictionary::StenoGeneratedSnippetDictionary()
    : StenoDictionary(MAX_OUTLINE_LENGTH) {}

StenoDictionaryLookupResult StenoGeneratedSnippetDictionary::Lookup(
    const StenoDictionaryLookup &lookup) const {
  return LookupProfile(lookup, Profile::Nvim);
}

const char *StenoGeneratedSnippetDictionary::GetName() const {
  return "generated-snippet";
}

//---------------------------------------------------------------------------
