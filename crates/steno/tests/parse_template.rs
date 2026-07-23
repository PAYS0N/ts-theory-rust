//! Port of `test/parse.test.ts` (template half): primitives, repeats,
//! computed expressions, and error cases for `parse_template`.

use steno::{Chunk, Expr, parse_template};

/// Shorthand for a literal chunk.
fn lit(s: &str) -> Chunk {
    Chunk::Lit(s.to_string())
}

/// A plain literal parses to a single lit chunk.
#[test]
fn plain_literal() {
    assert_eq!(
        parse_template("console.log(x)", 1).unwrap(),
        vec![lit("console.log(x)")]
    );
}

/// Landings, count, type-slot, body-break, and pattern all parse.
#[test]
fn landings_count_typeslot_bodybreak_pattern() {
    assert_eq!(
        parse_template("%0%d%t%b%p", 1).unwrap(),
        vec![
            Chunk::Landing(0),
            Chunk::Dcount,
            Chunk::TypeSlot,
            Chunk::BodyBreak,
            Chunk::Pattern,
        ]
    );
}

/// %12 is %1 then literal "2" — landings are single digit by design.
#[test]
fn multi_digit_landings_split() {
    assert_eq!(
        parse_template("%12", 1).unwrap(),
        vec![Chunk::Landing(1), lit("2")]
    );
}

/// Structural braces become brace chunks; %b stays separate.
#[test]
fn structural_braces_become_brace_chunks() {
    assert_eq!(
        parse_template("{%b}", 1).unwrap(),
        vec![
            Chunk::Brace { open: true },
            Chunk::BodyBreak,
            Chunk::Brace { open: false },
        ]
    );
}

/// Escaped brace/percent/backslash/backtick become literals; the n and t
/// escapes become chunks.
#[test]
fn escapes() {
    assert_eq!(
        parse_template("\\{\\}\\%\\\\\\`", 1).unwrap(),
        vec![lit("{}%\\`")]
    );
    assert_eq!(
        parse_template("a\\nb\\tc", 1).unwrap(),
        vec![lit("a"), Chunk::Newline, lit("b"), Chunk::Tab, lit("c")]
    );
}

/// Repeat with `sep | body` form (param list).
#[test]
fn repeat_sep_body_form() {
    assert_eq!(
        parse_template("(%[, |%d%])", 1).unwrap(),
        vec![
            lit("("),
            Chunk::Repeat {
                sep: vec![lit(", ")],
                body: vec![Chunk::Dcount],
            },
            lit(")"),
        ]
    );
}

/// Repeat without a pipe has an empty separator.
#[test]
fn repeat_no_pipe_form() {
    assert_eq!(
        parse_template("%[case %(2d-1):\\nbreak;\\n%]", 1).unwrap(),
        vec![Chunk::Repeat {
            sep: vec![],
            body: vec![
                lit("case "),
                Chunk::Computed(Expr { a: 2, b: -1 }),
                lit(":"),
                Chunk::Newline,
                lit("break;"),
                Chunk::Newline,
            ],
        }]
    );
}

/// A top-level `|` outside a repeat is a literal.
#[test]
fn top_level_pipe_is_literal() {
    assert_eq!(
        parse_template("T | undefined", 1).unwrap(),
        vec![lit("T | undefined")]
    );
}

/// Repeats nest.
#[test]
fn nested_repeats() {
    assert_eq!(
        parse_template("%[; |%[, |%p%]%]", 1).unwrap(),
        vec![Chunk::Repeat {
            sep: vec![lit("; ")],
            body: vec![Chunk::Repeat {
                sep: vec![lit(", ")],
                body: vec![Chunk::Pattern],
            }],
        }]
    );
}

/// Computed expressions parse in every supported linear form.
#[test]
fn computed_expressions() {
    let cases = [
        ("%(d)", Expr { a: 1, b: 0 }),
        ("%(2d)", Expr { a: 2, b: 0 }),
        ("%(2d-1)", Expr { a: 2, b: -1 }),
        ("%(2d+1)", Expr { a: 2, b: 1 }),
        ("%(-d)", Expr { a: -1, b: 0 }),
        ("%(5)", Expr { a: 0, b: 5 }),
        ("%( 2d - 1 )", Expr { a: 2, b: -1 }),
    ];
    for (src, expr) in cases {
        assert_eq!(
            parse_template(src, 1).unwrap(),
            vec![Chunk::Computed(expr)],
            "template {src}"
        );
    }
}

/// Malformed templates report the offending construct.
#[test]
fn template_errors() {
    let cases = [
        ("%z", "unknown operator %z"),
        ("abc%", "trailing '%'"),
        ("a\\", "trailing backslash"),
        ("x\\q", "unknown escape \\q"),
        ("%[a", "unterminated %["),
        ("%(2d", "unterminated %("),
        ("%(zz)", "bad computed expression"),
    ];
    for (src, needle) in cases {
        let err = parse_template(src, 1).unwrap_err();
        assert!(
            err.to_string().contains(needle),
            "template {src}: expected {needle:?} in {err}"
        );
    }
}

/// `%<N>` end landings parse into `Chunk::EndLanding`.
#[test]
fn end_landings() {
    assert_eq!(
        parse_template("%<0>", 1).unwrap(),
        vec![Chunk::EndLanding(0)]
    );
    assert_eq!(
        parse_template("%<12>", 1).unwrap(),
        vec![Chunk::EndLanding(12)]
    );
    assert_eq!(
        parse_template("{%<0>}", 1).unwrap(),
        vec![
            Chunk::Brace { open: true },
            Chunk::EndLanding(0),
            Chunk::Brace { open: false },
        ]
    );
}

/// Malformed end landings report the offending construct.
#[test]
fn end_landing_errors() {
    let cases = [
        ("%<", "unterminated %< ... >"),
        ("%<>", "empty end landing"),
        ("%<abc>", "bad end landing"),
        ("%<3", "unterminated %< ... >"),
    ];
    for (src, needle) in cases {
        let err = parse_template(src, 1).unwrap_err();
        assert!(
            err.to_string().contains(needle),
            "template {src}: expected {needle:?} in {err}"
        );
    }
}
