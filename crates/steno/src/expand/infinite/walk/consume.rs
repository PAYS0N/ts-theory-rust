//! Consume one (possibly nested) type from a residual stroke run.

use super::super::InfType;

/// One consumed top-level type: rendered text and whether it completed.
pub(super) struct Consumed {
    /// Rendered type text (complete or bracketless partial).
    pub(super) text: String,
    /// True when the type's obligations were fully discharged.
    pub(super) complete: bool,
}

/// Find a type-table record by stroke.
pub(super) fn lookup<'a>(stroke: &str, types: &'a [InfType]) -> Option<&'a InfType> {
    types.iter().find(|t| t.stroke == stroke)
}

/// Substitute `args` into the `%t` markers of a type's text.
fn render_type(text: &str, args: &[String]) -> String {
    let mut out = String::new();
    for (i, part) in text.split("%t").enumerate() {
        if i > 0 {
            out.push_str(args.get(i - 1).map_or("", String::as_str));
        }
        out.push_str(part);
    }
    out
}

/// The bracketless partial form: `Array`, or `Map number`.
fn partial(t: &InfType, args: &[String]) -> String {
    let name = t.text.split('<').next().unwrap_or(&t.text);
    if args.is_empty() {
        name.to_string()
    } else {
        format!("{name} {}", args.join(" "))
    }
}

/// Consume one complete (possibly nested) type from `ts` starting at `*pos`,
/// advancing `*pos`. `None` iff a stroke is not a valid type. An incomplete
/// type returns its partial text with `complete = false`.
pub(super) fn consume_type(ts: &[String], pos: &mut usize, types: &[InfType]) -> Option<Consumed> {
    let t = lookup(ts.get(*pos)?, types)?;
    *pos += 1;
    let mut args: Vec<String> = Vec::new();
    for _ in 0..t.arity {
        if *pos >= ts.len() {
            return Some(Consumed {
                text: partial(t, &args),
                complete: false,
            });
        }
        let arg = consume_type(ts, pos, types)?;
        let complete = arg.complete;
        args.push(arg.text);
        if !complete {
            return Some(Consumed {
                text: partial(t, &args),
                complete: false,
            });
        }
    }
    Some(Consumed {
        text: render_type(&t.text, &args),
        complete: true,
    })
}
