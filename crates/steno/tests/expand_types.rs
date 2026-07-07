//! Port of `test/types.test.ts`: Pass B type-append chains, arity rules,
//! and the full-corpus Pass A + B expansion.

use std::collections::HashMap;

use steno::{
    Chunk, Entry, TypeDef, TypeOptions, TypedEntry, build_type_set, expand_counts, expand_dict,
    expand_types_one, parse_source,
};

/// Render chunks for assertions; an unfilled `%t` is called out.
fn show(chunks: &[Chunk]) -> String {
    chunks
        .iter()
        .map(|c| match c {
            Chunk::Lit(t) => t.clone(),
            Chunk::Landing(n) => format!("%{n}"),
            Chunk::Brace { open } => (if *open { "{" } else { "}" }).to_string(),
            Chunk::BodyBreak => "%b".to_string(),
            Chunk::TypeSlot => "<UNFILLED %t>".to_string(),
            other => format!("<{}>", other.kind_name()),
        })
        .collect()
}

/// A concrete arity-0 type def.
fn def(stroke: &str, arity: u32, text: &str) -> TypeDef {
    TypeDef {
        stroke: stroke.to_string(),
        arity,
        text: text.to_string(),
        free_type: false,
    }
}

/// The fixed test type set: number, string, Promise, Map.
fn opts() -> TypeOptions {
    let number = def("TPH", 0, "number");
    let string = def("STR", 0, "string");
    let promise = def("PR", 1, "Promise<%t>");
    let map = def("PH", 2, "Map<%t, %t>");
    TypeOptions {
        types: vec![number.clone(), string.clone(), promise, map],
        generic_args: vec![number, string],
    }
}

/// Expand a simple %t-bearing construct (no count) into its chain.
fn construct(template: &str) -> Option<Vec<TypedEntry>> {
    let src = format!("````X\n{template}\n````");
    let mut entries = parse_source(&src).ok()?;
    let entry = entries.pop()?;
    let expanded = expand_counts(&entry).ok()?;
    let first = expanded.first()?;
    expand_types_one(first, &opts()).ok()
}

/// Index a chain by stroke.
fn by_stroke(chain: &[TypedEntry]) -> HashMap<&str, &TypedEntry> {
    chain.iter().map(|e| (e.stroke.as_str(), e)).collect()
}

/// Look up a stroke that must exist in the chain.
fn step<'a>(idx: &HashMap<&str, &'a TypedEntry>, k: &str) -> Option<&'a TypedEntry> {
    idx.get(k).copied()
}

/// The chain emits a non-terminal base before any type is appended.
#[test]
fn chain_has_non_terminal_base() {
    let chain = construct("let a: %t = b").unwrap();
    let idx = by_stroke(&chain);
    let base = step(&idx, "X").unwrap();
    assert!(!base.terminal);
    assert_eq!(show(&base.template), "let a:  = b");
}

/// Arity-0 types terminate immediately.
#[test]
fn arity_zero_terminates() {
    let chain = construct("let a: %t = b").unwrap();
    let idx = by_stroke(&chain);
    let str_step = step(&idx, "X/STR").unwrap();
    assert!(str_step.terminal);
    assert_eq!(show(&str_step.template), "let a: string = b");
}

/// Arity-1 generic: non-terminal head, then terminal with the arg bracketed.
#[test]
fn arity_one_generic_chain() {
    let chain = construct("let a: %t = b").unwrap();
    let idx = by_stroke(&chain);
    let head = step(&idx, "X/PR").unwrap();
    assert!(!head.terminal);
    assert_eq!(show(&head.template), "let a: Promise = b");
    let full = step(&idx, "X/PR/STR").unwrap();
    assert!(full.terminal);
    assert_eq!(show(&full.template), "let a: Promise<string> = b");
}

/// Arity-2 generic: head and one-arg steps are non-terminal (bracketless).
#[test]
fn arity_two_generic_chain() {
    let chain = construct("let a: %t = b").unwrap();
    let idx = by_stroke(&chain);

    let head = step(&idx, "X/PH").unwrap();
    assert!(!head.terminal);
    assert_eq!(show(&head.template), "let a: Map = b");

    let one_arg = step(&idx, "X/PH/STR").unwrap();
    assert!(!one_arg.terminal);
    assert_eq!(show(&one_arg.template), "let a: Map string = b");

    let full = step(&idx, "X/PH/STR/TPH").unwrap();
    assert!(full.terminal);
    assert_eq!(show(&full.template), "let a: Map<string, number> = b");
}

/// Chain size: base + 2 arity-0 + Promise(1+2) + Map(1+2+4); strokes unique.
#[test]
fn chain_count_and_uniqueness() {
    let chain = construct("let a: %t = b").unwrap();
    assert_eq!(chain.len(), 1 + 2 + 3 + 7);
    let strokes: std::collections::HashSet<&str> =
        chain.iter().map(|e| e.stroke.as_str()).collect();
    assert_eq!(strokes.len(), chain.len());
}

/// A construct without %t yields one terminal entry.
#[test]
fn no_typeslot_passes_through() {
    let chain = construct("console.log(%0)").unwrap();
    assert_eq!(chain.len(), 1);
    assert!(chain.first().unwrap().terminal);
}

/// The construct wrapper nests one appended generic
/// (`Promise<Map<string, number>>`).
#[test]
fn construct_wrapper_nests_appended_generic() {
    let chain = construct("(): Promise<%t> => {}").unwrap();
    let idx = by_stroke(&chain);
    assert_eq!(
        show(&step(&idx, "X/PH/STR/TPH").unwrap().template),
        "(): Promise<Map<string, number>> => {}"
    );
}

/// Read and parse the real dict.steno corpus.
fn corpus_entries() -> Option<Vec<Entry>> {
    let text =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dict.steno")).ok()?;
    parse_source(&text).ok()
}

/// The corpus type set has the expected arities and culls @noarg types.
#[test]
fn corpus_type_set() {
    let set = build_type_set(&corpus_entries().unwrap()).unwrap();
    let map = set.types.iter().find(|t| t.stroke == "PH").unwrap();
    assert_eq!(map.arity, 2);
    assert_eq!(map.text, "Map<%t, %t>");
    assert!(!map.free_type);
    // 14 arity-0 types minus the 4 @noarg ones (null/undefined/never/function).
    assert_eq!(set.arity0.len(), 10);
    assert!(!set.arity0.iter().any(|t| t.stroke == "STPHR")); // never culled
}

/// The corpus expands end-to-end through Pass A + B.
#[test]
fn corpus_expands_end_to_end() {
    let all = expand_dict(&corpus_entries().unwrap(), None).unwrap();
    assert!(all.len() > 1000);
}

/// A deep terminal type entry is present and correct (shape fused into Map).
#[test]
fn corpus_deep_terminal_type_entry() {
    let all = expand_dict(&corpus_entries().unwrap(), None).unwrap();
    let e = all
        .iter()
        .find(|x| x.stroke == "STKWR-PBGS/PH-FLT/STR/TPH")
        .unwrap();
    assert!(e.terminal);
    assert_eq!(
        show(&e.template),
        "function %0(%1): Map<string, number> {%b%2}"
    );
}

/// Free-type (SKP) keeps the colon, tabstops the type, leaves %0 on the name.
#[test]
fn corpus_free_type_tabstop() {
    let all = expand_dict(&corpus_entries().unwrap(), None).unwrap();
    let e = all
        .iter()
        .find(|x| x.stroke == "STKWR-PBGS/SKP-FLT")
        .unwrap();
    assert!(e.terminal);
    // type slot is %3, name still %0
    assert_eq!(show(&e.template), "function %0(%1): %3 {%b%2}");
}
