//! Error types for PHP deserialization.
//!
//! This module provides detailed error types with context information
//! to help debug parsing issues.

use std::fmt;
use thiserror::Error;

/// The main error type for PHP deserialization.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub struct PhpDeserializeError {
    /// The kind of error that occurred.
    pub kind: ErrorKind,
    /// The byte position where the error occurred.
    pub position: usize,
    /// Optional context about what was being parsed.
    pub context: Option<String>,
    /// Preview of input around error position for debugging.
    pub input_preview: Option<String>,
}

impl fmt::Display for PhpDeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at position {}", self.kind, self.position)?;
        if let Some(ref ctx) = self.context {
            write!(f, " ({})", ctx)?;
        }
        if let Some(ref preview) = self.input_preview {
            write!(f, "\n{}", preview)?;
        }
        Ok(())
    }
}

/// Specific kinds of deserialization errors.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// Unexpected end of input.
    #[error("unexpected end of input")]
    UnexpectedEof,

    /// Expected a specific character but found something else.
    #[error("expected '{expected}', found '{found}'")]
    UnexpectedChar {
        /// The character that was expected.
        expected: char,
        /// The character that was found.
        found: char,
    },

    /// Unknown type marker.
    #[error("unknown type marker '{0}'")]
    UnknownType(char),

    /// Invalid integer value.
    #[error("invalid integer: {0}")]
    InvalidInteger(String),

    /// Invalid float value.
    #[error("invalid float: {0}")]
    InvalidFloat(String),

    /// Invalid boolean value.
    #[error("invalid boolean value: {0}")]
    InvalidBoolean(String),

    /// String length mismatch.
    #[error("string length mismatch: expected {expected}, found {found}")]
    StringLengthMismatch {
        /// The expected string length in bytes.
        expected: usize,
        /// The actual number of bytes available.
        found: usize,
    },

    /// Invalid UTF-8 sequence.
    #[error("invalid UTF-8 sequence")]
    InvalidUtf8,

    /// Invalid reference index.
    #[error("invalid reference index: {0}")]
    InvalidReference(usize),

    /// Circular reference detected.
    #[error("circular reference detected at index {0}")]
    CircularReference(usize),

    /// Invalid array key type.
    #[error("invalid array key type: expected string or integer")]
    InvalidArrayKey,

    /// Missing semicolon terminator.
    #[error("missing semicolon terminator")]
    MissingSemicolon,

    /// Missing closing brace.
    #[error("missing closing brace")]
    MissingClosingBrace,

    /// Invalid escape sequence.
    #[error("invalid escape sequence")]
    InvalidEscape,

    /// Nesting depth exceeded.
    #[error("maximum nesting depth ({0}) exceeded")]
    MaxDepthExceeded(usize),
}

impl PhpDeserializeError {
    /// Create a new error with the given kind and position.
    #[inline]
    pub fn new(kind: ErrorKind, position: usize) -> Self {
        Self {
            kind,
            position,
            context: None,
            input_preview: None,
        }
    }

    /// Add context to the error.
    #[inline]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add input preview around the error position for debugging.
    ///
    /// Shows up to 20 bytes before and after the error position.
    #[cold]
    pub fn with_input_preview(mut self, data: &[u8], error_pos: usize) -> Self {
        let start = error_pos.saturating_sub(20);
        let end = (error_pos + 20).min(data.len());

        if start < end {
            let slice = &data[start..end];
            // Try UTF-8 first, fallback to lossy conversion
            let preview = String::from_utf8_lossy(slice);

            // Mark the error position with a caret
            let relative_pos = error_pos.saturating_sub(start);
            let mut result = String::with_capacity(preview.len() + 10);
            result.push_str(&preview);
            result.push('\n');
            result.push_str(&" ".repeat(relative_pos));
            result.push('^');

            self.input_preview = Some(result);
        }
        self
    }
}

/// Result type alias for PHP deserialization.
pub type Result<T> = std::result::Result<T, PhpDeserializeError>;
