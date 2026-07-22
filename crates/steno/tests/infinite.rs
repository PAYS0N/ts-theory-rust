//! The programmatic (`dict.infinite.steno`) path: table construction, the
//! obligation-stack reference walker, fuse inversion, and the two-sided
//! differential pin against `expand_dict` on the enumerable subset (D9).

use steno::{
    Construct, Entry, InfType, build_tables, check_fuse_ambiguity, expand_dict, parse_source,
    render_filled, walk,
};

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
    // Five @type records; the AOEU family is 16 constructs plus the emitter.
    assert_eq!(types.len(), 5);
    assert_eq!(constructs.len(), 17, "16 count members + 1 type emitter");
    check_fuse_ambiguity(&types, &constructs).expect("real corpus is unambiguous");
}

#[test]
fn walker_deep_nesting_is_unbounded() {
    let (types, constructs) = tables().unwrap();
    // Depth 3: never enumerable by Pass B, terminal by the obligation stack.
    let r = walk(&seg("STKWR-T/AR/AR/AR/TPH"), &types, &constructs).expect("valid");
    assert_eq!(r.text, "Array<Array<Array<number>>>");
    assert!(r.terminal);
}

#[test]
fn walker_partial_generic_is_non_terminal() {
    let (types, constructs) = tables().unwrap();
    let r = walk(&seg("STKWR-T/AR/AR"), &types, &constructs).expect("valid prefix");
    assert_eq!(r.text, "Array Array", "bracketless partial");
    assert!(!r.terminal, "stack depth 2 → non-terminal");
}

#[test]
fn walker_map_arity_two() {
    let (types, constructs) = tables().unwrap();
    let done = walk(&seg("STKWR-T/PH/TPH/STR"), &types, &constructs).expect("valid");
    assert_eq!(done.text, "Map<number, string>");
    assert!(done.terminal);
    let partial = walk(&seg("STKWR-T/PH/TPH"), &types, &constructs).expect("valid");
    assert_eq!(partial.text, "Map number");
    assert!(!partial.terminal);
}

#[test]
fn walker_fused_function_family_wide() {
    let (types, constructs) = tables().unwrap();
    // count=2 (bit O): shape OL; return number fused → TPHOL; params number, string.
    let r = walk(&seg("STKWR-PBGS/TPHOL/TPH/STR"), &types, &constructs).expect("valid");
    assert_eq!(r.text, "function (arg0: number, arg1: string): number {}");
    assert!(r.terminal);
}

#[test]
fn walker_fused_zero_params() {
    let (types, constructs) = tables().unwrap();
    // count=0: shape -L; return string fused → STR-L; no params.
    let r = walk(&seg("STKWR-PBGS/STR-L"), &types, &constructs).expect("valid");
    assert_eq!(r.text, "function (): string {}");
    assert!(r.terminal);
}

#[test]
fn walker_fused_return_generic() {
    let (types, constructs) = tables().unwrap();
    // count=0, return Array<number>: AR fused into -L → ARL, then TPH fills arg.
    let r = walk(&seg("STKWR-PBGS/ARL/TPH"), &types, &constructs).expect("valid");
    assert_eq!(r.text, "function (): Array<number> {}");
    assert!(r.terminal);
    // Missing the generic arg: non-terminal.
    let partial = walk(&seg("STKWR-PBGS/ARL"), &types, &constructs).expect("valid prefix");
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
    // A stroke that is not a type in the table: invalid.
    assert!(walk(&seg("STKWR-T/STKPWHR"), &types, &constructs).is_none());
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
        let expected = render_filled(&te.template, &[]);
        assert_eq!(w.text, expected, "text mismatch on {}", te.stroke);
        assert_eq!(
            w.terminal, te.terminal,
            "terminal mismatch on {}",
            te.stroke
        );
    }
}
