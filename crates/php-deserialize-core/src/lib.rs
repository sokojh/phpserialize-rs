//! High-performance PHP serialize/unserialize parser.
//!
//! This crate provides a zero-copy parser for PHP's serialize format with support for
//! all PHP types including objects, references, and PHP 8.1+ enums.
//!
//! # Features
//!
//! - **Zero-copy parsing** - Minimal memory allocations for maximum performance
//! - **Full PHP serialize support** - All types including objects, references, and enums
//! - **UTF-8 aware** - Proper handling of multi-byte characters
//! - **Auto-unescape** - Automatic detection and handling of DB-escaped strings
//! - **Detailed errors** - Precise error positions and messages
//!
//! # Quick Start
//!
//! ```rust
//! use php_deserialize_core::{from_bytes, PhpValue};
//!
//! let data = br#"a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}"#;
//! let value = from_bytes(data).unwrap();
//!
//! if let PhpValue::Array(items) = value {
//!     for (key, val) in items {
//!         println!("{} => {}", key, val);
//!     }
//! }
//! ```
//!
//! # Handling DB-Escaped Strings
//!
//! The parser automatically detects and handles strings that have been escaped
//! during database export (e.g., double quotes becoming `""`):
//!
//! ```rust
//! use php_deserialize_core::from_bytes;
//!
//! // DB-escaped format
//! let escaped = br#""a:1:{s:3:""key"";s:5:""value"";}""#;
//! let value = from_bytes(escaped).unwrap();
//! // Works correctly!
//! ```
//!
//! # Supported Types
//!
//! | PHP Type | Rust Type |
//! |----------|-----------|
//! | `null` | `PhpValue::Null` |
//! | `bool` | `PhpValue::Bool(bool)` |
//! | `int` | `PhpValue::Int(i64)` |
//! | `float` | `PhpValue::Float(f64)` |
//! | `string` | `PhpValue::String(Cow<[u8]>)` |
//! | `array` | `PhpValue::Array(Vec<(PhpValue, PhpValue)>)` |
//! | `object` | `PhpValue::Object { class_name, properties }` |
//! | `enum` | `PhpValue::Enum { class_name, case_name }` |
//! | `reference` | `PhpValue::Reference(usize)` |

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::inline_always)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::range_plus_one)]

pub mod error;
pub mod parser;
pub mod types;

#[cfg(feature = "serde")]
pub mod json;

pub use error::{ErrorKind, PhpDeserializeError, Result};
pub use parser::{from_bytes, from_bytes_with_config, preprocess, Parser, ParserConfig};
pub use types::{PhpProperty, PhpValue, Visibility};

#[cfg(feature = "serde")]
pub use json::to_json;
