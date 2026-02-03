//! JSON conversion for PHP values.
//!
//! This module provides conversion from `PhpValue` to JSON using serde_json.
//! Enable the `serde` feature to use this module.

use serde_json::{json, Map, Value as JsonValue};

use crate::types::PhpValue;

/// Convert a PHP value to a JSON value.
///
/// # Mapping Rules
///
/// | PHP Type | JSON Type |
/// |----------|-----------|
/// | `null` | `null` |
/// | `bool` | `boolean` |
/// | `int` | `number` |
/// | `float` | `number` (or `null` for NaN) |
/// | `string` | `string` (lossy UTF-8 conversion) |
/// | `array` (indexed) | `array` |
/// | `array` (associative) | `object` |
/// | `object` | `object` with `__class__` field |
/// | `enum` | `string` ("ClassName::CaseName") |
/// | `reference` | `null` (references not resolved) |
///
/// # Example
///
/// ```rust
/// use php_deserialize_core::{from_bytes, to_json};
///
/// let data = br#"a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}"#;
/// let php_value = from_bytes(data).unwrap();
/// let json = to_json(&php_value);
/// assert_eq!(json, serde_json::json!({"name": "Alice", "age": 30}));
/// ```
pub fn to_json(value: &PhpValue) -> JsonValue {
    match value {
        PhpValue::Null => JsonValue::Null,
        PhpValue::Bool(b) => JsonValue::Bool(*b),
        PhpValue::Int(i) => json!(*i),
        PhpValue::Float(f) => {
            if f.is_nan() {
                JsonValue::Null
            } else if f.is_infinite() {
                if f.is_sign_positive() {
                    json!("Infinity")
                } else {
                    json!("-Infinity")
                }
            } else {
                json!(*f)
            }
        }
        PhpValue::String(s) => {
            let string = String::from_utf8_lossy(s);
            JsonValue::String(string.into_owned())
        }
        PhpValue::Array(items) => {
            // Check if this is an indexed array (sequential integer keys starting from 0)
            let is_indexed = items.iter().enumerate().all(|(i, (k, _))| {
                matches!(k, PhpValue::Int(idx) if *idx as usize == i)
            });

            if is_indexed {
                // Convert to JSON array
                let arr: Vec<JsonValue> = items.iter().map(|(_, v)| to_json(v)).collect();
                JsonValue::Array(arr)
            } else {
                // Convert to JSON object
                let mut map = Map::new();
                for (k, v) in items {
                    let key = match k {
                        PhpValue::String(s) => String::from_utf8_lossy(s).into_owned(),
                        PhpValue::Int(i) => i.to_string(),
                        _ => continue, // Skip invalid keys
                    };
                    map.insert(key, to_json(v));
                }
                JsonValue::Object(map)
            }
        }
        PhpValue::Object {
            class_name,
            properties,
        } => {
            let mut map = Map::new();
            map.insert("__class__".to_string(), json!(class_name.as_ref()));

            for prop in properties {
                let key = match prop.visibility {
                    crate::types::Visibility::Private => {
                        if let Some(ref class) = prop.declaring_class {
                            format!("{}::{}", class, prop.name)
                        } else {
                            prop.name.to_string()
                        }
                    }
                    crate::types::Visibility::Protected => {
                        format!("*{}", prop.name)
                    }
                    crate::types::Visibility::Public => prop.name.to_string(),
                };
                map.insert(key, to_json(&prop.value));
            }

            JsonValue::Object(map)
        }
        PhpValue::Enum {
            class_name,
            case_name,
        } => {
            json!(format!("{}::{}", class_name, case_name))
        }
        PhpValue::Reference(idx) => {
            // References are not resolved, just mark them
            json!({ "__ref__": idx })
        }
    }
}

/// Convert a PHP value to a JSON string.
///
/// # Example
///
/// ```rust
/// use php_deserialize_core::{from_bytes, json::to_json_string};
///
/// let data = br#"a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}"#;
/// let php_value = from_bytes(data).unwrap();
/// let json_str = to_json_string(&php_value).unwrap();
/// // JSON key order is not guaranteed, so check contents
/// assert!(json_str.contains(r#""name":"Alice""#));
/// assert!(json_str.contains(r#""age":30"#));
/// ```
pub fn to_json_string(value: &PhpValue) -> serde_json::Result<String> {
    let json = to_json(value);
    serde_json::to_string(&json)
}

/// Convert a PHP value to a pretty-printed JSON string.
pub fn to_json_string_pretty(value: &PhpValue) -> serde_json::Result<String> {
    let json = to_json(value);
    serde_json::to_string_pretty(&json)
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;
    use crate::from_bytes;

    #[test]
    fn test_simple_types() {
        assert_eq!(to_json(&PhpValue::Null), JsonValue::Null);
        assert_eq!(to_json(&PhpValue::Bool(true)), JsonValue::Bool(true));
        assert_eq!(to_json(&PhpValue::Int(42)), json!(42));
        assert_eq!(to_json(&PhpValue::Float(3.14)), json!(3.14));
    }

    #[test]
    fn test_indexed_array() {
        let data = b"a:2:{i:0;s:3:\"foo\";i:1;s:3:\"bar\";}";
        let value = from_bytes(data).unwrap();
        assert_eq!(to_json(&value), json!(["foo", "bar"]));
    }

    #[test]
    fn test_associative_array() {
        let data = b"a:2:{s:4:\"name\";s:5:\"Alice\";s:3:\"age\";i:30;}";
        let value = from_bytes(data).unwrap();
        assert_eq!(to_json(&value), json!({"name": "Alice", "age": 30}));
    }

    #[test]
    fn test_mixed_array() {
        // Non-sequential keys -> object
        let data = b"a:2:{i:0;s:3:\"foo\";i:5;s:3:\"bar\";}";
        let value = from_bytes(data).unwrap();
        assert_eq!(to_json(&value), json!({"0": "foo", "5": "bar"}));
    }

    #[test]
    fn test_nested() {
        let data = b"a:1:{s:4:\"user\";a:2:{s:4:\"name\";s:5:\"Alice\";s:3:\"age\";i:30;}}";
        let value = from_bytes(data).unwrap();
        assert_eq!(
            to_json(&value),
            json!({"user": {"name": "Alice", "age": 30}})
        );
    }
}
