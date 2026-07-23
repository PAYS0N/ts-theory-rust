//! Port of `test/expand.test.ts`: Pass A count expansion, pass-through and
//! misconfiguration errors, and the full-corpus round trip.

use steno::{Chunk, Entry, ExpandedEntry, expand_all, expand_counts, parse_source};

/// Render resolved chunks for assertions. Any count chunk left over is a bug.
fn show(chunks: &[Chunk]) -> String {
    chunks
        .iter()
        .map(|c| match c {
            Chunk::Lit(t) => t.clone(),
            Chunk::Landing(n) => format!("%{n}"),
            Chunk::Brace { open } => (if *open { "{" } else { "}" }).to_string(),
            Chunk::Newline => "\\n".to_string(),
            Chunk::Tab => "\\t".to_string(),
            Chunk::TypeSlot => "%t".to_string(),
            Chunk::BodyBreak => "%b".to_string(),
            Chunk::Pattern => "%p".to_string(),
            other => format!("<UNRESOLVED:{}>", other.kind_name()),
        })
        .collect()
}

/// Parse a single-entry source and return that entry.
fn one(src: &str) -> Option<Entry> {
    let mut entries = parse_source(src).ok()?;
    (entries.len() == 1).then(|| entries.pop()).flatten()
}

/// Find the expansion with the given count.
fn at(ex: &[ExpandedEntry], d: u32) -> Option<&ExpandedEntry> {
    ex.iter().find(|e| e.count == Some(d))
}

/// The index construct fans out to 16 entries (0..15) over the FPLT bank.
#[test]
fn index_fans_out_over_fplt() {
    let ex = expand_counts(&one("````STKWR-BGS\n[%d]\n````\n@count FPLT").unwrap()).unwrap();
    assert_eq!(ex.len(), 16);
    let counts: Vec<Option<u32>> = ex.iter().map(|e| e.count).collect();
    assert_eq!(counts, (0..16).map(Some).collect::<Vec<_>>());

    // count 0 is the bare stroke -> [0]
    assert_eq!(at(&ex, 0).unwrap().stroke, "STKWR-BGS");
    assert_eq!(show(&at(&ex, 0).unwrap().template), "[0]");

    // non-zero counts merge bank keys and render the number
    assert_eq!(at(&ex, 1).unwrap().stroke, "STKWR-FBGS"); // +F
    assert_eq!(show(&at(&ex, 1).unwrap().template), "[1]");
    assert_eq!(at(&ex, 15).unwrap().stroke, "STKWR-FPBLGTS"); // +F,P,L,T
    assert_eq!(show(&at(&ex, 15).unwrap().template), "[15]");
}

/// Repeat with computed landings: %(d+1) with 0-based iteration yields
/// %1, %2, %3 — name %0 stays free.
#[test]
fn repeat_with_computed_landings() {
    let src = "````STKWR-PBGS/-FLT\n(%[, |%(d+1)%])\n````\n@count AOEU";
    let ex = expand_counts(&one(src).unwrap()).unwrap();

    // count 0 -> empty parens, bare second stroke
    assert_eq!(at(&ex, 0).unwrap().stroke, "STKWR-PBGS/-FLT");
    assert_eq!(show(&at(&ex, 0).unwrap().template), "()");

    // count 3 -> three comma-joined landings, merged stroke (A|O into -FLT)
    assert_eq!(at(&ex, 3).unwrap().stroke, "STKWR-PBGS/AOFLT");
    assert_eq!(show(&at(&ex, 3).unwrap().template), "(%1, %2, %3)");

    // the separator is a joiner (no trailing comma)
    assert_eq!(show(&at(&ex, 1).unwrap().template), "(%1)");
}

/// Switch-shaped constructs give each iteration a distinct landing pair.
#[test]
fn switch_shaped_per_iteration_landings() {
    let src = "````STKWR-FP\nswitch(%0) {%b%[case %(2d+1):\\n%(2d+2)\\nbreak;\\n%]}\n````\n@count RBGS\n@multiline";
    let ex = expand_counts(&one(src).unwrap()).unwrap();
    assert_eq!(
        show(&at(&ex, 2).unwrap().template),
        "switch(%0) {%bcase %1:\\n%2\\nbreak;\\ncase %3:\\n%4\\nbreak;\\n}"
    );
}

/// A computed landing going negative is a hard error.
#[test]
fn negative_computed_landing_errors() {
    // %(2d-1) with 0-based iteration is -1 on the first case.
    let bad = "````STKWR-FP\n{%b%[%(2d-1)%]}\n````\n@count RBGS";
    let err = expand_counts(&one(bad).unwrap()).unwrap_err();
    assert!(err.to_string().contains("< 0"));
}

/// `%<0>` lands after a repeat's computed landings, at a few count values.
#[test]
fn end_landing_after_repeat_computed_landings() {
    let src = "````STKWR-PBGS/-FLT\nname %0(%[, |%(d+1)%]) {%<0>}\n````\n@count AOEU";
    let ex = expand_counts(&one(src).unwrap()).unwrap();

    // 0 params -> body lands right after the name, at %1 (nothing else to skip)
    assert_eq!(show(&at(&ex, 0).unwrap().template), "name %0() {%1}");

    // 1 param -> param takes %1, body moves to %2
    assert_eq!(show(&at(&ex, 1).unwrap().template), "name %0(%1) {%2}");

    // 3 params -> params take %1..%3, body moves to %4 (one past the last)
    assert_eq!(
        show(&at(&ex, 3).unwrap().template),
        "name %0(%1, %2, %3) {%4}"
    );
}

/// `%<N>` resolving before enough landings exist is a hard error.
#[test]
fn end_landing_underflow_errors() {
    let err = expand_counts(&one("````STKWR-FP\n%<5>\n````").unwrap()).unwrap_err();
    assert!(
        err.to_string()
            .contains("resolves before enough landings exist")
    );
}

/// Two `%<N>`s in the same template both resolve against the same total
/// landing count, regardless of which one is written first: whichever
/// position holds `%<0>` always gets the highest index, and `%<1>` the next
/// one down.
#[test]
fn two_end_landings_resolve_order_independently() {
    let ex = expand_counts(&one("````STKWR-FP\n{%<1>, %0, %<0>}\n````").unwrap()).unwrap();
    assert_eq!(show(&ex.first().unwrap().template), "{%1, %0, %2}");

    let ex2 = expand_counts(&one("````STKWR-BGS\n{%<0>, %0, %<1>}\n````").unwrap()).unwrap();
    assert_eq!(show(&ex2.first().unwrap().template), "{%2, %0, %1}");
}

/// A non-count entry passes through unchanged.
#[test]
fn non_count_entry_passes_through() {
    let ex = expand_counts(&one("````STKWR-LG\nconsole.log(%0)\n````").unwrap()).unwrap();
    assert_eq!(ex.len(), 1);
    let e = ex.first().unwrap();
    assert_eq!(e.count, None);
    assert_eq!(e.stroke, "STKWR-LG");
    assert_eq!(show(&e.template), "console.log(%0)");
}

/// @count without a count operator (and vice versa) are errors.
#[test]
fn count_misconfiguration_errors() {
    let err = expand_counts(&one("````X\nhello\n````\n@count AOEU").unwrap()).unwrap_err();
    assert!(err.to_string().contains("no count operator"));

    let err2 = expand_counts(&one("````X\n[%d]\n````").unwrap()).unwrap_err();
    assert!(err2.to_string().contains("no @count"));
}

/// Read the real dict.steno corpus from the repository root.
fn corpus_entries() -> Option<Vec<Entry>> {
    let text =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dict.steno")).ok()?;
    parse_source(&text).ok()
}

/// The corpus expands end-to-end with no errors and grows the entry set.
#[test]
fn corpus_expands_and_grows() {
    let entries = corpus_entries().unwrap();
    let all = expand_all(&entries).unwrap();
    assert!(all.len() > entries.len());
}

/// The param stroke at count 2 gives two comma-joined cursor slots, no
/// parens.
#[test]
fn corpus_param_stroke_count_two() {
    let entries = corpus_entries().unwrap();
    let all = expand_all(&entries).unwrap();
    // STKWR-PB + O (=2)
    let e = all.iter().find(|x| x.stroke == "STKWROPB").unwrap();
    assert_eq!(show(&e.template), "%0, %1");
}

/// No expanded corpus entry leaves a count operator unresolved.
#[test]
fn corpus_leaves_no_count_operators() {
    let entries = corpus_entries().unwrap();
    for e in expand_all(&entries).unwrap() {
        let unresolved = e
            .template
            .iter()
            .any(|c| matches!(c, Chunk::Repeat { .. } | Chunk::Dcount | Chunk::Computed(_)));
        assert!(!unresolved, "unresolved count operator in {}", e.stroke);
    }
}
