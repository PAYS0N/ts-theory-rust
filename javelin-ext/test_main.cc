//---------------------------------------------------------------------------
//
// Differential test for StenoGeneratedDictionary (of-javelin brief, D9): replay
// every golden stroke sequence emitted by `build-javelin` through the generated
// dictionary and assert the definition and validity match the Rust reference
// walker's recorded verdict. Built and run by scripts/cpp_check.sh with
// RUN_TESTS=1; it is the C++-vs-walker half of the differential.
//
//---------------------------------------------------------------------------

// Only the check harness compiles a main(); a firmware build of this directory
// defines nothing here, so the translation unit collapses to empty. This gate
// is deliberately NOT javelin-steno's own RUN_TESTS macro, which would pull in
// that project's in-source UnitTest framework.
#if JAVELIN_EXT_RUN_TESTS

#include "steno_generated_dictionary.h"
#include "steno_generated_dictionary_data.h"
#include "steno_generated_testdata.h"

#include "stroke.h"

#include <stdio.h>
#include <string.h>

//---------------------------------------------------------------------------

// The generated dictionary overrides PrintInfo, so no code path reaches the
// Console. This stub only satisfies the linker if --gc-sections keeps a
// reference; it is never called.
#include "console.h"
void Console::PrintfInternal(const char *, ...) {}

//---------------------------------------------------------------------------

using namespace steno_generated;

static int CheckGolden(const StenoGeneratedDictionary &dictionary,
                       const GenGolden &golden) {
  StenoStroke strokes[64];
  for (uint32_t i = 0; i < golden.length; ++i) {
    strokes[i] = StenoStroke(golden.strokes[i]);
  }

  StenoDictionaryLookupResult result =
      dictionary.Lookup(strokes, golden.length);

  int failed = 0;
  if (golden.valid) {
    if (!result.IsValid()) {
      printf("FAIL: expected valid \"%s\", got invalid\n", golden.text);
      failed = 1;
    } else if (strcmp(result.GetText(), golden.text) != 0) {
      printf("FAIL: expected \"%s\", got \"%s\"\n", golden.text,
             result.GetText());
      failed = 1;
    }
  } else if (result.IsValid()) {
    printf("FAIL: expected invalid, got valid \"%s\"\n", result.GetText());
    failed = 1;
  }

  result.Destroy();
  return failed;
}

int main() {
  StenoGeneratedDictionary dictionary;

  int failures = 0;
  for (uint32_t i = 0; i < GOLDEN_COUNT; ++i) {
    failures += CheckGolden(dictionary, GOLDENS[i]);
  }

  if (failures != 0) {
    printf("FAIL: %d of %u golden(s) mismatched the reference walker\n",
           failures, GOLDEN_COUNT);
    return 1;
  }
  printf("pass: %u goldens agree with the reference walker\n", GOLDEN_COUNT);
  return 0;
}

#endif // JAVELIN_EXT_RUN_TESTS

//---------------------------------------------------------------------------
