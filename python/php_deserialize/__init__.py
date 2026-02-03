"""
High-performance PHP serialize/unserialize parser.

This module provides functions to deserialize PHP serialized data into Python objects.
It is implemented in Rust for maximum performance.

Example:
    >>> from php_deserialize import loads
    >>> loads(b'a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}')
    {'name': 'Alice', 'age': 30}

    >>> from php_deserialize import loads_json
    >>> loads_json(b'a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}')
    '{"name":"Alice","age":30}'
"""

from php_deserialize._core import (
    PhpDeserializeError,
    is_serialized,
    loads,
    loads_json,
    preprocess,
    version,
)

__all__ = [
    "PhpDeserializeError",
    "is_serialized",
    "loads",
    "loads_json",
    "preprocess",
    "version",
    "__version__",
]

__version__ = version()
