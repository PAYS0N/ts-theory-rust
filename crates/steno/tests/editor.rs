//! Port of `test/editor.test.ts`: the editor simulation under each profile,
//! plus serialization and movement compilation.

use steno::{
    Behaviors, Event, KeyName, PLAIN, SMART, SMART_INDENT, interpret, movement_events, serialize,
};

/// A single literal-text event.
fn type_text(s: &str) -> Vec<Event> {
    vec![Event::Text(s.to_owned())]
}

/// A repeated special-key event.
const fn key(k: KeyName, n: usize) -> Event {
    Event::Key { key: k, n }
}

/// Render an editor state as "buffer" with a `<>` cursor marker.
fn show(events: &[Event], b: Behaviors) -> String {
    let s = interpret(events, b);
    let chars: Vec<char> = s.buffer.chars().collect();
    let head: String = chars.get(..s.rest).unwrap_or(&[]).iter().collect();
    let tail: String = chars.get(s.rest..).unwrap_or(&[]).iter().collect();
    format!("{head}<>{tail}")
}

/// PLAIN types everything literally, with no auto-close.
#[test]
fn plain_types_literally() {
    assert_eq!(show(&type_text("a(b)c"), PLAIN), "a(b)c<>");
}

/// PLAIN Enter just inserts a newline at the cursor.
#[test]
fn plain_enter_is_bare_newline() {
    let evs = vec![
        Event::Text("a".to_owned()),
        key(KeyName::Enter, 1),
        Event::Text("b".to_owned()),
    ];
    assert_eq!(show(&evs, PLAIN), "a\nb<>");
}

/// SMART auto-closes an opener and sits between the pair.
#[test]
fn smart_auto_closes_and_sits_between() {
    assert_eq!(show(&type_text("("), SMART), "(<>)");
}

/// SMART types over a closer the editor already supplied.
#[test]
fn smart_types_over_closer() {
    assert_eq!(show(&type_text("()"), SMART), "()<>");
}

/// SMART nests auto-closers, then types over them.
#[test]
fn smart_nests_and_types_over() {
    assert_eq!(show(&type_text("foo(("), SMART), "foo((<>))");
    assert_eq!(show(&type_text("foo(()"), SMART), "foo(()<>)");
}

/// SMART without auto-indent: Enter is a bare newline (no expansion).
#[test]
fn smart_enter_without_indent() {
    let evs = vec![Event::Text("{".to_owned()), key(KeyName::Enter, 1)];
    assert_eq!(show(&evs, SMART), "{\n<>}");
}

/// SMART does not auto-close `<`.
#[test]
fn smart_leaves_angle_bracket() {
    assert_eq!(show(&type_text("Promise<"), SMART), "Promise<<>");
}

/// `SMART_INDENT` block-expands with an indented body and dedented close.
#[test]
fn smart_indent_block_expands() {
    let evs = vec![Event::Text("{".to_owned()), key(KeyName::Enter, 1)];
    assert_eq!(show(&evs, SMART_INDENT), "{\n    <>\n}");
}

/// `SMART_INDENT` keeps the same indent for a sibling Enter.
#[test]
fn smart_indent_sibling_keeps_level() {
    let evs = vec![
        Event::Text("{".to_owned()),
        key(KeyName::Enter, 1),
        Event::Text("a;".to_owned()),
        key(KeyName::Enter, 1),
    ];
    assert_eq!(show(&evs, SMART_INDENT), "{\n    a;\n    <>\n}");
}

/// `SMART_INDENT` Backspace removes one whole indent level.
#[test]
fn smart_indent_backspace_dedents() {
    let evs = vec![
        Event::Text("{".to_owned()),
        key(KeyName::Enter, 1),
        key(KeyName::BackSpace, 1),
    ];
    assert_eq!(show(&evs, SMART_INDENT), "{\n<>\n}");
}

/// A mark never splits a text span (the space before `%0(` stays literal).
#[test]
fn serialize_mark_keeps_span() {
    let evs = vec![
        Event::Text("function ".to_owned()),
        Event::Mark(0),
        Event::Text("()".to_owned()),
    ];
    assert_eq!(serialize(&evs), "function ()");
}

/// A trailing space (before a key) is protected.
#[test]
fn serialize_protects_trailing_space() {
    let evs = vec![Event::Text("x ".to_owned()), key(KeyName::Left, 1)];
    assert_eq!(serialize(&evs), "x{^ ^}{#Left}");
}

/// Enter serializes as `\n` and breaks `{#...}` groups.
#[test]
fn serialize_enter_breaks_groups() {
    let evs = vec![
        Event::Text("a".to_owned()),
        key(KeyName::Enter, 1),
        key(KeyName::Up, 2),
        key(KeyName::End, 1),
    ];
    assert_eq!(serialize(&evs), "a\\n{#Up Up End}");
}

/// Same-line movement is a straight `Left`.
#[test]
fn movement_same_line() -> Result<(), steno::RenderError> {
    assert_eq!(
        serialize(&movement_events("abcdef", 6, 2)?),
        "{#Left Left Left Left}"
    );
    Ok(())
}

/// Cross-line movement is `Up` then `End` then `Left`.
#[test]
fn movement_cross_line() -> Result<(), steno::RenderError> {
    assert_eq!(
        serialize(&movement_events("abc\nde", 6, 1)?),
        "{#Up End Left Left}"
    );
    Ok(())
}

/// No movement when `from == to`.
#[test]
fn movement_none_when_equal() -> Result<(), steno::RenderError> {
    assert_eq!(movement_events("abc", 2, 2)?, Vec::new());
    Ok(())
}
