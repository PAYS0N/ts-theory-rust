//! Port of `test/struct.test.ts`: every `@literal` block in the corpus is
//! reproduced by a smart auto-indent editor, and the emitted keystrokes never
//! type a bare closing brace.

use steno::{
    Entry, Event, KeyName, SMART_INDENT, emit_struct, interpret, parse_source, struct_text,
};

/// Read the real dict.steno corpus from the repository root.
fn corpus() -> std::io::Result<String> {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dict.steno"))
}

/// The `@literal` entries of the corpus.
fn literals(src: &str) -> Option<Vec<Entry>> {
    let entries = parse_source(src).ok()?;
    Some(entries.into_iter().filter(|e| e.flags.literal()).collect())
}

/// The buffer an auto-indent editor should end with: tabs -> 4 spaces, and the
/// blank separator lines removed (the v2 emitter skips blanks).
fn intended(raw: &str) -> String {
    raw.split('\n')
        .filter_map(|line| {
            let chars: Vec<char> = line.chars().collect();
            let mut d = 0;
            while chars.get(d) == Some(&'\t') {
                d += 1;
            }
            let rest: String = chars.get(d..).unwrap_or(&[]).iter().collect();
            if rest.trim().is_empty() {
                return None;
            }
            Some(format!("{}{rest}", "    ".repeat(d)))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The corpus contains more than five data-structure (`@literal`) entries.
#[test]
fn found_the_data_structure_entries() {
    let src = corpus().unwrap();
    let structs = literals(&src).unwrap();
    assert!(
        structs.len() > 5,
        "expected >5 @literal entries, got {}",
        structs.len()
    );
}

/// Every `@literal` block is reproduced byte-for-byte under `SMART_INDENT`.
#[test]
fn reproduces_every_literal_block() {
    let src = corpus().unwrap();
    let structs = literals(&src).unwrap();
    for e in &structs {
        let got = interpret(&emit_struct(&e.template), SMART_INDENT).buffer;
        assert_eq!(
            got,
            intended(&struct_text(&e.template)),
            "mismatch reproducing {}",
            e.stroke_raw
        );
    }
}

/// `emitStruct` types opening lines and navigates, never typing a bare closer.
#[test]
fn never_types_a_bare_closing_brace() {
    let src = corpus().unwrap();
    let structs = literals(&src).unwrap();
    let e = structs
        .iter()
        .find(|x| x.stroke_raw == "STKWR-RBGT/S")
        .expect("STKWR-RBGT/S present");
    let events = emit_struct(&e.template);
    for ev in &events {
        if let Event::Text(s) = ev {
            assert_ne!(s, "}", "emitted a bare closing brace");
        }
    }
    assert!(
        events.iter().any(|ev| matches!(
            ev,
            Event::Key {
                key: KeyName::Down,
                ..
            }
        )),
        "expected Down navigation to exit nested blocks"
    );
}
