//! The programmatic (`dict.infinite.steno`) path: table construction, the
//! obligation-stack reference walker, fuse inversion, and the two-sided
//! differential pin against `expand_dict` on the enumerable subset (D9).

use steno::{
    Chunk, Construct, Entry, InfType, Profile, build_tables, check_fuse_ambiguity, expand_dict,
    parse_source, render_plain, render_walk, walk,
};

/// Flatten a resolved walk template to plain text for test readability:
/// landings drop out (there is no board cursor in a flat string) — the real
/// render/snippet code is what actually consumes them, and that is what
/// `render_walk` in the differential test below pins.
fn flat_text(template: &[Chunk]) -> String {
    template
        .iter()
        .map(|c| match c {
            Chunk::Lit(s) => s.clone(),
            Chunk::Brace { open: true } => "{".to_owned(),
            Chunk::Brace { open: false } => "}".to_owned(),
            Chunk::Newline | Chunk::BodyBreak => "\n".to_owned(),
            Chunk::Tab => "\t".to_owned(),
            _ => String::new(),
        })
        .collect()
}

/// Load and parse the real programmatic corpus from the repo root.
fn corpus() -> Option<Vec<Entry>> {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../dict.infinite.steno"
    ))
    .ok()?;
    parse_source(&src).ok()
}

/// The two tables built from the real corpus.
fn tables() -> Option<(Vec<InfType>, Vec<Construct>)> {
    build_tables(&corpus()?).ok()
}

/// Split a `/`-joined stroke into owned segments.
fn seg(stroke: &str) -> Vec<String> {
    stroke.split('/').map(str::to_owned).collect()
}

#[test]
fn corpus_builds_two_tables_no_pass_b() {
    let (types, constructs) = tables().unwrap();
    // 21 @type records; 10 STKWR-PBGS families (base + 9 named variants) at
    // 16 count members each, plus the type emitter.
    assert_eq!(types.len(), 21);
    assert_eq!(
        constructs.len(),
        161,
        "10 families * 16 count members + 1 type emitter"
    );
    check_fuse_ambiguity(&types, &constructs).expect("real corpus is unambiguous");
}

#[test]
fn walker_deep_nesting_is_unbounded() {
    let (types, constructs) = tables().unwrap();
    // Depth 3: never enumerable by Pass B, terminal by the obligation stack.
    // Array<%t> is stroke "R" (renamed from "AR" when the corpus grew its
    // type table).
    let r = walk(&seg("STKWR-T/R/R/R/TPH"), &types, &constructs).expect("valid");
    assert_eq!(flat_text(&r.template), "Array<Array<Array<number>>>");
    assert!(r.terminal);
}

#[test]
fn walker_partial_generic_is_non_terminal() {
    let (types, constructs) = tables().unwrap();
    let r = walk(&seg("STKWR-T/R/R"), &types, &constructs).expect("valid prefix");
    assert_eq!(flat_text(&r.template), "Array Array", "bracketless partial");
    assert!(!r.terminal, "stack depth 2 → non-terminal");
}

#[test]
fn terminal_walk_flattens_to_full_text() {
    let (types, constructs) = tables().unwrap();
    let done = walk(&seg("STKWR-T/R/R/R/TPH"), &types, &constructs).expect("valid");
    assert_eq!(flat_text(&done.template), "Array<Array<Array<number>>>");
}

#[test]
fn non_terminal_plain_strips_brackets_and_newlines() {
    let (types, constructs) = tables().unwrap();
    // Fused return type consumed (count=2 member, shape "-FLT" — the corpus's
    // ###deFauLT variant, renamed from the old bare "-L" shape), no params
    // stroked yet: still non-terminal, but the programmatic dict must show
    // something for it — the same "in progress" convention the enumerated
    // dict.steno path uses.
    let partial = walk(&seg("STKWR-PBGS/TPHOFLT"), &types, &constructs).expect("valid prefix");
    assert!(!partial.terminal);
    let plain = render_walk(&partial, Profile::Plain).expect("plain");
    // `{^}`/`{^ ^}` are Plover's own cursor/glue control syntax, unconditionally
    // wrapped around every non-terminal Plain render (ADR-1/2) — not type
    // syntax, so they're excluded from the leaked-bracket check below.
    let type_syntax = plain.replace("{^}", "").replace("{^ ^}", "");
    assert!(
        !type_syntax.contains(['(', ')', '[', ']', '<', '>', '{', '}']),
        "type brackets stripped: {plain}"
    );
    assert!(!plain.contains('\n'), "no newlines: {plain}");
}

#[test]
fn walker_map_arity_two() {
    let (types, constructs) = tables().unwrap();
    let done = walk(&seg("STKWR-T/PH/TPH/STR"), &types, &constructs).expect("valid");
    assert_eq!(flat_text(&done.template), "Map<number, string>");
    assert!(done.terminal);
    let partial = walk(&seg("STKWR-T/PH/TPH"), &types, &constructs).expect("valid");
    assert_eq!(flat_text(&partial.template), "Map number");
    assert!(!partial.terminal);
}

#[test]
fn walker_fused_function_family_wide() {
    let (types, constructs) = tables().unwrap();
    // count=2 (bit O): shape OFLT (the ###deFauLT variant, renamed from the
    // old bare "-L" shape); return number fused → TPHOFLT; params number,
    // string. The %(d+1) param label is now an invisible editor landing
    // rather than a literal "arg%d" prefix, so filled params render as bare
    // ": <type>" — this is an intended template change, not a regression.
    let r = walk(&seg("STKWR-PBGS/TPHOFLT/TPH/STR"), &types, &constructs).expect("valid");
    assert_eq!(
        flat_text(&r.template),
        "function (: number, : string): number {\n}"
    );
    assert!(r.terminal);
}

#[test]
fn walker_fused_zero_params() {
    let (types, constructs) = tables().unwrap();
    // count=0: shape -FLT; return string fused → STR-FLT; no params.
    let r = walk(&seg("STKWR-PBGS/STR-FLT"), &types, &constructs).expect("valid");
    assert_eq!(flat_text(&r.template), "function (): string {\n}");
    assert!(r.terminal);
}

#[test]
fn walker_fused_return_generic() {
    let (types, constructs) = tables().unwrap();
    // count=0, return Array<number>: R (renamed from AR) fused into -FLT →
    // R-FLT, then TPH fills the generic arg.
    let r = walk(&seg("STKWR-PBGS/R-FLT/TPH"), &types, &constructs).expect("valid");
    assert_eq!(flat_text(&r.template), "function (): Array<number> {\n}");
    assert!(r.terminal);
    // Missing the generic arg: non-terminal.
    let partial = walk(&seg("STKWR-PBGS/R-FLT"), &types, &constructs).expect("valid prefix");
    assert!(!partial.terminal);
}

#[test]
fn walker_negative_unmatched_base_and_extra_strokes() {
    let (types, constructs) = tables().unwrap();
    assert!(
        walk(&seg("STKPWHR"), &types, &constructs).is_none(),
        "no base"
    );
    // A completed emitter with an extra dangling type stroke: invalid.
    assert!(
        walk(&seg("STKWR-T/TPH/STR"), &types, &constructs).is_none(),
        "extra stroke after a filled single slot"
    );
    // A stroke that is not a type in the table: invalid. STKPWHR itself is
    // now the "any" type (the corpus grew its type table), so a fake stroke
    // that collides with no real chord is used instead.
    assert!(walk(&seg("STKWR-T/ZKPWHR"), &types, &constructs).is_none());
}

#[test]
fn fuse_ambiguity_check_fires() {
    // Two shapes/types that fuse to the same stroke: TPH+(-L) == TP+(H-L).
    let src = "\
````TP
custom
````
@type
````TPH
number
````
@type
````TKPW/-L
: %T
````
@fuse
````TKPW/H-L
: %T
````
@fuse
";
    let entries = parse_source(src).expect("parse");
    let (types, constructs) = build_tables(&entries).expect("tables");
    let err = check_fuse_ambiguity(&types, &constructs).expect_err("must detect the collision");
    assert!(format!("{err}").contains("fuse ambiguity"), "{err}");
}

/// The differential subset: the type emitter plus its append set. Pass B can
/// enumerate this (generic args draw from the arity-0 pool, so nesting is
/// shallow), which pins the walker to established semantics.
fn emitter_subset() -> Option<Vec<Entry>> {
    Some(
        corpus()?
            .into_iter()
            .filter(|e| e.flags.is_type() || e.stroke_raw == "STKWR-T")
            .collect(),
    )
}

#[test]
fn walker_matches_expand_dict_on_enumerable_subset() {
    let subset = emitter_subset().unwrap();
    let (types, constructs) = build_tables(&subset).expect("tables");
    let rows = expand_dict(&subset, None).expect("expand_dict");
    assert!(rows.len() > 10, "subset should enumerate many rows");
    for te in &rows {
        let strokes = seg(&te.stroke);
        let w = walk(&strokes, &types, &constructs).expect("walker rejected an enumerable stroke");
        assert_eq!(
            w.terminal, te.terminal,
            "terminal mismatch on {}",
            te.stroke
        );
        let from_walk = render_walk(&w, Profile::Plain).expect("walk plain");
        let from_static = render_plain(te).expect("static plain").1;
        assert_eq!(
            from_walk, from_static,
            "plain render mismatch on {}",
            te.stroke
        );
    }
}
