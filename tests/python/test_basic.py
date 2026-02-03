"""Basic tests for php_deserialize."""

import pytest


def test_import():
    """Test that the module can be imported."""
    from php_deserialize import loads, loads_json, is_serialized, preprocess, version
    assert callable(loads)
    assert callable(loads_json)
    assert callable(is_serialized)
    assert callable(preprocess)
    assert isinstance(version(), str)


def test_version():
    """Test version string."""
    from php_deserialize import __version__, version
    assert __version__ == version()
    assert "." in __version__


class TestNull:
    """Tests for null values."""

    def test_null(self):
        from php_deserialize import loads
        assert loads(b"N;") is None


class TestBool:
    """Tests for boolean values."""

    def test_true(self):
        from php_deserialize import loads
        assert loads(b"b:1;") is True

    def test_false(self):
        from php_deserialize import loads
        assert loads(b"b:0;") is False


class TestInt:
    """Tests for integer values."""

    def test_zero(self):
        from php_deserialize import loads
        assert loads(b"i:0;") == 0

    def test_positive(self):
        from php_deserialize import loads
        assert loads(b"i:42;") == 42

    def test_negative(self):
        from php_deserialize import loads
        assert loads(b"i:-123;") == -123

    def test_large(self):
        from php_deserialize import loads
        assert loads(b"i:9223372036854775807;") == 9223372036854775807


class TestFloat:
    """Tests for float values."""

    def test_zero(self):
        from php_deserialize import loads
        assert loads(b"d:0;") == 0.0

    def test_positive(self):
        from php_deserialize import loads
        assert loads(b"d:3.14;") == 3.14

    def test_negative(self):
        from php_deserialize import loads
        assert loads(b"d:-2.5;") == -2.5

    def test_scientific(self):
        from php_deserialize import loads
        assert loads(b"d:1.5e10;") == 1.5e10

    def test_infinity(self):
        from php_deserialize import loads
        import math
        result = loads(b"d:INF;")
        assert math.isinf(result) and result > 0

    def test_negative_infinity(self):
        from php_deserialize import loads
        import math
        result = loads(b"d:-INF;")
        assert math.isinf(result) and result < 0

    def test_nan(self):
        from php_deserialize import loads
        import math
        assert math.isnan(loads(b"d:NAN;"))


class TestString:
    """Tests for string values."""

    def test_empty(self):
        from php_deserialize import loads
        assert loads(b's:0:"";') == ""

    def test_simple(self):
        from php_deserialize import loads
        assert loads(b's:5:"hello";') == "hello"

    def test_with_spaces(self):
        from php_deserialize import loads
        assert loads(b's:11:"hello world";') == "hello world"

    def test_korean(self):
        from php_deserialize import loads
        # "한글" = 6 bytes in UTF-8
        result = loads(b's:6:"\xed\x95\x9c\xea\xb8\x80";')
        assert result == "한글"

    def test_special_chars(self):
        from php_deserialize import loads
        assert loads(b's:13:"hello & world";') == "hello & world"

    def test_quotes(self):
        from php_deserialize import loads
        # PHP uses length-based strings, quotes inside don't need escaping
        # 'say "hi"' = 8 bytes
        assert loads(b's:8:"say "hi"";') == 'say "hi"'


class TestArray:
    """Tests for array values."""

    def test_empty(self):
        from php_deserialize import loads
        assert loads(b"a:0:{}") == []

    def test_indexed(self):
        from php_deserialize import loads
        result = loads(b'a:2:{i:0;s:3:"foo";i:1;s:3:"bar";}')
        assert result == ["foo", "bar"]

    def test_associative(self):
        from php_deserialize import loads
        result = loads(b'a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}')
        assert result == {"name": "Alice", "age": 30}

    def test_nested(self):
        from php_deserialize import loads
        data = b'a:1:{s:4:"user";a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}}'
        result = loads(data)
        assert result == {"user": {"name": "Alice", "age": 30}}

    def test_mixed_keys(self):
        from php_deserialize import loads
        # Non-sequential integer keys become a dict
        result = loads(b'a:2:{i:0;s:3:"foo";i:5;s:3:"bar";}')
        assert result == {0: "foo", 5: "bar"}


class TestObject:
    """Tests for object values."""

    def test_stdclass(self):
        from php_deserialize import loads
        data = b'O:8:"stdClass":2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}'
        result = loads(data)
        assert result["__class__"] == "stdClass"
        assert result["name"] == "Alice"
        assert result["age"] == 30


class TestEnum:
    """Tests for PHP 8.1+ enum values."""

    def test_simple_enum(self):
        from php_deserialize import loads
        result = loads(b'E:13:"Status:Active";')
        assert result == "Status::Active"

    def test_backed_enum(self):
        from php_deserialize import loads
        # "Priority:High" = 13 bytes
        result = loads(b'E:13:"Priority:High";')
        assert result == "Priority::High"


class TestDbEscape:
    """Tests for DB-escaped strings."""

    def test_double_quoted(self):
        from php_deserialize import loads
        # DB-escaped format: "a:1:{s:3:""key"";s:5:""value"";}"
        # "key" = 3 bytes
        escaped = b'"a:1:{s:3:""key"";s:5:""value"";}"'
        result = loads(escaped)
        assert result == {"key": "value"}

    def test_preprocess(self):
        from php_deserialize import preprocess
        # "key" = 3 bytes
        escaped = b'"a:1:{s:3:""key"";s:5:""value"";}"'
        unescaped = preprocess(escaped)
        assert unescaped == b'a:1:{s:3:"key";s:5:"value";}'

    def test_no_unescape_normal(self):
        from php_deserialize import preprocess
        # "key" = 3 bytes
        normal = b'a:1:{s:3:"key";s:5:"value";}'
        assert preprocess(normal) == normal


class TestJson:
    """Tests for JSON output."""

    def test_simple(self):
        from php_deserialize import loads_json
        import json
        result = loads_json(b'a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}')
        parsed = json.loads(result)
        assert parsed == {"name": "Alice", "age": 30}

    def test_array(self):
        from php_deserialize import loads_json
        import json
        result = loads_json(b'a:2:{i:0;s:3:"foo";i:1;s:3:"bar";}')
        parsed = json.loads(result)
        assert parsed == ["foo", "bar"]


class TestIsSerialized:
    """Tests for is_serialized function."""

    def test_valid(self):
        from php_deserialize import is_serialized
        assert is_serialized(b"N;") is True
        assert is_serialized(b"b:1;") is True
        assert is_serialized(b"i:42;") is True
        assert is_serialized(b's:5:"hello";') is True
        assert is_serialized(b"a:0:{}") is True

    def test_invalid(self):
        from php_deserialize import is_serialized
        assert is_serialized(b"") is False
        assert is_serialized(b"not serialized") is False
        assert is_serialized(b"hello world") is False


class TestErrors:
    """Tests for error handling."""

    def test_invalid_data(self):
        from php_deserialize import loads, PhpDeserializeError
        with pytest.raises(PhpDeserializeError):
            loads(b"invalid")

    def test_truncated(self):
        from php_deserialize import loads, PhpDeserializeError
        with pytest.raises(PhpDeserializeError):
            loads(b"s:10:\"hello")

    def test_error_mode_replace(self):
        from php_deserialize import loads
        # Invalid UTF-8 with replace mode (default)
        data = b's:4:"\xff\xfe\xfd\xfc";'
        result = loads(data, errors="replace")
        assert isinstance(result, str)

    def test_error_mode_bytes(self):
        from php_deserialize import loads
        # Invalid UTF-8 with bytes mode
        data = b's:4:"\xff\xfe\xfd\xfc";'
        result = loads(data, errors="bytes")
        assert isinstance(result, bytes)
