//! Port of `test/smart.test.ts`: smart-profile rendering and the full smart
//! dictionary build.
//!
//! The three upstream `STKWR-LT` "template literal" assertions were stale: the
//! corpus repurposed `STKWR-LT` to a `let` binding, so those tests pointed at a
//! fixture that no longer matched. The renderer is unchanged and still produces
//! the intended values for a `` `${%0}` `` template, so this port builds that
//! entry directly via the public API (see `template_literal`) and keeps the
//! original expected outputs. See docs/porting-notes.md.

use steno::{
    Entry, EntryFlags, TypedEntry, build_smart_dict, expand_dict, parse_source, parse_template,
    render_plain, render_smart,
};

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

/// A standalone terminal entry built from a template source (no corpus fixture).
fn terminal_entry(stroke: &str, template_src: &str) -> Option<TypedEntry> {
    let template = parse_template(template_src, 1).ok()?;
    let source = Entry {
        stroke: vec![stroke.to_owned()],
        stroke_raw: stroke.to_owned(),
        template: template.clone(),
        raw: template_src.to_owned(),
        count: None,
        arity: None,
        flags: EntryFlags::default(),
        line: 1,
    };
    Some(TypedEntry {
        stroke: stroke.to_owned(),
        template,
        terminal: true,
        one_liner: false,
        count: None,
        source,
    })
}

/// The `` `${%0}` `` template literal, built directly (its former stroke was
/// repurposed in the corpus).
fn template_literal() -> Option<TypedEntry> {
    terminal_entry("STKWR-LT", "`${%0}`")
}

/// A function drops the trailing `}` (editor auto-closes) and lands on `%0`.
#[test]
fn function_drops_trailing_brace() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-PBGS/TPH-FLT").unwrap();
    let (key, value) = render_smart(e).unwrap();
    assert_eq!(key, "STKWR-PBGS/TPH-FLT");
    assert_eq!(
        value,
        "{^}function (): number \\{\\n{#Up End Left Left Left Left Left Left Left Left Left Left Left Left}{^}"
    );
    assert!(!value.contains("\\}"), "no typed closing brace");
}

/// The interior `)` is kept (type-over); only the trailing closer drops.
#[test]
fn interior_paren_kept() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-PBGS/TPH-FLT").unwrap();
    assert!(render_smart(e).unwrap().1.contains("function ():"));
}

/// The index drops its trailing `]`.
#[test]
fn index_drops_bracket() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-BGS").unwrap();
    assert_eq!(render_smart(e).unwrap().1, "{^}[0{^}");
}

/// Generics keep both angle brackets (`<` is not auto-closed).
#[test]
fn generics_keep_angle_brackets() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-T/PR/STR").unwrap();
    assert_eq!(render_smart(e).unwrap().1, "{^}Promise<string>{^}");
}

/// The template literal lands the cursor inside `${}` with no movement.
#[test]
fn template_literal_cursor_inside() {
    let e = template_literal().unwrap();
    assert_eq!(render_smart(&e).unwrap().1, "{^}`$\\{{^}");
}

/// The U one-liner stays on one line and lands on `%0`.
#[test]
fn one_liner_stays_single_line() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWRUPBGS/TPH-FLT").unwrap();
    let value = render_smart(e).unwrap().1;
    assert!(!value.contains("\\n"));
    assert_eq!(
        value,
        "{^}function (): number \\{{#Left Left Left Left Left Left Left Left Left Left Left Left}{^}"
    );
}

/// Non-terminal entries are byte-identical to plain.
#[test]
fn non_terminal_matches_plain() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-PBGS/PR-FLT").unwrap();
    assert!(!e.terminal);
    assert_eq!(render_smart(e).unwrap().1, render_plain(e).unwrap().1);
}

/// `@literal` data structures use the v2 struct emitter (not verbatim).
#[test]
fn literal_blocks_use_struct_emitter() {
    let entries = typed().unwrap();
    for s in ["STKWR-RBGT/S", "STKWR-RBGT/HR"] {
        let e = by_stroke(&entries, s).unwrap();
        let smart = render_smart(e).unwrap().1;
        let plain = render_plain(e).unwrap().1;
        assert_ne!(smart, plain);
        assert!(plain.contains("\\}"), "plain types every closing brace");
        assert!(!smart.contains("\\}"), "smart lets the editor supply them");
        assert!(smart.contains("{#Down"), "and navigates around them");
    }
}

/// A bracket-free construct (ternary) is identical to plain.
#[test]
fn bracket_free_matches_plain() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-RPBT").unwrap();
    assert_eq!(render_smart(e).unwrap().1, render_plain(e).unwrap().1);
}

/// The full smart dict builds with no collisions and over 1000 keys.
#[test]
fn full_smart_dict() {
    let entries = typed().unwrap();
    let result = build_smart_dict(&entries).unwrap();
    assert_eq!(result.collisions, Vec::<String>::new());
    assert!(result.dict.len() > 1000, "keys = {}", result.dict.len());
}
