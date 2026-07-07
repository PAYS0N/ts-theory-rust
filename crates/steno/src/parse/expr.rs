//! Linear-in-`d` expression parsing for `%(EXPR)` computed landings.

use super::Expr;

/// Parse an expression linear in `d`: `[-]N d [±M]`, bare `d`, or a bare
/// integer. Whitespace is ignored. Returns None when malformed.
pub(super) fn parse_expr(raw: &str) -> Option<Expr> {
    let s: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some((a_part, b_part)) = s.split_once('d') {
        let a = match a_part {
            "" => 1,
            "-" => -1,
            _ if is_signed_digits(a_part) => a_part.parse().ok()?,
            _ => return None,
        };
        let b = parse_offset(b_part)?;
        return Some(Expr { a, b });
    }
    if is_signed_digits(&s) {
        return Some(Expr {
            a: 0,
            b: s.parse().ok()?,
        });
    }
    None
}

/// True when `s` is an optional `-` followed by at least one digit.
fn is_signed_digits(s: &str) -> bool {
    let body = s.strip_prefix('-').unwrap_or(s);
    !body.is_empty() && body.chars().all(|c| c.is_ascii_digit())
}

/// Parse the `±M` offset tail of a linear expression: empty means 0; anything
/// else must be an explicit sign followed by digits.
fn parse_offset(s: &str) -> Option<i32> {
    if s.is_empty() {
        return Some(0);
    }
    let (sign, body) = s.split_at_checked(1)?;
    if !(sign == "+" || sign == "-") || !is_signed_digits(body) || body.starts_with('-') {
        return None;
    }
    let magnitude: i32 = body.parse().ok()?;
    Some(if sign == "-" { -magnitude } else { magnitude })
}
