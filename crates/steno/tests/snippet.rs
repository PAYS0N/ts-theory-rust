//! Port of `test/snippet.test.ts`: LSP snippet bodies and the two nvim
//! artifacts.
//!
//! The single-landing `${0}` and escaping tests upstream pointed at `STKWR-LT`,
//! whose corpus meaning changed (now a `let` binding). The renderer is
//! unchanged and still produces the intended snippet for a `` `${%0}` ``
//! template, so this port builds that entry directly. See docs/porting-notes.md.

use steno::{
    Entry, EntryFlags, SENTINEL_CLOSE, SENTINEL_OPEN, TypedEntry, build_snippets, expand_dict,
    parse_source, parse_template, render_snippet,
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

/// The `` `${%0}` `` template literal, built directly (its former stroke was
/// repurposed in the corpus).
fn template_literal() -> Option<TypedEntry> {
    let template = parse_template("`${%0}`", 1).ok()?;
    let source = Entry {
        stroke: vec!["STKWR-LT".to_owned()],
        stroke_raw: "STKWR-LT".to_owned(),
        template: template.clone(),
        raw: "`${%0}`".to_owned(),
        count: None,
        arity: None,
        flags: EntryFlags::default(),
        line: 1,
    };
    Some(TypedEntry {
        stroke: "STKWR-LT".to_owned(),
        template,
        terminal: true,
        one_liner: false,
        count: None,
        source,
    })
}

/// Count `${<digit>}` tabstops in a snippet body.
fn tabstop_count(body: &str) -> usize {
    (0..10u32)
        .map(|d| body.matches(&format!("${{{d}}}")).count())
        .sum()
}

/// A function: landings renumber to tab order, body is the `${0}` exit.
#[test]
fn function_tabstops_renumber() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-PBGS/TPH-FLT").unwrap();
    let s = render_snippet(e).unwrap();
    assert_eq!(s.key_id, "STKWR-PBGS/TPH-FLT");
    assert!(s.terminal);
    assert_eq!(s.body, "function ${1}(${2}): number {\n${0}\\}");
}

/// A single-landing construct puts it at `${0}`.
#[test]
fn single_landing_is_exit() {
    let e = template_literal().unwrap();
    assert_eq!(render_snippet(&e).unwrap().body, "`\\${${0}\\}`");
}

/// The U one-liner keeps the body on one line.
#[test]
fn one_liner_single_line() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWRUPBGS/TPH-FLT").unwrap();
    let body = render_snippet(e).unwrap().body;
    assert!(!body.contains('\n'));
    assert_eq!(body, "function ${1}(${2}): number {${0}\\}");
}

/// A free-type (SKP) leaves a tabstop at the type slot.
#[test]
fn free_type_leaves_tabstop() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-PBGS/SKP-FLT").unwrap();
    let body = render_snippet(e).unwrap().body;
    assert!(body.contains("): ${"), "a tabstop sits where the type goes");
    assert_eq!(tabstop_count(&body), 4, "name, param, type, body");
}

/// Non-terminal partials emit bracket-stripped text with no tabstops.
#[test]
fn non_terminal_partial() {
    let entries = typed().unwrap();
    let e = by_stroke(&entries, "STKWR-PBGS/PR-FLT").unwrap();
    assert!(!e.terminal);
    let body = render_snippet(e).unwrap().body;
    assert_eq!(body, "function : Promise ");
    assert!(!body.contains('$'), "no tabstops in a partial");
}

/// Escaping: literal `$`, `}`, `\` are escaped but tabstops emit raw.
#[test]
fn escapes_literals_not_tabstops() {
    let e = template_literal().unwrap();
    let body = render_snippet(&e).unwrap().body;
    assert!(body.contains("\\$"), "literal $ escaped");
    assert!(body.contains("\\}"), "literal closing brace escaped");
    assert!(body.contains("${0}"), "the tabstop is raw");
}

/// `buildSnippets` produces no keyset collisions.
#[test]
fn build_no_collisions() {
    let entries = typed().unwrap();
    let build = build_snippets(&entries).unwrap();
    assert_eq!(build.collisions, Vec::<String>::new());
}

/// The Plover dict maps a stroke to its sentinel-wrapped token.
#[test]
fn plover_token_wrapped() {
    let entries = typed().unwrap();
    let build = build_snippets(&entries).unwrap();
    let token = build.plover_keys.get("STKWR-PBGS/TPH-FLT").unwrap();
    assert_eq!(
        token,
        format!("{SENTINEL_OPEN}STKWR-PBGS/TPH-FLT{SENTINEL_CLOSE}")
    );
}

/// Every Plover token resolves to a snippet body, and there are over 1000.
#[test]
fn tokens_resolve_to_bodies() {
    let entries = typed().unwrap();
    let build = build_snippets(&entries).unwrap();
    for (_, token) in build.plover_keys.iter() {
        let key = token
            .strip_prefix(SENTINEL_OPEN)
            .and_then(|t| t.strip_suffix(SENTINEL_CLOSE))
            .unwrap();
        assert!(build.snippets.get(key).is_some(), "token {key} has a body");
    }
    assert!(
        build.snippets.len() > 1000,
        "snippets = {}",
        build.snippets.len()
    );
}
