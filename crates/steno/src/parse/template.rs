//! Template-string parser: one fence body -> `Vec<Chunk>`.

use super::Chunk;
use crate::error::StenoError;

/// Parse a single template string into chunks. `line` is the 1-based source
/// line of the template's first line (used in error messages).
///
/// # Errors
/// Returns [`StenoError`] on unterminated constructs, unknown escapes or
/// operators, and malformed computed expressions.
pub fn parse_template(src: &str, line: usize) -> Result<Vec<Chunk>, StenoError> {
    let mut parser = TemplateParser {
        chars: src.chars().collect(),
        i: 0,
        line,
    };
    let (chunks, _) = parser.parse_chunks(false, false)?;
    Ok(chunks)
}

/// What ended a `parse_chunks` scan.
#[derive(PartialEq, Eq)]
enum Stop {
    /// A top-level `|` (separator/body divider inside a repeat).
    Pipe,
    /// A `%]` repeat closer.
    Close,
    /// End of input.
    End,
}

/// Chunk accumulator: pending literal text plus completed chunks.
#[derive(Default)]
struct ChunkAcc {
    /// Completed chunks in order.
    chunks: Vec<Chunk>,
    /// Literal text not yet flushed into a `Chunk::Lit`.
    lit: String,
}

impl ChunkAcc {
    /// Flush pending literal text into a `Chunk::Lit`, if any.
    fn flush(&mut self) {
        if !self.lit.is_empty() {
            self.chunks.push(Chunk::Lit(std::mem::take(&mut self.lit)));
        }
    }

    /// Flush pending text, then append a completed chunk.
    fn push(&mut self, chunk: Chunk) {
        self.flush();
        self.chunks.push(chunk);
    }
}

/// Cursor over the template's characters.
struct TemplateParser {
    /// Template body as characters (templates are effectively ASCII).
    chars: Vec<char>,
    /// Current position.
    i: usize,
    /// 1-based source line for error messages.
    line: usize,
}

impl TemplateParser {
    /// Look `o` characters ahead of the cursor.
    fn peek(&self, o: usize) -> Option<char> {
        self.chars.get(self.i.saturating_add(o)).copied()
    }

    /// Build an error at the current column.
    fn fail(&self, msg: &str) -> StenoError {
        StenoError::new(
            format!("{msg} (col {})", self.i.saturating_add(1)),
            self.line,
        )
    }

    /// Scan chunks until end of input or a requested stop token.
    fn parse_chunks(
        &mut self,
        stop_on_pipe: bool,
        stop_on_close: bool,
    ) -> Result<(Vec<Chunk>, Stop), StenoError> {
        let mut acc = ChunkAcc::default();
        while let Some(c) = self.peek(0) {
            if let Some(stop) = self.take_stop(stop_on_pipe, stop_on_close, &mut acc) {
                return Ok((acc.chunks, stop));
            }
            self.take_char(c, &mut acc)?;
        }
        acc.flush();
        if stop_on_close {
            return Err(self.fail("unterminated %[ ... %]"));
        }
        Ok((acc.chunks, Stop::End))
    }

    /// Consume a stop token (`|` or `%]`) at the cursor if one is requested
    /// and present.
    fn take_stop(
        &mut self,
        stop_on_pipe: bool,
        stop_on_close: bool,
        acc: &mut ChunkAcc,
    ) -> Option<Stop> {
        let c = self.peek(0)?;
        if stop_on_close && c == '%' && self.peek(1) == Some(']') {
            acc.flush();
            self.i += 2;
            return Some(Stop::Close);
        }
        if stop_on_pipe && c == '|' {
            acc.flush();
            self.i += 1;
            return Some(Stop::Pipe);
        }
        None
    }

    /// Consume one ordinary (non-stop) character at the cursor.
    fn take_char(&mut self, c: char, acc: &mut ChunkAcc) -> Result<(), StenoError> {
        match c {
            '\\' => self.take_escape(acc),
            '{' => {
                acc.push(Chunk::Brace { open: true });
                self.i += 1;
                Ok(())
            },
            '}' => {
                acc.push(Chunk::Brace { open: false });
                self.i += 1;
                Ok(())
            },
            '%' => self.take_operator(acc),
            _ => {
                acc.lit.push(c);
                self.i += 1;
                Ok(())
            },
        }
    }

    /// Consume a backslash escape: `\n`/`\t` become chunks, the rest become
    /// literal characters.
    fn take_escape(&mut self, acc: &mut ChunkAcc) -> Result<(), StenoError> {
        let Some(n) = self.peek(1) else {
            return Err(self.fail("trailing backslash"));
        };
        self.i += 2;
        match n {
            'n' => acc.push(Chunk::Newline),
            't' => acc.push(Chunk::Tab),
            '{' | '}' | '%' | '|' | '\\' | '`' => acc.lit.push(n),
            _ => {
                self.i -= 2;
                return Err(self.fail(&format!("unknown escape \\{n}")));
            },
        }
        Ok(())
    }

    /// Consume a `%` operator: landing digit, simple slot, repeat, or
    /// computed expression.
    fn take_operator(&mut self, acc: &mut ChunkAcc) -> Result<(), StenoError> {
        let Some(n) = self.peek(1) else {
            return Err(self.fail("trailing '%'"));
        };
        let simple = match n {
            _ if n.is_ascii_digit() => n.to_digit(10).map(Chunk::Landing),
            'd' => Some(Chunk::Dcount),
            't' => Some(Chunk::TypeSlot),
            'b' => Some(Chunk::BodyBreak),
            'p' => Some(Chunk::Pattern),
            _ => None,
        };
        if let Some(chunk) = simple {
            acc.push(chunk);
            self.i += 2;
            return Ok(());
        }
        self.take_block_operator(n, acc)
    }

    /// Consume a `%[` repeat or `%(` computed opener; anything else is an
    /// unknown operator.
    fn take_block_operator(&mut self, n: char, acc: &mut ChunkAcc) -> Result<(), StenoError> {
        match n {
            '[' => {
                self.i += 2;
                let repeat = self.parse_repeat()?;
                acc.push(repeat);
                Ok(())
            },
            '(' => {
                self.i += 2;
                let computed = self.parse_computed()?;
                acc.push(computed);
                Ok(())
            },
            _ => Err(self.fail(&format!("unknown operator %{n}"))),
        }
    }

    /// Parse a repeat after its `%[` is consumed. The first segment stops on
    /// a top-level `|` or `%]`; without a `|` the single segment is the body.
    fn parse_repeat(&mut self) -> Result<Chunk, StenoError> {
        let (first, stop) = self.parse_chunks(true, true)?;
        if stop == Stop::Pipe {
            let (body, _) = self.parse_chunks(false, true)?;
            return Ok(Chunk::Repeat { sep: first, body });
        }
        Ok(Chunk::Repeat {
            sep: Vec::new(),
            body: first,
        })
    }

    /// Parse a computed landing after its `%(` is consumed.
    fn parse_computed(&mut self) -> Result<Chunk, StenoError> {
        let mut expr = String::new();
        while let Some(c) = self.peek(0) {
            if c == ')' {
                break;
            }
            expr.push(c);
            self.i += 1;
        }
        if self.peek(0) != Some(')') {
            return Err(self.fail("unterminated %( ... )"));
        }
        self.i += 1;
        let parsed = super::expr::parse_expr(&expr)
            .ok_or_else(|| self.fail(&format!("bad computed expression \"{expr}\"")))?;
        Ok(Chunk::Computed(parsed))
    }
}
