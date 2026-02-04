"""Type stubs for php_deserialize."""

from typing import Any, Literal, overload

__version__: str

class PhpDeserializeError(Exception):
    """Exception raised when PHP deserialization fails."""
    ...

@overload
def loads(
    data: bytes,
    *,
    errors: Literal["strict"] = ...,
    auto_unescape: bool = ...,
) -> dict[str, Any] | list[Any] | str | int | float | bool | None: ...

@overload
def loads(
    data: bytes,
    *,
    errors: Literal["replace"],
    auto_unescape: bool = ...,
) -> dict[str, Any] | list[Any] | str | int | float | bool | None: ...

@overload
def loads(
    data: bytes,
    *,
    errors: Literal["bytes"],
    auto_unescape: bool = ...,
) -> dict[str, Any] | list[Any] | str | bytes | int | float | bool | None: ...

def loads(
    data: bytes,
    *,
    errors: Literal["strict", "replace", "bytes"] = "replace",
    auto_unescape: bool = True,
    strict: bool = False,
) -> dict[str, Any] | list[Any] | str | bytes | int | float | bool | None:
    """
    Deserialize PHP serialized data to a Python object.

    Args:
        data: Bytes containing PHP serialized data
        errors: Error handling mode for invalid UTF-8:
            - "strict": Raise an exception
            - "replace": Replace invalid bytes with replacement character (default)
            - "bytes": Return bytes instead of string for binary data
        auto_unescape: Automatically detect and unescape DB-exported strings
        strict: Disable automatic fallback for string length mismatches (default: False)

    Returns:
        The deserialized Python object

    Raises:
        PhpDeserializeError: If the data cannot be parsed
    """
    ...

def loads_json(
    data: bytes,
    *,
    auto_unescape: bool = True,
    strict: bool = False,
) -> str:
    """
    Deserialize PHP serialized data directly to a JSON string.

    This is optimized for cases where you need JSON output (e.g., Databricks).

    Args:
        data: Bytes containing PHP serialized data
        auto_unescape: Automatically detect and unescape DB-exported strings
        strict: Disable automatic fallback for string length mismatches (default: False)

    Returns:
        A JSON string representation of the deserialized data

    Raises:
        PhpDeserializeError: If the data cannot be parsed
    """
    ...

def is_serialized(data: bytes) -> bool:
    """
    Check if data looks like PHP serialized format.

    Args:
        data: Bytes to check

    Returns:
        True if the data appears to be PHP serialized
    """
    ...

def preprocess(data: bytes) -> bytes:
    """
    Preprocess data to unescape DB-exported strings.

    Args:
        data: Bytes that may be DB-escaped

    Returns:
        Unescaped bytes suitable for parsing
    """
    ...

def version() -> str:
    """Get the version of the library."""
    ...
