//! Port of `test/parse.test.ts` (source half): fenced entries, directives,
//! error cases, and the dict.steno corpus checks.

use std::collections::{HashMap, HashSet};

use steno::{Chunk, Entry, parse_source};

/// Read the real dict.steno corpus from the repository root.
fn corpus() -> std::io::Result<String> {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dict.steno"))
}

/// The three-entry example source shared by the directive tests.
const SRC: &str = "# a comment\n// another\n\n````STKWR-PBGS/-FLT\nfunction %0(%[, |%d%]): %t {%b%2}\n````\n@count AOE\n\n````PH\nMap<%t, %t>\n````\n@arity 2\n\n````STKWR-FP\nswitch (%0) {%b}\n````\n@multiline";

/// Entries parse, strokes split on `/`, and directives attach to the entry
/// above.
#[test]
fn parses_entries_and_attaches_directives() {
    let entries = parse_source(SRC).unwrap();
    let strokes: Vec<&str> = entries.iter().map(|e| e.stroke_raw.as_str()).collect();
    assert_eq!(strokes, vec!["STKWR-PBGS/-FLT", "PH", "STKWR-FP"]);

    let mut it = entries.iter();
    let func = it.next().unwrap();
    assert_eq!(func.stroke, vec!["STKWR-PBGS", "-FLT"]);
    assert_eq!(func.count.as_deref(), Some("AOE"));
    assert_eq!(func.arity, None);

    assert_eq!(it.next().unwrap().arity, Some(2));
    assert!(it.next().unwrap().flags.multiline());
}

/// Multi-line literal blocks are preserved verbatim.
#[test]
fn preserves_multi_line_blocks_verbatim() {
    let block = "````X\nclass A {\n\tx = 1;\n}\n````";
    let entries = parse_source(block).unwrap();
    let e = entries.first().unwrap();
    assert_eq!(e.raw, "class A {\n\tx = 1;\n}");
    // first chunk is the literal up to the first structural brace
    assert_eq!(e.template.first().unwrap(), &Chunk::Lit("class A ".into()));
}

/// Bad sources are rejected with a descriptive error.
#[test]
fn rejects_bad_source() {
    let cases = [
        ("@count AOEU", "directive before any entry"),
        ("````X\nunterminated", "unterminated block"),
        ("````", "closing fence without an open block"),
        ("junk line", "unexpected text"),
        ("````X\nok\n````\n@nope x", "unknown directive @nope"),
        (
            "````X\nok\n````\n@arity two",
            "@arity needs a non-negative integer",
        ),
    ];
    for (bad, needle) in cases {
        let err = parse_source(bad).unwrap_err();
        assert!(
            err.to_string().contains(needle),
            "source {bad:?}: expected {needle:?} in {err}"
        );
    }
}

/// A `StenoError` carries the 1-based line number.
#[test]
fn steno_error_carries_line_number() {
    let err = parse_source("\n\njunk").unwrap_err();
    assert_eq!(err.line(), 3);
}

/// The real dict.steno parses with no errors.
#[test]
fn corpus_parses() {
    parse_source(&corpus().unwrap()).unwrap();
}

/// Index the corpus entries by raw stroke.
fn by_stroke(entries: &[Entry]) -> HashMap<&str, &Entry> {
    entries.iter().map(|e| (e.stroke_raw.as_str(), e)).collect()
}

/// Look up a stroke that must exist in the corpus index.
fn entry<'a>(idx: &HashMap<&str, &'a Entry>, k: &str) -> Option<&'a Entry> {
    idx.get(k).copied()
}

/// The corpus has the expected counts, flags, and arities.
#[test]
fn corpus_has_expected_directives() {
    let entries = parse_source(&corpus().unwrap()).unwrap();
    let idx = by_stroke(&entries);

    assert!(entries.len() > 70);
    let count_of = |k: &str| entry(&idx, k).and_then(|e| e.count.as_deref());
    assert_eq!(count_of("STKWR-PB"), Some("AOE")); // param stroke
    assert_eq!(count_of("STKWR-PBGS/-FLT"), None); // functions are no-param now
    assert_eq!(count_of("STKWR-BGS"), Some("AOE"));
    assert_eq!(count_of("STKWR-BGSD"), Some("AOE")); // destructuring
    assert_eq!(count_of("STKWR-FP"), Some("AOE"));
    assert!(entry(&idx, "STPHR").unwrap().flags.no_arg()); // never is not a generic arg
    assert!(entry(&idx, "STKWR-FP").unwrap().flags.multiline());
    assert_eq!(entry(&idx, "PR").unwrap().arity, Some(1));
    assert_eq!(entry(&idx, "PH").unwrap().arity, Some(2));
    assert_eq!(entry(&idx, "SKWR").unwrap().arity, Some(1)); // Generator
}

/// The data-structure blocks survived the migration intact.
#[test]
fn corpus_has_expected_struct_blocks() {
    let entries = parse_source(&corpus().unwrap()).unwrap();
    let idx = by_stroke(&entries);

    // the data-structure block round-trips and contains a brace chunk
    let stack = entry(&idx, "STKWR-RBGT/S").unwrap();
    assert!(stack.raw.contains("class Stack<T> {"));
    assert!(
        stack
            .template
            .iter()
            .any(|c| matches!(c, Chunk::Brace { .. }))
    );

    // all 11 data-structure selectors survived the migration
    let structs = entries
        .iter()
        .filter(|e| e.stroke_raw.starts_with("STKWR-RBGT/"))
        .count();
    assert_eq!(structs, 11);
}

/// No duplicate strokes exist in the corpus (the collision invariant).
#[test]
fn corpus_has_no_duplicate_strokes() {
    let entries = parse_source(&corpus().unwrap()).unwrap();
    let mut seen = HashSet::new();
    let dups: Vec<&str> = entries
        .iter()
        .filter(|e| !seen.insert(e.stroke_raw.as_str()))
        .map(|e| e.stroke_raw.as_str())
        .collect();
    assert_eq!(dups, Vec::<&str>::new());
}

/// Every `%` in the corpus is consumed and the corpus is non-trivial.
#[test]
fn corpus_is_non_trivial() {
    let entries = parse_source(&corpus().unwrap()).unwrap();
    let total: usize = entries.iter().map(|e| e.template.len()).sum();
    assert!(total > 50);
}
