# The `.steno` format

Normative reference for the source DSL parsed by
[`crates/steno/src/parse/`](../crates/steno/src/parse/). Ground truth is that
module plus [`stroke/`](../crates/steno/src/stroke/) (count banks) and the header
comment of [`dict.steno`](../dict.steno). Where this doc and the code disagree,
the code wins.

A `.steno` file is a sequence of **entries**, **directives**, **comments**, and
blank lines, parsed line by line. Anything that is none of these is an error.

## Entries (fenced blocks)

An entry is a fenced block. A line is a **fence** when it begins with **four or
more backticks**. The text after the backticks (trimmed) is the block's stroke:

````text
````STROKE
template line(s)
````
````

- The **opening** fence carries a non-empty stroke.
- The **closing** fence is backticks with nothing after them (empty after trim).
- Everything between the two fences is the template body, joined with `\n`.

A fence-looking line whose stroke is empty when no block is open is a stray
closing fence (error). A block with no closing fence before EOF is unterminated
(error). Both errors carry the 1-based source line.

### The stroke

The stroke is split on `/` into sub-strokes; each is trimmed and empty segments
are dropped. `STKWR-PBGS/-FLT` is two sub-strokes, `["STKWR-PBGS", "-FLT"]`. The
exact text as written is also retained (for diagnostics and collision reports).

Each sub-stroke is canonical Plover steno. Key order within a single stroke is:

```
#  |  S T K P W H R  |  A O * E U  |  F R P B L G T S D Z
   left bank            middle        right bank
```

`#` is the number bar. The hyphen appears **only** to separate right-bank keys
when there is no middle key (vowel or `*`) to do it; with a middle present, no
hyphen. Strokes are re-rendered into this canonical order after count keys are
merged in, so `STKWR-PBGS/-FLT` with a count of 3 becomes `STKWR-PBGS/AOFLT`.

## Comments and blank lines

A line whose trimmed text starts with `#` or `//` is a comment. Blank lines are
ignored. Both are legal anywhere outside a fenced block. (Inside a block, every
line is template text — `#` and `//` there are literal.)

## Directives

A directive is a line whose trimmed text starts with `@`. It attaches to the
**entry immediately above it**; a directive before any entry is an error. The
name runs to the first non-`[A-Za-z0-9_]` character; the rest (trimmed) is its
argument. An unknown directive name is an error.

| Directive | Arg | Meaning |
|---|---|---|
| `@count KEYS` | key list (required) | Fan this entry out over a count bank (see below). Requires a count operator (`%d`, `%[…%]`, or `%(…)`) in the template. |
| `@arity N` | non-negative int | Type-arg count for a generic `@type` entry. |
| `@multiline` | — | Construct never collapses to one line (ignores the one-liner flag). |
| `@type` | — | This entry is an appendable **type**, not a standalone construct; it is consumed by the type-append pass and never emitted on its own. |
| `@noarg` | — | A type that may be a return type but never a generic argument. |
| `@fuse` | — | Fuse the entry's last stroke segment into the first appended type stroke, so the type-less intermediate is never a required stroke (e.g. `func / TPH-FLT` → a number function directly). |
| `@literal` | — | The body is a complete pre-formatted multi-line block (a data structure). The smart profile drives it structurally via block-expansion rather than typing literal tabs and closers (see [pipeline.md](pipeline.md)). |

`@count` and `@arity` take an argument; the five boolean flags take none.
`@count` without a key list, `@arity` with a non-integer, and a count/operator
mismatch are all errors.

## Template operators

Within a block body, `%`, `{`, `}`, and `\` are special; everything else is
literal text. Raw `{` and `}` are structural braces (a profile may strip the
closer). The `%` operators:

| Operator | Name | Meaning |
|---|---|---|
| `%0`–`%9` | landing | Ordered landing point. The plain profile lands the cursor on `%0`; snippets renumber every landing to tabstops (see below). |
| `%d` | count digit | The in-scope count as a literal digit (**not** a landing), `0` included. Outside a repeat it is the total count; inside a repeat it is the 0-based iteration index. |
| `%t` | type slot | Filled by an appended type stroke during the type-append pass. |
| `%b` | body break | A newline the one-liner flag can toggle: every `%b` in a translation collapses together under the one-liner variant. |
| `%p` | pattern | Destructuring-pattern slot. |
| `%[ sep \| body %]` | repeat | Repeat `body` once per count; `sep` is the joiner emitted *between* items, never after the last. With no `\|`, the single segment is the body and there is no separator. |
| `%(EXPR)` | computed landing | A computed landing point. `EXPR` is linear in `d` (the in-scope count); it resolves to a landing index at expansion time. Negative results are an error; there is no renumbering, so a resolved conflict is an error. |

`%[` must be closed by `%]`; `%(` must be closed by `)`. Both unterminated forms,
and an unknown `%x` operator, are errors carrying the source line and column.

### Escapes

A backslash escapes the next character:

- `\n` → newline, `\t` → tab (emitted as structural newline/tab chunks).
- `\{`, `\}`, `\%`, `\|`, `\\`, `` \` `` → the literal character.
- Any other escape (`\x`) is an error, as is a trailing backslash at EOF.

## Count-bank encoding

`@count KEYS` turns an entry into a family of strokes, one per count value. The
key list is read **LSB-first**: the i-th listed key carries bit weight `2^i`.

```
@count AOEU     A=1  O=2  E=4  U=8      → encodes counts 0..15
@count AOE      A=1  O=2  E=4           → encodes counts 0..7
@count AO       A=1  O=2                → encodes counts 0..3
```

A bank of width `w` encodes `0 ..= 2^w − 1` inclusive. Each key must be a middle
key (vowel or `*`: `A O * E U`) or a right-bank key (`F R P B L G T S D Z`); any
other key, or a spec too wide to encode in 32 bits, is an error.

To realize count `n`, the bits of `n` select their keys, and those keys are
merged into the entry's **last** sub-stroke, which is then re-rendered in
canonical order. A count key already present in that sub-stroke, or a value
outside `0..=max`, is an error. The entry fans out to one stroke for every value
in `0..=max`; the template's `%d`, `%[…%]`, and `%(…)` operators are resolved
against that value (see [pipeline.md](pipeline.md), Pass A).
