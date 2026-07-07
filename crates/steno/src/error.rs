//! Typed pipeline errors. Every failure in the compiler surfaces as one of
//! these; nothing is swallowed and nothing panics.

use std::error::Error;
use std::fmt;

/// Error from stroke parsing, rendering, merging, or count-bank arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrokeError {
    /// Human-readable description of the failure.
    message: String,
}

impl StrokeError {
    /// Wrap a description into an error value.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for StrokeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for StrokeError {}

/// Parse error carrying the 1-based source line where it occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StenoError {
    /// Description of the failure (column info included where relevant).
    message: String,
    /// 1-based source line.
    line: usize,
}

impl StenoError {
    /// Wrap a description and source line into an error value.
    pub(crate) fn new(message: impl Into<String>, line: usize) -> Self {
        Self {
            message: message.into(),
            line,
        }
    }

    /// The 1-based source line the error refers to.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }
}

impl fmt::Display for StenoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl Error for StenoError {}

/// Error from the count- or type-expansion passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandError {
    /// Human-readable description of the failure.
    message: String,
}

impl ExpandError {
    /// Wrap a description into an error value.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ExpandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ExpandError {}

impl From<StrokeError> for ExpandError {
    /// Stroke arithmetic failures surface as expansion errors with the same
    /// message (the TS pipeline let them propagate unwrapped).
    fn from(e: StrokeError) -> Self {
        Self::new(e.to_string())
    }
}

/// Error from the rendering pass: an unresolved chunk reaching a renderer, or
/// a movement whose target is not up-and-left of the resting cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError {
    /// Human-readable description of the failure.
    message: String,
}

impl RenderError {
    /// Wrap a description into an error value.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for RenderError {}

/// Error from the snippet renderer: an unresolved chunk reaching it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetError {
    /// Human-readable description of the failure.
    message: String,
}

impl SnippetError {
    /// Wrap a description into an error value.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SnippetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for SnippetError {}
