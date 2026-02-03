//! PHP value types.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

/// A PHP value that can be deserialized.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PhpValue<'a> {
    /// PHP null value.
    #[default]
    Null,

    /// PHP boolean value.
    Bool(bool),

    /// PHP integer value.
    Int(i64),

    /// PHP float/double value.
    Float(f64),

    /// PHP string value (may contain non-UTF8 bytes).
    /// Uses Cow for zero-copy when possible.
    String(Cow<'a, [u8]>),

    /// PHP array value (ordered map).
    /// PHP arrays maintain insertion order and can have mixed key types.
    Array(Vec<(PhpValue<'a>, PhpValue<'a>)>),

    /// PHP object value.
    Object {
        /// The class name of the object.
        class_name: Cow<'a, str>,
        /// Object properties with their names and values.
        properties: Vec<PhpProperty<'a>>,
    },

    /// PHP 8.1+ enum value.
    Enum {
        /// The enum class name.
        class_name: Cow<'a, str>,
        /// The enum case name.
        case_name: Cow<'a, str>,
    },

    /// Reference to another value (used during parsing).
    Reference(usize),
}

/// A PHP object property.
#[derive(Debug, Clone, PartialEq)]
pub struct PhpProperty<'a> {
    /// Property name.
    pub name: Cow<'a, str>,
    /// Property visibility.
    pub visibility: Visibility,
    /// For private properties, the class that declared it.
    pub declaring_class: Option<Cow<'a, str>>,
    /// Property value.
    pub value: PhpValue<'a>,
}

/// PHP property visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Public property.
    Public,
    /// Protected property (prefixed with `\0*\0`).
    Protected,
    /// Private property (prefixed with `\0ClassName\0`).
    Private,
}

impl<'a> PhpValue<'a> {
    /// Check if the value is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, PhpValue::Null)
    }

    /// Check if the value is a boolean.
    #[inline]
    pub fn is_bool(&self) -> bool {
        matches!(self, PhpValue::Bool(_))
    }

    /// Check if the value is an integer.
    #[inline]
    pub fn is_int(&self) -> bool {
        matches!(self, PhpValue::Int(_))
    }

    /// Check if the value is a float.
    #[inline]
    pub fn is_float(&self) -> bool {
        matches!(self, PhpValue::Float(_))
    }

    /// Check if the value is a string.
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, PhpValue::String(_))
    }

    /// Check if the value is an array.
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(self, PhpValue::Array(_))
    }

    /// Check if the value is an object.
    #[inline]
    pub fn is_object(&self) -> bool {
        matches!(self, PhpValue::Object { .. })
    }

    /// Get the value as a boolean.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PhpValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get the value as an integer.
    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            PhpValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get the value as a float.
    #[inline]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            PhpValue::Float(f) => Some(*f),
            PhpValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get the value as a byte slice.
    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            PhpValue::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    /// Get the value as a UTF-8 string.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PhpValue::String(s) => std::str::from_utf8(s.as_ref()).ok(),
            _ => None,
        }
    }

    /// Get the value as an array.
    #[inline]
    pub fn as_array(&self) -> Option<&[(PhpValue<'a>, PhpValue<'a>)]> {
        match self {
            PhpValue::Array(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    /// Convert the array to a HashMap if all keys are strings.
    pub fn as_string_map(&self) -> Option<HashMap<String, &PhpValue<'a>>> {
        let arr = self.as_array()?;
        let mut map = HashMap::with_capacity(arr.len());
        for (k, v) in arr {
            let key = match k {
                PhpValue::String(s) => String::from_utf8_lossy(s).into_owned(),
                PhpValue::Int(i) => i.to_string(),
                _ => return None,
            };
            map.insert(key, v);
        }
        Some(map)
    }

    /// Convert to an owned value that doesn't borrow from the input.
    pub fn into_owned(self) -> PhpValue<'static> {
        match self {
            PhpValue::Null => PhpValue::Null,
            PhpValue::Bool(b) => PhpValue::Bool(b),
            PhpValue::Int(i) => PhpValue::Int(i),
            PhpValue::Float(f) => PhpValue::Float(f),
            PhpValue::String(s) => PhpValue::String(Cow::Owned(s.into_owned())),
            PhpValue::Array(arr) => PhpValue::Array(
                arr.into_iter()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect(),
            ),
            PhpValue::Object {
                class_name,
                properties,
            } => PhpValue::Object {
                class_name: Cow::Owned(class_name.into_owned()),
                properties: properties
                    .into_iter()
                    .map(|p| PhpProperty {
                        name: Cow::Owned(p.name.into_owned()),
                        visibility: p.visibility,
                        declaring_class: p.declaring_class.map(|c| Cow::Owned(c.into_owned())),
                        value: p.value.into_owned(),
                    })
                    .collect(),
            },
            PhpValue::Enum {
                class_name,
                case_name,
            } => PhpValue::Enum {
                class_name: Cow::Owned(class_name.into_owned()),
                case_name: Cow::Owned(case_name.into_owned()),
            },
            PhpValue::Reference(r) => PhpValue::Reference(r),
        }
    }

    /// Get a type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            PhpValue::Null => "null",
            PhpValue::Bool(_) => "boolean",
            PhpValue::Int(_) => "integer",
            PhpValue::Float(_) => "float",
            PhpValue::String(_) => "string",
            PhpValue::Array(_) => "array",
            PhpValue::Object { .. } => "object",
            PhpValue::Enum { .. } => "enum",
            PhpValue::Reference(_) => "reference",
        }
    }
}

impl fmt::Display for PhpValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhpValue::Null => write!(f, "null"),
            PhpValue::Bool(b) => write!(f, "{}", b),
            PhpValue::Int(i) => write!(f, "{}", i),
            PhpValue::Float(fl) => write!(f, "{}", fl),
            PhpValue::String(s) => {
                match std::str::from_utf8(s) {
                    Ok(s) => write!(f, "\"{}\"", s),
                    Err(_) => write!(f, "<binary {} bytes>", s.len()),
                }
            }
            PhpValue::Array(arr) => {
                write!(f, "[")?;
                for (i, (k, v)) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} => {}", k, v)?;
                }
                write!(f, "]")
            }
            PhpValue::Object { class_name, .. } => write!(f, "{}{{...}}", class_name),
            PhpValue::Enum {
                class_name,
                case_name,
            } => write!(f, "{}::{}", class_name, case_name),
            PhpValue::Reference(idx) => write!(f, "&{}", idx),
        }
    }
}

