//---------------------------------------------------------------------------
//
// Differential test for the three StenoGenerated*Dictionary subclasses (of-
// javelin brief, D9): replay every golden stroke sequence emitted by
// `build-javelin` through each generated dictionary and assert the
// plain/smart/nvim definitions and validity match the Rust reference
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

// The generated dictionaries override PrintInfo, so no code path reaches the
// Console. This stub only satisfies the linker if --gc-sections keeps a
// reference; it is never called.
#include "console.h"
void Console::PrintfInternal(const char *, ...) {}

//---------------------------------------------------------------------------

using namespace steno_generated;

// `valid` handling shared across all three profiles: if valid, the result
// must be valid and strcmp-equal to `expected`; if not, the result must be
// invalid regardless of what `expected` holds. Prints a FAIL line and
// returns 1 on mismatch.
static int CheckResult(const StenoDictionaryLookupResult &result,
                       const GenGolden &golden, const char *expected,
                       const char *profileName) {
  if (golden.valid) {
    if (!result.IsValid()) {
      printf("FAIL: [%s] expected valid \"%s\", got invalid\n", profileName,
             expected);
      return 1;
    }
    if (strcmp(result.GetText(), expected) != 0) {
      printf("FAIL: [%s] expected \"%s\", got \"%s\"\n", profileName,
             expected, result.GetText());
      return 1;
    }
    return 0;
  }
  if (result.IsValid()) {
    printf("FAIL: [%s] expected invalid, got valid \"%s\"\n", profileName,
           result.GetText());
    return 1;
  }
  return 0;
}

// One profile check: `expected` against `dictionary`'s lookup of `golden`'s
// strokes.
static int CheckGolden(const StenoDictionary &dictionary,
                       const GenGolden &golden, const char *expected,
                       const char *profileName) {
  StenoStroke strokes[64];
  for (uint32_t i = 0; i < golden.length; ++i) {
    strokes[i] = StenoStroke(golden.strokes[i]);
  }

  StenoDictionaryLookupResult result =
      dictionary.Lookup(strokes, golden.length);
  const int failed = CheckResult(result, golden, expected, profileName);
  result.Destroy();
  return failed;
}

int main() {
  StenoGeneratedPlainDictionary plainDictionary;
  StenoGeneratedSmartDictionary smartDictionary;
  StenoGeneratedSnippetDictionary snippetDictionary;

  int failures = 0;
  for (uint32_t i = 0; i < GOLDEN_COUNT; ++i) {
    const GenGolden &golden = GOLDENS[i];
    failures += CheckGolden(plainDictionary, golden, golden.plain, "plain");
    failures += CheckGolden(smartDictionary, golden, golden.smart, "smart");
    failures += CheckGolden(snippetDictionary, golden, golden.nvim, "nvim");
  }

  if (failures != 0) {
    printf("FAIL: %d of %u golden checks mismatched the reference walker\n",
           failures, GOLDEN_COUNT * 3);
    return 1;
  }
  printf("pass: %u goldens agree with the reference walker across all three "
        "profiles\n",
        GOLDEN_COUNT);
  return 0;
}

#endif // JAVELIN_EXT_RUN_TESTS

//---------------------------------------------------------------------------
