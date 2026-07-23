//! Bracketed template operators: `%[...%]` repeats, `%(...)` computed
//! landings, and `%<...>` end landings.

use super::{Chunk, ChunkAcc, StenoError, Stop, TemplateParser};

impl TemplateParser {
    /// Consume a `%[` repeat, `%(` computed, or `%<` end-landing opener;
    /// anything else is an unknown operator.
    pub(super) fn take_block_operator(
        &mut self,
        n: char,
        acc: &mut ChunkAcc,
    ) -> Result<(), StenoError> {
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
            '<' => {
                self.i += 2;
                let end_landing = self.parse_end_landing()?;
                acc.push(end_landing);
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
        let parsed = super::super::expr::parse_expr(&expr)
            .ok_or_else(|| self.fail(&format!("bad computed expression \"{expr}\"")))?;
        Ok(Chunk::Computed(parsed))
    }

    /// Parse an end landing `%<N>` after its `%<` is consumed.
    fn parse_end_landing(&mut self) -> Result<Chunk, StenoError> {
        let mut digits = String::new();
        while let Some(c) = self.peek(0) {
            if c == '>' {
                break;
            }
            if !c.is_ascii_digit() {
                return Err(self.fail(&format!("bad end landing \"%<{digits}{c}\"")));
            }
            digits.push(c);
            self.i += 1;
        }
        if self.peek(0) != Some('>') {
            return Err(self.fail("unterminated %< ... >"));
        }
        if digits.is_empty() {
            return Err(self.fail("empty end landing \"%<>\""));
        }
        self.i += 1;
        let n = digits
            .parse::<u32>()
            .map_err(|_| self.fail(&format!("bad end landing \"%<{digits}>\"")))?;
        Ok(Chunk::EndLanding(n))
    }
}
