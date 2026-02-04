//! Zero-copy PHP serialize parser.
//!
//! This module provides a high-performance, zero-copy parser for PHP's serialize format.
//! The parser is designed to minimize allocations and maximize throughput.
//!
//! # Performance Characteristics
//!
//! - **Zero-copy strings**: Borrows directly from input when possible
//! - **SIMD-accelerated search**: Uses `memchr` for fast delimiter scanning
//! - **Inline hot paths**: Critical functions are marked for inlining
//! - **Minimal allocations**: Only allocates for owned data and collections
//!
//! # Tracing Support
//!
//! Enable the `tracing` feature for detailed parsing instrumentation:
//!
//! ```toml
//! php-deserialize-core = { version = "0.1", features = ["tracing"] }
//! ```

use std::borrow::Cow;

use memchr::memchr;

#[cfg(feature = "tracing")]
use tracing::{debug, instrument, trace, warn};

use crate::error::{ErrorKind, PhpDeserializeError, Result};
use crate::types::{PhpProperty, PhpValue, Visibility};

/// Maximum nesting depth to prevent stack overflow.
const MAX_DEPTH: usize = 512;

/// Parser configuration options.
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Maximum nesting depth for arrays and objects.
    pub max_depth: usize,
    /// Whether to automatically unescape DB-exported strings.
    pub auto_unescape: bool,
    /// Whether to be strict about UTF-8 encoding in strings.
    pub strict_utf8: bool,
    /// Whether to disable automatic fallback for string length mismatches.
    ///
    /// When `strict` is `false` (the default), the parser will automatically try
    /// lenient recovery when strict parsing fails for strings. This handles cases
    /// where data was serialized with a different encoding (e.g., EUC-KR) but
    /// stored/transmitted as UTF-8.
    ///
    /// When `strict` is `true`, the parser will fail immediately on string length
    /// mismatches without attempting recovery.
    pub strict: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_depth: MAX_DEPTH,
            auto_unescape: true,
            strict_utf8: false,
            strict: false, // Auto-fallback enabled by default
        }
    }
}

/// A zero-copy PHP deserialize parser.
pub struct Parser<'a> {
    /// Input data.
    data: &'a [u8],
    /// Current position in the input.
    pos: usize,
    /// Parser configuration.
    config: ParserConfig,
    /// Current nesting depth.
    depth: usize,
    /// Reference table for handling R/r references.
    references: Vec<usize>,
}

impl<'a> Parser<'a> {
    /// Create a new parser with default configuration.
    pub fn new(data: &'a [u8]) -> Self {
        Self::with_config(data, ParserConfig::default())
    }

    /// Create a new parser with custom configuration.
    pub fn with_config(data: &'a [u8], config: ParserConfig) -> Self {
        Self {
            data,
            pos: 0,
            config,
            depth: 0,
            references: Vec::new(),
        }
    }

    /// Parse the input and return a PHP value.
    ///
    /// This is the main entry point for parsing PHP serialized data.
    /// If the `tracing` feature is enabled, this method will emit trace events.
    #[cfg_attr(feature = "tracing", instrument(skip(self), fields(data_len = self.data.len())))]
    pub fn parse(&mut self) -> Result<PhpValue<'a>> {
        #[cfg(feature = "tracing")]
        debug!(data_len = self.data.len(), "Starting PHP deserialize");

        // Preprocess if needed
        let data = if self.config.auto_unescape {
            preprocess(self.data)
        } else {
            Cow::Borrowed(self.data)
        };

        #[cfg(feature = "tracing")]
        if matches!(data, Cow::Owned(_)) {
            trace!("Data was preprocessed (DB escape detected)");
        }

        // If preprocessing changed the data, we need to use the owned version
        let result = match data {
            Cow::Borrowed(_) => self.parse_value(),
            Cow::Owned(owned) => {
                // Create a new parser for the owned data
                let mut parser = Parser::with_config(
                    // SAFETY: We're creating a temporary reference that lives as long as this function.
                    // The owned data is kept alive by the local variable `owned` until parse_value completes.
                    // The result is immediately converted to owned via into_owned(), so no references
                    // to the temporary data escape this function.
                    // TODO: Consider using a safer approach with OwnedParser type (see IMPROVEMENTS.md)
                    unsafe { std::mem::transmute::<&[u8], &'a [u8]>(&owned) },
                    self.config.clone(),
                );
                parser.parse_value().map(|v| v.into_owned())
            }
        };

        #[cfg(feature = "tracing")]
        match &result {
            Ok(value) => debug!(value_type = value.type_name(), "Parse completed successfully"),
            Err(e) => warn!(error = %e, "Parse failed"),
        }

        result
    }

    /// Parse a single value at the current position.
    ///
    /// This is the core parsing dispatch function that routes to type-specific parsers.
    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace", fields(pos = self.pos, depth = self.depth)))]
    fn parse_value(&mut self) -> Result<PhpValue<'a>> {
        // Check depth limit
        if self.depth > self.config.max_depth {
            #[cfg(feature = "tracing")]
            warn!(depth = self.depth, max_depth = self.config.max_depth, "Max depth exceeded");
            return Err(PhpDeserializeError::new(
                ErrorKind::MaxDepthExceeded(self.config.max_depth),
                self.pos,
            ));
        }

        let type_byte = self.peek_byte()?;

        #[cfg(feature = "tracing")]
        trace!(type_marker = %char::from(type_byte), pos = self.pos, "Parsing value");

        // Record position for reference table
        let start_pos = self.pos;
        self.references.push(start_pos);

        let value = match type_byte {
            b'N' => self.parse_null()?,
            b'b' => self.parse_bool()?,
            b'i' => self.parse_int()?,
            b'd' => self.parse_float()?,
            b's' => self.parse_string()?,
            b'a' => self.parse_array()?,
            b'O' => self.parse_object()?,
            b'C' => self.parse_custom_object()?,
            b'R' | b'r' => self.parse_reference()?,
            b'E' => self.parse_enum()?,
            _ => {
                #[cfg(feature = "tracing")]
                warn!(type_byte = %char::from(type_byte), pos = self.pos, "Unknown type marker");
                return Err(PhpDeserializeError::new(
                    ErrorKind::UnknownType(type_byte as char),
                    self.pos,
                )
                .with_input_preview(self.data, self.pos));
            }
        };

        Ok(value)
    }

    /// Parse a null value: `N;`
    fn parse_null(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b'N')?;
        self.expect_byte(b';')?;
        Ok(PhpValue::Null)
    }

    /// Parse a boolean value: `b:0;` or `b:1;`
    fn parse_bool(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b'b')?;
        self.expect_byte(b':')?;
        let value_byte = self.read_byte()?;
        self.expect_byte(b';')?;

        match value_byte {
            b'0' => Ok(PhpValue::Bool(false)),
            b'1' => Ok(PhpValue::Bool(true)),
            _ => Err(PhpDeserializeError::new(
                ErrorKind::InvalidBoolean((value_byte as char).to_string()),
                self.pos - 2,
            )),
        }
    }

    /// Parse an integer value: `i:<value>;`
    fn parse_int(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b'i')?;
        self.expect_byte(b':')?;

        let start = self.pos;
        let value = self.read_until(b';')?;

        let int_str = std::str::from_utf8(value).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger("invalid UTF-8".into()), start)
        })?;

        let int_value: i64 = int_str.parse().map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger(int_str.to_string()), start)
        })?;

        self.expect_byte(b';')?;
        Ok(PhpValue::Int(int_value))
    }

    /// Parse a float/double value: `d:<value>;`
    fn parse_float(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b'd')?;
        self.expect_byte(b':')?;

        let start = self.pos;
        let value = self.read_until(b';')?;

        let float_str = std::str::from_utf8(value).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidFloat("invalid UTF-8".into()), start)
        })?;

        // Handle special PHP float values
        let float_value: f64 = match float_str {
            "INF" => f64::INFINITY,
            "-INF" => f64::NEG_INFINITY,
            "NAN" => f64::NAN,
            _ => float_str.parse().map_err(|_| {
                PhpDeserializeError::new(ErrorKind::InvalidFloat(float_str.to_string()), start)
            })?,
        };

        self.expect_byte(b';')?;
        Ok(PhpValue::Float(float_value))
    }

    /// Parse a string value: `s:<len>:"<data>";`
    ///
    /// By default (when `config.strict` is `false`), this method will automatically
    /// try lenient recovery if strict parsing fails due to string length mismatches.
    fn parse_string(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b's')?;
        self.expect_byte(b':')?;

        // Read length
        let len_start = self.pos;
        let len_bytes = self.read_until(b':')?;
        let len_str = std::str::from_utf8(len_bytes).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger("invalid UTF-8".into()), len_start)
        })?;
        let len: usize = len_str.parse().map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger(len_str.to_string()), len_start)
        })?;

        self.expect_byte(b':')?;
        self.expect_byte(b'"')?;

        let string_start = self.pos;

        // Try strict parsing first
        if self.pos + len <= self.data.len() {
            // Check if the declared length works
            let potential_end = self.pos + len;
            if potential_end + 1 < self.data.len()
                && self.data[potential_end] == b'"'
                && self.data[potential_end + 1] == b';'
            {
                // Strict parsing works
                let string_data = &self.data[self.pos..potential_end];
                self.pos = potential_end;
                self.expect_byte(b'"')?;
                self.expect_byte(b';')?;
                return Ok(PhpValue::String(Cow::Borrowed(string_data)));
            }
        }

        // Strict parsing failed - try automatic fallback unless strict mode is enabled
        if !self.config.strict {
            #[cfg(feature = "tracing")]
            warn!(
                pos = string_start,
                declared_len = len,
                "Strict string parsing failed, attempting automatic lenient recovery"
            );

            return self.parse_string_lenient(string_start, len);
        }

        // Strict mode - return error without fallback
        if self.pos + len > self.data.len() {
            return Err(PhpDeserializeError::new(
                ErrorKind::StringLengthMismatch {
                    expected: len,
                    found: self.data.len() - self.pos,
                },
                string_start,
            ));
        }

        // Move position and expect closing chars (will fail with better error)
        self.pos += len;
        self.expect_byte(b'"')?;
        self.expect_byte(b';')?;

        unreachable!()
    }

    /// Parse a string in lenient mode by searching for the `";` pattern.
    #[cold]
    fn parse_string_lenient(&mut self, string_start: usize, declared_len: usize) -> Result<PhpValue<'a>> {
        #[cfg(feature = "tracing")]
        trace!(
            declared_len = declared_len,
            pos = string_start,
            "Using lenient string parsing"
        );

        // Search for `";` pattern from the current position
        // We need to be careful: the string might contain `";` as content
        // Strategy: search for `";` and verify it's followed by a valid type marker or end
        let search_start = string_start;
        let mut search_pos = search_start;

        while search_pos < self.data.len().saturating_sub(1) {
            // Find next `"` character
            match memchr(b'"', &self.data[search_pos..]) {
                Some(offset) => {
                    let quote_pos = search_pos + offset;
                    // Check if followed by `;`
                    if quote_pos + 1 < self.data.len() && self.data[quote_pos + 1] == b';' {
                        // Verify this is a valid end by checking what follows
                        let after_semicolon = quote_pos + 2;
                        let is_valid_end = if after_semicolon >= self.data.len() {
                            // End of data - valid
                            true
                        } else {
                            // Check for valid following byte
                            let next_byte = self.data[after_semicolon];
                            // Valid: type marker, closing brace, or another string/array start
                            matches!(
                                next_byte,
                                b'N' | b'b' | b'i' | b'd' | b's' | b'a' | b'O' | b'C'
                                    | b'R' | b'r' | b'E' | b'}'
                            )
                        };

                        if is_valid_end {
                            // Found valid end
                            let string_data = &self.data[string_start..quote_pos];
                            self.pos = quote_pos;
                            self.expect_byte(b'"')?;
                            self.expect_byte(b';')?;

                            #[cfg(feature = "tracing")]
                            if string_data.len() != declared_len {
                                warn!(
                                    declared = declared_len,
                                    actual = string_data.len(),
                                    "String length mismatch recovered"
                                );
                            }

                            return Ok(PhpValue::String(Cow::Borrowed(string_data)));
                        }
                    }
                    // Not a valid end, continue searching
                    search_pos = quote_pos + 1;
                }
                None => break,
            }
        }

        // Could not find valid end
        Err(PhpDeserializeError::new(
            ErrorKind::StringLengthMismatch {
                expected: declared_len,
                found: self.data.len() - string_start,
            },
            string_start,
        )
        .with_context("lenient parsing also failed to find string end"))
    }

    /// Parse an array value: `a:<count>:{<key><value>...}`
    fn parse_array(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b'a')?;
        self.expect_byte(b':')?;

        // Read count
        let count_start = self.pos;
        let count_bytes = self.read_until(b':')?;
        let count_str = std::str::from_utf8(count_bytes).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger("invalid UTF-8".into()), count_start)
        })?;
        let count: usize = count_str.parse().map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger(count_str.to_string()), count_start)
        })?;

        self.expect_byte(b':')?;
        self.expect_byte(b'{')?;

        self.depth += 1;
        let mut items = Vec::with_capacity(count.min(1024)); // Cap initial allocation

        for _ in 0..count {
            let key = self.parse_value()?;

            // Validate key type
            match &key {
                PhpValue::String(_) | PhpValue::Int(_) => {}
                _ => {
                    return Err(PhpDeserializeError::new(ErrorKind::InvalidArrayKey, self.pos));
                }
            }

            let value = self.parse_value()?;
            items.push((key, value));
        }

        self.depth -= 1;
        self.expect_byte(b'}')?;

        Ok(PhpValue::Array(items))
    }

    /// Parse an object value: `O:<namelen>:"<name>":<count>:{<prop>...}`
    fn parse_object(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b'O')?;
        self.expect_byte(b':')?;

        // Read class name length
        let len_start = self.pos;
        let len_bytes = self.read_until(b':')?;
        let len_str = std::str::from_utf8(len_bytes).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger("invalid UTF-8".into()), len_start)
        })?;
        let name_len: usize = len_str.parse().map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger(len_str.to_string()), len_start)
        })?;

        self.expect_byte(b':')?;
        self.expect_byte(b'"')?;

        // Read class name
        if self.pos + name_len > self.data.len() {
            return Err(PhpDeserializeError::new(ErrorKind::UnexpectedEof, self.pos));
        }
        let class_name_bytes = &self.data[self.pos..self.pos + name_len];
        let class_name = std::str::from_utf8(class_name_bytes)
            .map_err(|_| PhpDeserializeError::new(ErrorKind::InvalidUtf8, self.pos))?;
        self.pos += name_len;

        self.expect_byte(b'"')?;
        self.expect_byte(b':')?;

        // Read property count
        let count_start = self.pos;
        let count_bytes = self.read_until(b':')?;
        let count_str = std::str::from_utf8(count_bytes).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger("invalid UTF-8".into()), count_start)
        })?;
        let count: usize = count_str.parse().map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger(count_str.to_string()), count_start)
        })?;

        self.expect_byte(b':')?;
        self.expect_byte(b'{')?;

        self.depth += 1;
        let mut properties = Vec::with_capacity(count.min(1024));

        for _ in 0..count {
            let prop = self.parse_property()?;
            properties.push(prop);
        }

        self.depth -= 1;
        self.expect_byte(b'}')?;

        Ok(PhpValue::Object {
            class_name: Cow::Borrowed(class_name),
            properties,
        })
    }

    /// Parse an object property with visibility handling.
    fn parse_property(&mut self) -> Result<PhpProperty<'a>> {
        // Property name is always a string
        let name_value = self.parse_value()?;
        let name_bytes = match &name_value {
            PhpValue::String(s) => s.as_ref(),
            _ => {
                return Err(PhpDeserializeError::new(ErrorKind::InvalidArrayKey, self.pos));
            }
        };

        // Parse visibility from property name
        // Private: \0ClassName\0propName
        // Protected: \0*\0propName
        // Public: propName
        let (name, visibility, declaring_class) = parse_property_name(name_bytes)?;

        let value = self.parse_value()?;

        Ok(PhpProperty {
            name: Cow::Owned(name),
            visibility,
            declaring_class: declaring_class.map(Cow::Owned),
            value,
        })
    }

    /// Parse a custom serialized object: `C:<namelen>:"<name>":<datalen>:{<data>}`
    fn parse_custom_object(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b'C')?;
        self.expect_byte(b':')?;

        // Read class name length
        let len_start = self.pos;
        let len_bytes = self.read_until(b':')?;
        let len_str = std::str::from_utf8(len_bytes).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger("invalid UTF-8".into()), len_start)
        })?;
        let name_len: usize = len_str.parse().map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger(len_str.to_string()), len_start)
        })?;

        self.expect_byte(b':')?;
        self.expect_byte(b'"')?;

        // Read class name
        if self.pos + name_len > self.data.len() {
            return Err(PhpDeserializeError::new(ErrorKind::UnexpectedEof, self.pos));
        }
        let class_name_bytes = &self.data[self.pos..self.pos + name_len];
        let class_name = std::str::from_utf8(class_name_bytes)
            .map_err(|_| PhpDeserializeError::new(ErrorKind::InvalidUtf8, self.pos))?;
        self.pos += name_len;

        self.expect_byte(b'"')?;
        self.expect_byte(b':')?;

        // Read data length
        let data_len_start = self.pos;
        let data_len_bytes = self.read_until(b':')?;
        let data_len_str = std::str::from_utf8(data_len_bytes).map_err(|_| {
            PhpDeserializeError::new(
                ErrorKind::InvalidInteger("invalid UTF-8".into()),
                data_len_start,
            )
        })?;
        let data_len: usize = data_len_str.parse().map_err(|_| {
            PhpDeserializeError::new(
                ErrorKind::InvalidInteger(data_len_str.to_string()),
                data_len_start,
            )
        })?;

        self.expect_byte(b':')?;
        self.expect_byte(b'{')?;

        // Read custom data
        if self.pos + data_len > self.data.len() {
            return Err(PhpDeserializeError::new(ErrorKind::UnexpectedEof, self.pos));
        }
        let custom_data = &self.data[self.pos..self.pos + data_len];
        self.pos += data_len;

        self.expect_byte(b'}')?;

        // Return as an object with a single "__data" property containing the raw data
        Ok(PhpValue::Object {
            class_name: Cow::Borrowed(class_name),
            properties: vec![PhpProperty {
                name: Cow::Borrowed("__data"),
                visibility: Visibility::Public,
                declaring_class: None,
                value: PhpValue::String(Cow::Borrowed(custom_data)),
            }],
        })
    }

    /// Parse a reference: `R:<index>;` or `r:<index>;`
    fn parse_reference(&mut self) -> Result<PhpValue<'a>> {
        let ref_type = self.read_byte()?;
        self.expect_byte(b':')?;

        let idx_start = self.pos;
        let idx_bytes = self.read_until(b';')?;
        let idx_str = std::str::from_utf8(idx_bytes).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger("invalid UTF-8".into()), idx_start)
        })?;
        let idx: usize = idx_str.parse().map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger(idx_str.to_string()), idx_start)
        })?;

        self.expect_byte(b';')?;

        // PHP references are 1-indexed
        if idx == 0 || idx > self.references.len() {
            return Err(PhpDeserializeError::new(
                ErrorKind::InvalidReference(idx),
                idx_start,
            ));
        }

        // For now, just return a reference marker
        // Full reference resolution would require a two-pass approach
        let _ = ref_type; // R = object reference, r = value reference
        Ok(PhpValue::Reference(idx))
    }

    /// Parse a PHP 8.1+ enum: `E:<len>:"<ClassName:CaseName>";`
    fn parse_enum(&mut self) -> Result<PhpValue<'a>> {
        self.expect_byte(b'E')?;
        self.expect_byte(b':')?;

        // Read length
        let len_start = self.pos;
        let len_bytes = self.read_until(b':')?;
        let len_str = std::str::from_utf8(len_bytes).map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger("invalid UTF-8".into()), len_start)
        })?;
        let len: usize = len_str.parse().map_err(|_| {
            PhpDeserializeError::new(ErrorKind::InvalidInteger(len_str.to_string()), len_start)
        })?;

        self.expect_byte(b':')?;
        self.expect_byte(b'"')?;

        // Read enum string (ClassName:CaseName)
        if self.pos + len > self.data.len() {
            return Err(PhpDeserializeError::new(ErrorKind::UnexpectedEof, self.pos));
        }
        let enum_str = &self.data[self.pos..self.pos + len];
        let enum_str = std::str::from_utf8(enum_str)
            .map_err(|_| PhpDeserializeError::new(ErrorKind::InvalidUtf8, self.pos))?;
        self.pos += len;

        self.expect_byte(b'"')?;
        self.expect_byte(b';')?;

        // Split on colon
        let colon_pos = enum_str.find(':').ok_or_else(|| {
            PhpDeserializeError::new(
                ErrorKind::UnexpectedChar {
                    expected: ':',
                    found: ' ',
                },
                self.pos,
            )
            .with_context("enum format should be ClassName:CaseName")
        })?;

        let class_name = &enum_str[..colon_pos];
        let case_name = &enum_str[colon_pos + 1..];

        Ok(PhpValue::Enum {
            class_name: Cow::Owned(class_name.to_string()),
            case_name: Cow::Owned(case_name.to_string()),
        })
    }

    // Helper methods - marked #[inline] for performance on hot paths

    /// Peek at the current byte without consuming it.
    #[inline(always)]
    fn peek_byte(&self) -> Result<u8> {
        self.data
            .get(self.pos)
            .copied()
            .ok_or_else(|| PhpDeserializeError::new(ErrorKind::UnexpectedEof, self.pos))
    }

    /// Read and consume the current byte.
    #[inline(always)]
    fn read_byte(&mut self) -> Result<u8> {
        let byte = self.peek_byte()?;
        self.pos += 1;
        Ok(byte)
    }

    /// Expect a specific byte, returning an error if it doesn't match.
    #[inline]
    fn expect_byte(&mut self, expected: u8) -> Result<()> {
        let byte = self.read_byte()?;
        if byte != expected {
            return Err(self.make_unexpected_char_error(expected, byte));
        }
        Ok(())
    }

    /// Create an unexpected character error with proper context.
    #[cold]
    #[inline(never)]
    fn make_unexpected_char_error(&self, expected: u8, found: u8) -> PhpDeserializeError {
        PhpDeserializeError::new(
            ErrorKind::UnexpectedChar {
                expected: expected as char,
                found: found as char,
            },
            self.pos - 1,
        )
        .with_input_preview(self.data, self.pos.saturating_sub(1))
    }

    /// Read bytes until the delimiter, using SIMD-accelerated search.
    #[inline]
    fn read_until(&mut self, delimiter: u8) -> Result<&'a [u8]> {
        let start = self.pos;
        match memchr(delimiter, &self.data[start..]) {
            Some(offset) => {
                let result = &self.data[start..start + offset];
                self.pos = start + offset;
                Ok(result)
            }
            None => Err(self.make_delimiter_not_found_error(delimiter)),
        }
    }

    /// Create a delimiter not found error with proper context.
    #[cold]
    #[inline(never)]
    fn make_delimiter_not_found_error(&self, delimiter: u8) -> PhpDeserializeError {
        PhpDeserializeError::new(
            ErrorKind::UnexpectedChar {
                expected: delimiter as char,
                found: if self.pos < self.data.len() {
                    self.data[self.pos] as char
                } else {
                    '\0'
                },
            },
            self.pos,
        )
        .with_input_preview(self.data, self.pos)
    }
}

/// Parse a property name and extract visibility information.
fn parse_property_name(name: &[u8]) -> Result<(String, Visibility, Option<String>)> {
    if name.is_empty() {
        return Ok((String::new(), Visibility::Public, None));
    }

    // Check for null byte prefix (private or protected)
    if name[0] == 0 {
        // Find the second null byte
        if let Some(second_null) = memchr(0, &name[1..]) {
            let prefix = &name[1..1 + second_null];
            let actual_name = &name[2 + second_null..];
            let actual_name_str =
                String::from_utf8_lossy(actual_name).into_owned();

            if prefix == b"*" {
                // Protected: \0*\0propName
                Ok((actual_name_str, Visibility::Protected, None))
            } else {
                // Private: \0ClassName\0propName
                let class_name = String::from_utf8_lossy(prefix).into_owned();
                Ok((actual_name_str, Visibility::Private, Some(class_name)))
            }
        } else {
            // Malformed, treat as public
            Ok((
                String::from_utf8_lossy(name).into_owned(),
                Visibility::Public,
                None,
            ))
        }
    } else {
        // Public property
        Ok((
            String::from_utf8_lossy(name).into_owned(),
            Visibility::Public,
            None,
        ))
    }
}

/// Preprocess data to handle DB-escaped strings.
///
/// Detects patterns like:
/// - `"a:1:{...}"` (wrapped in quotes)
/// - Double quotes escaped as `""`
pub fn preprocess(data: &[u8]) -> Cow<'_, [u8]> {
    // Check if data starts with `"` and ends with `"`
    if data.len() >= 2 && data[0] == b'"' && data[data.len() - 1] == b'"' {
        // Check if inner content looks like PHP serialized data
        let inner = &data[1..data.len() - 1];
        if !inner.is_empty() {
            let first_char = inner[0];
            // PHP serialize type markers
            if matches!(
                first_char,
                b'N' | b'b' | b'i' | b'd' | b's' | b'a' | b'O' | b'C' | b'R' | b'r' | b'E'
            ) {
                // Unescape double quotes
                return Cow::Owned(unescape_double_quotes(inner));
            }
        }
    }

    Cow::Borrowed(data)
}

/// Unescape double quotes (`""` -> `"`).
fn unescape_double_quotes(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        if i + 1 < data.len() && data[i] == b'"' && data[i + 1] == b'"' {
            result.push(b'"');
            i += 2;
        } else {
            result.push(data[i]);
            i += 1;
        }
    }

    result
}

/// Parse PHP serialized data from bytes.
///
/// This is the primary API for parsing PHP serialized data.
///
/// # Example
///
/// ```rust
/// use php_deserialize_core::from_bytes;
///
/// let value = from_bytes(b"i:42;").unwrap();
/// assert_eq!(value.as_int(), Some(42));
/// ```
#[inline]
pub fn from_bytes(data: &[u8]) -> Result<PhpValue<'_>> {
    #[cfg(feature = "tracing")]
    trace!(data_len = data.len(), "from_bytes called");

    let mut parser = Parser::new(data);
    parser.parse()
}

/// Parse PHP serialized data from bytes with custom configuration.
///
/// # Example
///
/// ```rust
/// use php_deserialize_core::{from_bytes_with_config, ParserConfig};
///
/// let config = ParserConfig {
///     max_depth: 64,
///     auto_unescape: false,
///     strict_utf8: true,
///     strict: true, // Disable automatic fallback for string length mismatches
/// };
/// let value = from_bytes_with_config(b"i:42;", config).unwrap();
/// ```
#[inline]
pub fn from_bytes_with_config(data: &[u8], config: ParserConfig) -> Result<PhpValue<'_>> {
    #[cfg(feature = "tracing")]
    trace!(data_len = data.len(), ?config, "from_bytes_with_config called");

    let mut parser = Parser::with_config(data, config);
    parser.parse()
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;

    #[test]
    fn test_null() {
        let result = from_bytes(b"N;").unwrap();
        assert_eq!(result, PhpValue::Null);
    }

    #[test]
    fn test_bool() {
        assert_eq!(from_bytes(b"b:0;").unwrap(), PhpValue::Bool(false));
        assert_eq!(from_bytes(b"b:1;").unwrap(), PhpValue::Bool(true));
    }

    #[test]
    fn test_int() {
        assert_eq!(from_bytes(b"i:0;").unwrap(), PhpValue::Int(0));
        assert_eq!(from_bytes(b"i:42;").unwrap(), PhpValue::Int(42));
        assert_eq!(from_bytes(b"i:-123;").unwrap(), PhpValue::Int(-123));
        assert_eq!(
            from_bytes(b"i:9223372036854775807;").unwrap(),
            PhpValue::Int(i64::MAX)
        );
    }

    #[test]
    fn test_float() {
        assert_eq!(from_bytes(b"d:0;").unwrap(), PhpValue::Float(0.0));
        assert_eq!(from_bytes(b"d:3.14;").unwrap(), PhpValue::Float(3.14));
        assert_eq!(from_bytes(b"d:-2.5;").unwrap(), PhpValue::Float(-2.5));
        assert!(matches!(from_bytes(b"d:INF;").unwrap(), PhpValue::Float(f) if f.is_infinite() && f.is_sign_positive()));
        assert!(matches!(from_bytes(b"d:-INF;").unwrap(), PhpValue::Float(f) if f.is_infinite() && f.is_sign_negative()));
        assert!(matches!(from_bytes(b"d:NAN;").unwrap(), PhpValue::Float(f) if f.is_nan()));
    }

    #[test]
    fn test_string() {
        assert_eq!(
            from_bytes(b"s:0:\"\";").unwrap(),
            PhpValue::String(Cow::Borrowed(b""))
        );
        assert_eq!(
            from_bytes(b"s:5:\"hello\";").unwrap(),
            PhpValue::String(Cow::Borrowed(b"hello"))
        );
    }

    #[test]
    fn test_string_korean() {
        // "한글" = 6 bytes in UTF-8
        let korean = b"s:6:\"\xed\x95\x9c\xea\xb8\x80\";";
        let result = from_bytes(korean).unwrap();
        assert_eq!(result.as_str(), Some("한글"));
    }

    #[test]
    fn test_array_empty() {
        let result = from_bytes(b"a:0:{}").unwrap();
        assert_eq!(result, PhpValue::Array(vec![]));
    }

    #[test]
    fn test_array_indexed() {
        let result = from_bytes(b"a:2:{i:0;s:3:\"foo\";i:1;s:3:\"bar\";}").unwrap();
        if let PhpValue::Array(items) = result {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].0, PhpValue::Int(0));
            assert_eq!(items[0].1.as_str(), Some("foo"));
            assert_eq!(items[1].0, PhpValue::Int(1));
            assert_eq!(items[1].1.as_str(), Some("bar"));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_array_associative() {
        let result = from_bytes(b"a:2:{s:4:\"name\";s:5:\"Alice\";s:3:\"age\";i:30;}").unwrap();
        let map = result.as_string_map().unwrap();
        assert_eq!(map.get("name").unwrap().as_str(), Some("Alice"));
        assert_eq!(map.get("age").unwrap().as_int(), Some(30));
    }

    #[test]
    fn test_preprocess_db_escaped() {
        // DB-escaped format: "a:1:{s:3:""key"";s:5:""value"";}"
        // After unescape: a:1:{s:3:"key";s:5:"value";}
        let escaped = b"\"a:1:{s:3:\"\"key\"\";s:5:\"\"value\"\";}\"";
        let result = from_bytes(escaped).unwrap();
        let map = result.as_string_map().unwrap();
        assert_eq!(map.get("key").unwrap().as_str(), Some("value"));
    }

    #[test]
    fn test_object() {
        let data = br#"O:8:"stdClass":2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}"#;
        let result = from_bytes(data).unwrap();
        if let PhpValue::Object {
            class_name,
            properties,
        } = result
        {
            assert_eq!(class_name.as_ref(), "stdClass");
            assert_eq!(properties.len(), 2);
            assert_eq!(properties[0].name.as_ref(), "name");
            assert_eq!(properties[0].visibility, Visibility::Public);
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_enum() {
        let result = from_bytes(b"E:13:\"Status:Active\";").unwrap();
        if let PhpValue::Enum {
            class_name,
            case_name,
        } = result
        {
            assert_eq!(class_name.as_ref(), "Status");
            assert_eq!(case_name.as_ref(), "Active");
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_reference() {
        // R:1; references the first value (1-indexed)
        let result = from_bytes(b"R:1;").unwrap();
        assert_eq!(result, PhpValue::Reference(1));
    }

    #[test]
    fn test_custom_object() {
        // C:<namelen>:"<name>":<datalen>:{<data>}
        // "MyClass" = 7 bytes
        let data = b"C:7:\"MyClass\":5:{hello}";
        let result = from_bytes(data).unwrap();
        if let PhpValue::Object { class_name, properties } = result {
            assert_eq!(class_name.as_ref(), "MyClass");
            assert_eq!(properties.len(), 1);
            assert_eq!(properties[0].name.as_ref(), "__data");
            assert_eq!(properties[0].value.as_bytes(), Some(b"hello".as_slice()));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_object_private_protected() {
        // Object with private and protected properties
        // Private: \0ClassName\0propName  (\0Test\0priv = 10 bytes: 1+4+1+4)
        // Protected: \0*\0propName  (\0*\0prot = 7 bytes: 1+1+1+4)
        // "Test" = 4 bytes, "pub" = 3 bytes
        let data = b"O:4:\"Test\":3:{s:3:\"pub\";s:6:\"public\";s:10:\"\x00Test\x00priv\";s:7:\"private\";s:7:\"\x00*\x00prot\";s:9:\"protected\";}";
        let result = from_bytes(data).unwrap();
        if let PhpValue::Object { class_name, properties } = result {
            assert_eq!(class_name.as_ref(), "Test");
            assert_eq!(properties.len(), 3);

            // Public
            assert_eq!(properties[0].name.as_ref(), "pub");
            assert_eq!(properties[0].visibility, Visibility::Public);

            // Private
            assert_eq!(properties[1].name.as_ref(), "priv");
            assert_eq!(properties[1].visibility, Visibility::Private);
            assert_eq!(properties[1].declaring_class.as_ref().map(|s| s.as_ref()), Some("Test"));

            // Protected
            assert_eq!(properties[2].name.as_ref(), "prot");
            assert_eq!(properties[2].visibility, Visibility::Protected);
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_nested_array_depth() {
        // Test deeply nested arrays don't overflow
        let mut data = String::from("s:4:\"leaf\";");
        for _ in 0..100 {
            data = format!("a:1:{{s:1:\"k\";{}}}", data);
        }
        let result = from_bytes(data.as_bytes()).unwrap();
        assert!(result.is_array());
    }

    #[test]
    fn test_error_invalid_type() {
        let result = from_bytes(b"X:1;");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind, ErrorKind::UnknownType('X')));
    }

    #[test]
    fn test_error_truncated_string() {
        let result = from_bytes(b"s:10:\"hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_invalid_int() {
        let result = from_bytes(b"i:abc;");
        assert!(result.is_err());
    }

    #[test]
    fn test_special_string_binary() {
        // Binary data with null bytes
        let data = b"s:5:\"a\x00b\x00c\";";
        let result = from_bytes(data).unwrap();
        assert_eq!(result.as_bytes(), Some(b"a\x00b\x00c".as_slice()));
    }

    #[test]
    fn test_float_special_values() {
        // Test INF, -INF, NAN
        assert!(matches!(from_bytes(b"d:INF;").unwrap(), PhpValue::Float(f) if f.is_infinite() && f.is_sign_positive()));
        assert!(matches!(from_bytes(b"d:-INF;").unwrap(), PhpValue::Float(f) if f.is_infinite() && f.is_sign_negative()));
        assert!(matches!(from_bytes(b"d:NAN;").unwrap(), PhpValue::Float(f) if f.is_nan()));
    }

    #[test]
    fn test_empty_array_types() {
        // Empty indexed array
        assert_eq!(from_bytes(b"a:0:{}").unwrap(), PhpValue::Array(vec![]));
    }

    #[test]
    fn test_array_non_sequential_keys() {
        // Non-sequential integer keys should be preserved
        let result = from_bytes(b"a:2:{i:5;s:1:\"a\";i:10;s:1:\"b\";}").unwrap();
        if let PhpValue::Array(items) = result {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].0, PhpValue::Int(5));
            assert_eq!(items[1].0, PhpValue::Int(10));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_string_with_semicolon() {
        // String containing semicolon (common in serialized data)
        let result = from_bytes(b"s:11:\"hello;world\";").unwrap();
        assert_eq!(result.as_str(), Some("hello;world"));
    }

    #[test]
    fn test_string_with_quotes() {
        // String containing quotes (PHP uses length-based, no escape needed)
        // "say "hi"" = 8 bytes
        let result = from_bytes(b"s:8:\"say \"hi\"\";").unwrap();
        assert_eq!(result.as_str(), Some("say \"hi\""));
    }

    #[test]
    fn test_auto_fallback_encoding_mismatch() {
        // "한글" is 6 bytes in UTF-8, but declared as 4 (EUC-KR byte count)
        // This simulates data serialized with EUC-KR but stored as UTF-8
        // Default config (strict=false) should auto-recover
        let data = b"s:4:\"\xed\x95\x9c\xea\xb8\x80\";";
        let result = from_bytes(data).unwrap();
        assert_eq!(result.as_str(), Some("한글"));
    }

    #[test]
    fn test_auto_fallback_in_array() {
        // Array containing a string with encoding mismatch
        // "한글" = 6 bytes in UTF-8, declared as 4
        let data = b"a:1:{s:3:\"key\";s:4:\"\xed\x95\x9c\xea\xb8\x80\";}";
        let result = from_bytes(data).unwrap();
        let map = result.as_string_map().unwrap();
        assert_eq!(map.get("key").unwrap().as_str(), Some("한글"));
    }

    #[test]
    fn test_strict_mode_fails_on_mismatch() {
        // Same data but with strict=true should fail
        let data = b"s:4:\"\xed\x95\x9c\xea\xb8\x80\";";
        let config = ParserConfig {
            strict: true,
            ..Default::default()
        };
        let result = from_bytes_with_config(data, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_strict_false_is_default() {
        // Verify that strict=false (auto-fallback enabled) is the default
        let config = ParserConfig::default();
        assert!(!config.strict);
    }
}
