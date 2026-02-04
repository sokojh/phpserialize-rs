//! Python bindings for php-deserialize-core.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

use php_deserialize_core::{
    from_bytes_with_config, json::to_json_string, ParserConfig, PhpValue,
};

pyo3::create_exception!(php_deserialize, PhpDeserializeError, pyo3::exceptions::PyException);

/// Convert a PhpValue to a Python object.
fn php_value_to_python(py: Python<'_>, value: &PhpValue, errors: &str) -> PyResult<PyObject> {
    match value {
        PhpValue::Null => Ok(py.None()),
        PhpValue::Bool(b) => Ok(b.to_object(py)),
        PhpValue::Int(i) => Ok(i.to_object(py)),
        PhpValue::Float(f) => Ok(f.to_object(py)),
        PhpValue::String(s) => {
            // Try to decode as UTF-8
            match std::str::from_utf8(s) {
                Ok(string) => Ok(string.to_object(py)),
                Err(_) => {
                    match errors {
                        "strict" => Err(PyValueError::new_err("Invalid UTF-8 in string")),
                        "bytes" => Ok(PyBytes::new_bound(py, s).to_object(py)),
                        // "replace" and any other value: replace invalid bytes with replacement character
                        _ => {
                            let string = String::from_utf8_lossy(s);
                            Ok(string.to_object(py))
                        }
                    }
                }
            }
        }
        PhpValue::Array(items) => {
            // Check if this is an indexed array
            let is_indexed = items.iter().enumerate().all(|(i, (k, _))| {
                matches!(k, PhpValue::Int(idx) if *idx as usize == i)
            });

            if is_indexed {
                let list = PyList::empty_bound(py);
                for (_, v) in items {
                    list.append(php_value_to_python(py, v, errors)?)?;
                }
                Ok(list.to_object(py))
            } else {
                let dict = PyDict::new_bound(py);
                for (k, v) in items {
                    let key: PyObject = match k {
                        PhpValue::String(s) => {
                            match std::str::from_utf8(s) {
                                Ok(string) => string.to_object(py),
                                Err(_) => {
                                    let string = String::from_utf8_lossy(s);
                                    string.to_object(py)
                                }
                            }
                        }
                        PhpValue::Int(i) => i.to_object(py),
                        _ => continue,
                    };
                    dict.set_item(key, php_value_to_python(py, v, errors)?)?;
                }
                Ok(dict.to_object(py))
            }
        }
        PhpValue::Object {
            class_name,
            properties,
        } => {
            let dict = PyDict::new_bound(py);
            dict.set_item("__class__", class_name.as_ref())?;

            for prop in properties {
                let key = match prop.visibility {
                    php_deserialize_core::Visibility::Private => {
                        if let Some(ref class) = prop.declaring_class {
                            format!("{}::{}", class, prop.name)
                        } else {
                            prop.name.to_string()
                        }
                    }
                    php_deserialize_core::Visibility::Protected => {
                        format!("*{}", prop.name)
                    }
                    php_deserialize_core::Visibility::Public => prop.name.to_string(),
                };
                dict.set_item(key, php_value_to_python(py, &prop.value, errors)?)?;
            }

            Ok(dict.to_object(py))
        }
        PhpValue::Enum {
            class_name,
            case_name,
        } => {
            // Return as "ClassName::CaseName" string
            let value = format!("{}::{}", class_name, case_name);
            Ok(value.to_object(py))
        }
        PhpValue::Reference(idx) => {
            // References not resolved, return a dict marking it
            let dict = PyDict::new_bound(py);
            dict.set_item("__ref__", *idx)?;
            Ok(dict.to_object(py))
        }
    }
}

/// Deserialize PHP serialized data to a Python object.
///
/// Args:
///     data: Bytes containing PHP serialized data
///     errors: Error handling mode for invalid UTF-8:
///         - "strict": Raise an exception
///         - "replace": Replace invalid bytes with replacement character (default)
///         - "bytes": Return bytes instead of string for binary data
///     auto_unescape: Automatically detect and unescape DB-exported strings (default: True)
///     strict: Disable automatic fallback for string length mismatches (default: False)
///         When False (default), the parser will automatically try lenient recovery
///         if strict parsing fails due to string length mismatches. This is useful
///         for data that was serialized with a different encoding (e.g., EUC-KR)
///         but stored as UTF-8.
///         When True, the parser will fail immediately on string length mismatches
///         without attempting recovery.
///
/// Returns:
///     The deserialized Python object (dict, list, str, int, float, bool, or None)
///
/// Raises:
///     PhpDeserializeError: If the data cannot be parsed
///
/// Example:
///     >>> from php_deserialize import loads
///     >>> loads(b'a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}')
///     {'name': 'Alice', 'age': 30}
///
///     >>> # Automatic fallback handles encoding mismatch (no option needed!)
///     >>> loads(b'a:1:{s:4:"key";s:10:"\xed\x95\x9c\xea\xb8\x80";}')
///     {'key': '한글'}
///
///     >>> # Use strict=True to disable automatic fallback
///     >>> loads(b'a:1:{s:4:"key";s:10:"\xed\x95\x9c\xea\xb8\x80";}', strict=True)
///     # Raises PhpDeserializeError
#[pyfunction]
#[pyo3(signature = (data, *, errors="replace", auto_unescape=true, strict=false))]
fn loads(
    py: Python<'_>,
    data: &[u8],
    errors: &str,
    auto_unescape: bool,
    strict: bool,
) -> PyResult<PyObject> {
    let config = ParserConfig {
        auto_unescape,
        strict,
        ..Default::default()
    };

    let value = from_bytes_with_config(data, config).map_err(|e| {
        PhpDeserializeError::new_err(format!("{}", e))
    })?;

    php_value_to_python(py, &value, errors)
}

/// Deserialize PHP serialized data directly to a JSON string.
///
/// This is optimized for cases where you need JSON output (e.g., Databricks).
/// It avoids the overhead of creating intermediate Python objects.
///
/// Args:
///     data: Bytes containing PHP serialized data
///     auto_unescape: Automatically detect and unescape DB-exported strings (default: True)
///     strict: Disable automatic fallback for string length mismatches (default: False)
///         When False (default), automatic recovery is attempted for encoding mismatches.
///         When True, the parser fails immediately on string length mismatches.
///
/// Returns:
///     A JSON string representation of the deserialized data
///
/// Raises:
///     PhpDeserializeError: If the data cannot be parsed
///
/// Example:
///     >>> from php_deserialize import loads_json
///     >>> loads_json(b'a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}')
///     '{"name":"Alice","age":30}'
#[pyfunction]
#[pyo3(signature = (data, *, auto_unescape=true, strict=false))]
fn loads_json(data: &[u8], auto_unescape: bool, strict: bool) -> PyResult<String> {
    let config = ParserConfig {
        auto_unescape,
        strict,
        ..Default::default()
    };

    let value = from_bytes_with_config(data, config).map_err(|e| {
        PhpDeserializeError::new_err(format!("{}", e))
    })?;

    to_json_string(&value).map_err(|e| PhpDeserializeError::new_err(format!("{}", e)))
}

/// Check if data looks like PHP serialized format.
///
/// This is a quick check that doesn't fully validate the data.
///
/// Args:
///     data: Bytes to check
///
/// Returns:
///     True if the data appears to be PHP serialized, False otherwise
///
/// Example:
///     >>> from php_deserialize import is_serialized
///     >>> is_serialized(b'a:1:{i:0;s:3:"foo";}')
///     True
///     >>> is_serialized(b'not serialized')
///     False
#[pyfunction]
fn is_serialized(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    // Check first character for valid type markers
    let first = data[0];
    matches!(
        first,
        b'N' | b'b' | b'i' | b'd' | b's' | b'a' | b'O' | b'C' | b'R' | b'r' | b'E'
    )
}

/// Preprocess data to unescape DB-exported strings.
///
/// This function handles strings that have been double-escaped during
/// database export (e.g., quotes becoming "").
///
/// Args:
///     data: Bytes that may be DB-escaped
///
/// Returns:
///     Unescaped bytes suitable for parsing
///
/// Example:
///     >>> from php_deserialize import preprocess
///     >>> preprocess(b'"a:1:{s:3:""key"";s:5:""value"";}"')
///     b'a:1:{s:3:"key";s:5:"value";}'
#[pyfunction]
fn preprocess<'py>(py: Python<'py>, data: &[u8]) -> Bound<'py, PyBytes> {
    let result = match php_deserialize_core::preprocess(data) {
        std::borrow::Cow::Borrowed(b) => b.to_vec(),
        std::borrow::Cow::Owned(o) => o,
    };
    PyBytes::new_bound(py, &result)
}

/// Get the version of the library.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// PHP deserialize module for Python.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("PhpDeserializeError", m.py().get_type_bound::<PhpDeserializeError>())?;
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    m.add_function(wrap_pyfunction!(loads_json, m)?)?;
    m.add_function(wrap_pyfunction!(is_serialized, m)?)?;
    m.add_function(wrap_pyfunction!(preprocess, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
