//---------------------------------------------------------------------------
//
// StenoGeneratedDictionary: three programmatic steno dictionaries whose
// entries are never enumerated. Each carries the same Pass-A rule tables
// emitted by `build-javelin` (steno_generated_dictionary_data.h) and
// reconstructs a definition at lookup time by replaying the obligation-stack
// walk (see the of-javelin brief, criterion 6), then rendering the matched
// construct's op-list in its own profile: Plover plain value, Plover smart
// (auto-close) value, or the nvim inline LSP value. This is the C++ port of
// the Rust reference walker's three render profiles.
//
//---------------------------------------------------------------------------

#pragma once
#include "dictionary/dictionary.h"

//---------------------------------------------------------------------------

class StenoGeneratedPlainDictionary final : public StenoDictionary {
public:
  StenoGeneratedPlainDictionary();

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

class StenoGeneratedSmartDictionary final : public StenoDictionary {
public:
  StenoGeneratedSmartDictionary();

  StenoDictionaryLookupResult
  Lookup(const StenoDictionaryLookup &lookup) const final;

  using StenoDictionary::Lookup;

  const char *GetName() const final;

  void PrintInfo(int depth) const final {}
};

class StenoGeneratedSnippetDictionary final : public StenoDictionary {
public:
  StenoGeneratedSnippetDictionary();

  StenoDictionaryLookupResult
  Lookup(const StenoDictionaryLookup &lookup) const final;

  using StenoDictionary::Lookup;

  const char *GetName() const final;

  void PrintInfo(int depth) const final {}
};

//---------------------------------------------------------------------------
