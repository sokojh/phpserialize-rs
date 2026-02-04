# phpserialize-rs

[![Crates.io](https://img.shields.io/crates/v/php-deserialize-core.svg)](https://crates.io/crates/php-deserialize-core)
[![PyPI](https://img.shields.io/pypi/v/phpserialize-rs.svg)](https://pypi.org/project/phpserialize-rs/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/sokojh/phpserialize-rs/workflows/CI/badge.svg)](https://github.com/sokojh/phpserialize-rs/actions)

High-performance PHP serialize/unserialize parser written in Rust with Python bindings.

## Features

- **Zero-copy parsing** - Minimal memory allocations for maximum performance
- **Full PHP serialize support** - All types including objects, references, and PHP 8.1 enums
- **UTF-8 aware** - Proper handling of multi-byte characters (Korean, Chinese, etc.)
- **Auto-unescape** - Automatic detection and handling of DB-escaped strings
- **Auto-fallback** - Automatic recovery from encoding mismatches (e.g., EUC-KR to UTF-8)
- **Error recovery** - Configurable error handling for malformed data
- **PySpark integration** - Ready-to-use Arrow-optimized UDFs for Databricks/Spark workloads

## Installation

### Python

```bash
pip install phpserialize-rs
```

### Rust

```toml
[dependencies]
php-deserialize-core = "0.1"
```

## Quick Start

### Python

```python
from php_deserialize import loads, loads_json

# Basic usage
data = b'a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}'
result = loads(data)
print(result)  # {'name': 'Alice', 'age': 30}

# Direct JSON conversion (optimized for Databricks)
json_str = loads_json(data)
print(json_str)  # {"name":"Alice","age":30}

# Handle DB-escaped strings automatically
escaped = b'"a:1:{s:4:""key"";s:5:""value"";}"'
result = loads(escaped)  # Auto-unescapes
print(result)  # {'key': 'value'}

# Auto-fallback for encoding mismatches (no option needed!)
# Handles data serialized with EUC-KR but stored as UTF-8
mismatch = b's:4:"\xed\x95\x9c\xea\xb8\x80";'  # "한글" with wrong length
result = loads(mismatch)  # Automatically recovers
print(result)  # '한글'

# Strict mode (disable auto-fallback)
result = loads(data, strict=True)  # Fails on length mismatch

# Error handling options
result = loads(data, errors="replace")  # Replace invalid UTF-8
result = loads(data, errors="bytes")    # Return bytes for invalid UTF-8
```

### PySpark / Databricks

```python
from php_deserialize.spark import php_to_json
from pyspark.sql.functions import get_json_object

# Convert PHP serialize to JSON (Arrow-optimized UDF)
df = spark.table("bronze.my_table")
df = df.withColumn("data_json", php_to_json("serialized_column"))

# Extract fields from JSON
df = df.withColumn("name", get_json_object("data_json", "$.name"))
df = df.withColumn("age", get_json_object("data_json", "$.age"))

df.display()
```

For Databricks installation:

```python
%pip install phpserialize-rs
```

### Rust

```rust
use php_deserialize_core::{from_bytes, PhpValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = br#"a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}"#;
    let value = from_bytes(data)?;

    if let PhpValue::Array(items) = value {
        for (key, val) in items {
            println!("{:?} => {:?}", key, val);
        }
    }

    Ok(())
}
```

## Supported PHP Types

| Type | PHP Format | Example |
|------|------------|---------|
| Null | `N;` | `N;` |
| Boolean | `b:0;` / `b:1;` | `b:1;` |
| Integer | `i:<value>;` | `i:42;` |
| Float | `d:<value>;` | `d:3.14;` |
| String | `s:<len>:"<data>";` | `s:5:"hello";` |
| Array | `a:<count>:{...}` | `a:1:{i:0;s:3:"foo";}` |
| Object | `O:<len>:"<class>":<count>:{...}` | Object with properties |
| Reference | `R:<index>;` / `r:<index>;` | Circular references |
| Enum (PHP 8.1+) | `E:<len>:"<Class:Case>";` | `E:10:"Status:Active";` |

## Performance

Benchmarked on Apple M1 Pro:

| Operation | Throughput |
|-----------|------------|
| Simple array | ~1.5 GB/s |
| Nested structure | ~800 MB/s |
| Large string | ~2.0 GB/s |

Compared to `php2json` (Python):
- **10-50x faster** for typical workloads
- **100x faster** for large arrays

## Error Handling

The library provides detailed error messages for debugging:

```python
from php_deserialize import loads, PhpDeserializeError

try:
    loads(b"invalid data")
except PhpDeserializeError as e:
    print(f"Parse error at position {e.position}: {e.message}")
```

## DB Escape Handling

When data is exported from databases (MySQL, PostgreSQL), strings may be double-quoted and escaped:

```
Original: a:1:{s:4:"key";s:5:"value";}
DB Export: "a:1:{s:4:""key"";s:5:""value"";}"
```

The library automatically detects and handles this format:

```python
# Both work identically
loads(b'a:1:{s:4:"key";s:5:"value";}')
loads(b'"a:1:{s:4:""key"";s:5:""value"";}"')
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)

at your option.
