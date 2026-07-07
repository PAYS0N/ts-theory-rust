//! Port of `test/steno.test.ts`: stroke parse/render round-trips, count-bank
//! weighting, and count application.

use steno::{Side, apply_count, count_bank, parse_stroke, render_stroke};

/// A representative spread of real sub-strokes from dict.steno round-trips
/// through parse + render unchanged.
#[test]
fn parse_render_round_trips() {
    let cases = [
        "STKWR",      // left only
        "STKWR-PBGS", // left + right (hyphen)
        "-FLT",       // right only (leading hyphen)
        "STKWR*F",    // left + star + right
        "STKWR*PLT",  // left + star + right
        "TH-R",       // left + right (hyphen)
        "TPH",        // left-only type stroke
        "SKWR",       // left-only type stroke
        "PHAP",       // left + vowel + right
        "TPEURLT",    // left + 2 vowels + right
        "RAOUS",      // left + 3 vowels + right
        "EFR",        // vowel + right (no left)
        "SORT",       // left + vowel + right
        "STKWR-FP",   // switch base
        "STKWR-BGS",  // index base
    ];
    for s in cases {
        assert_eq!(render_stroke(&parse_stroke(s).unwrap()), s);
    }
}

/// Garbage keys and double hyphens are rejected.
#[test]
fn rejects_garbage_keys() {
    assert!(parse_stroke("XYZ").is_err());
    let err = parse_stroke("ST-K-R").unwrap_err();
    assert!(err.to_string().contains("two hyphens"));
}

/// Flatten a bank's bits to comparable tuples.
fn bits_of(spec: &str) -> Result<Vec<(char, u32, Side)>, steno::StrokeError> {
    Ok(count_bank(spec)?
        .bits
        .iter()
        .map(|b| (b.key, b.weight, b.side))
        .collect())
}

/// A vowel count bank weights keys LSB-first; max is `2^width - 1`.
#[test]
fn count_bank_weights_vowels_lsb_first() {
    assert_eq!(
        bits_of("AOEU").unwrap(),
        vec![
            ('A', 1, Side::Mid),
            ('O', 2, Side::Mid),
            ('E', 4, Side::Mid),
            ('U', 8, Side::Mid),
        ]
    );
    assert_eq!(count_bank("AOEU").unwrap().max, 15);
}

/// A right-bank count bank weights keys LSB-first on the right side.
#[test]
fn count_bank_weights_right_bank_lsb_first() {
    assert_eq!(
        bits_of("RBGS").unwrap(),
        vec![
            ('R', 1, Side::Right),
            ('B', 2, Side::Right),
            ('G', 4, Side::Right),
            ('S', 8, Side::Right),
        ]
    );
}

/// A key outside the vowel/right banks is rejected in an `@count` spec.
#[test]
fn count_bank_rejects_non_vowel_right_key() {
    let err = count_bank("AOK").unwrap_err();
    assert!(err.to_string().contains("not a vowel or right-bank key"));
}

/// Vowel counts merge into a function terminal stroke.
#[test]
fn apply_count_merges_vowel_counts() {
    // -FLT + count 3 (A=1 | O=2) -> vowels AO + right FLT -> AOFLT
    assert_eq!(apply_count("-FLT", "AOEU", 3).unwrap(), "AOFLT");
    // count 0 adds nothing
    assert_eq!(apply_count("-FLT", "AOEU", 0).unwrap(), "-FLT");
    // U=8
    assert_eq!(apply_count("-FLT", "AOEU", 8).unwrap(), "UFLT");
}

/// Right-bank counts merge into the switch base.
#[test]
fn apply_count_merges_right_bank_counts() {
    // STKWR-FP + RBGS, count 15 -> right F,P + R,B,G,S -> F R P B G S
    assert_eq!(apply_count("STKWR-FP", "RBGS", 15).unwrap(), "STKWR-FRPBGS");
    // +R
    assert_eq!(apply_count("STKWR-FP", "RBGS", 1).unwrap(), "STKWR-FRP");
}

/// Right-bank counts merge into the index base.
#[test]
fn apply_count_merges_index_base_counts() {
    // STKWR-BGS + FPLT, count 15 -> F,P,L,T + B,G,S -> F P B L G S T
    assert_eq!(
        apply_count("STKWR-BGS", "FPLT", 15).unwrap(),
        "STKWR-FPBLGTS"
    );
    assert_eq!(apply_count("STKWR-BGS", "FPLT", 0).unwrap(), "STKWR-BGS");
}

/// Out-of-range counts and key collisions are rejected.
#[test]
fn apply_count_rejects_range_and_collisions() {
    let range_err = apply_count("-FLT", "AOEU", 16).unwrap_err();
    assert!(range_err.to_string().contains("out of range"));
    // F is already in the segment; a bank that re-adds F must error.
    let collision_err = apply_count("STKWR-FP", "FPLT", 1).unwrap_err();
    assert!(collision_err.to_string().contains("already present"));
}
