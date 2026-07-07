//! Port of `test/render.test.ts`: plain-profile rendering and the full plain
//! dictionary build.

use steno::{TypedEntry, build_plain_dict, expand_dict, parse_source, render_plain};

/// Read the real dict.steno corpus from the repository root.
fn corpus() -> std::io::Result<String> {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dict.steno"))
}

/// Expand the whole corpus to typed entries.
fn typed() -> Option<Vec<TypedEntry>> {
    let src = corpus().ok()?;
    let entries = parse_source(&src).ok()?;
    expand_dict(&entries, None).ok()
}

/// Find the typed entry with the given full stroke.
fn by_stroke<'a>(entries: &'a [TypedEntry], stroke: &str) -> Option<&'a TypedEntry> {
    entries.iter().find(|e| e.stroke == stroke)
}

/// A terminal entry emits braces, escapes, and movement back to `%0`.
#[test]
fn terminal_emits_braces_and_movement() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-PBGS/TPH-FLT").unwrap();
    let (key, value) = render_plain(e).unwrap();
    assert_eq!(key, "STKWR-PBGS/TPH-FLT");
    assert_eq!(
        value,
        "{^}function (): number \\{\\n\\}{#Up End Left Left Left Left Left Left Left Left Left Left Left Left}{^}"
    );
}

/// A non-terminal entry drops all bracing and emits no movement.
#[test]
fn non_terminal_drops_bracing() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-PBGS/PH-FLT").unwrap();
    assert!(!e.terminal);
    let (_, value) = render_plain(e).unwrap();
    assert!(!value.contains("{#"), "no movement expected");
    assert!(
        !value.contains(['(', ')', '[', ']', '<', '>']),
        "auto-paired delimiters should be stripped"
    );
}

/// A single-line terminal lands with no `{#Up}` (same line).
#[test]
fn single_line_terminal_no_movement() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-BGS").unwrap();
    let (_, value) = render_plain(e).unwrap();
    assert_eq!(value, "{^}[0]{^}");
}

/// The full plain dict builds with no value collisions and over 1000 keys.
#[test]
fn full_plain_dict() {
    let entries = typed().unwrap();
    let result = build_plain_dict(&entries).unwrap();
    assert_eq!(result.collisions, Vec::<String>::new());
    assert!(result.dict.len() > 1000, "keys = {}", result.dict.len());
}
