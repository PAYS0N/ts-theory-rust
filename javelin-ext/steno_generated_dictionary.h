//---------------------------------------------------------------------------
//
// StenoGeneratedDictionary: a programmatic steno dictionary whose entries are
// never enumerated. Instead of a table of outline->definition rows, it carries
// the two Pass-A rule tables emitted by `build-javelin`
// (steno_generated_dictionary_data.h) and reconstructs a definition at lookup
// time by replaying the obligation-stack walk (see the of-javelin brief,
// criterion 6). This is the C++ port of the Rust reference walker.
//
//---------------------------------------------------------------------------

#pragma once
#include "dictionary/dictionary.h"

//---------------------------------------------------------------------------

class StenoGeneratedDictionary final : public StenoDictionary {
public:
  StenoGeneratedDictionary();

  StenoDictionaryLookupResult
  Lookup(const StenoDictionaryLookup &lookup) const final;

  // Keep the base's (strokes, length) convenience overload visible; declaring
  // Lookup above would otherwise hide it.
  using StenoDictionary::Lookup;

  const char *GetName() const final;

  // The generated dictionary has nothing to print and must not pull in the
  // Console machinery (keeps the on-device footprint to the two ROM tables).
  void PrintInfo(int depth) const final {}
};

//---------------------------------------------------------------------------
