"""Additional edge case tests for comprehensive coverage.

These tests cover extreme cases, malformed input, and boundary conditions.
"""

import math

import pytest


class TestMalformedInput:
    """Tests for malformed/invalid input handling."""

    def test_empty_input(self):
        """Empty input should raise error."""
        from php_deserialize import PhpDeserializeError, loads

        with pytest.raises(PhpDeserializeError):
            loads(b"")

    def test_truncated_null(self):
        """Truncated null should raise error."""
        from php_deserialize import PhpDeserializeError, loads

        with pytest.raises(PhpDeserializeError):
            loads(b"N")

    def test_truncated_bool(self):
        """Truncated boolean should raise error."""
        from php_deserialize import PhpDeserializeError, loads

        with pytest.raises(PhpDeserializeError):
            loads(b"b:1")
        with pytest.raises(PhpDeserializeError):
            loads(b"b:")

    def test_truncated_int(self):
        """Truncated integer should raise error."""
        from php_deserialize import PhpDeserializeError, loads

        with pytest.raises(PhpDeserializeError):
            loads(b"i:42")
        with pytest.raises(PhpDeserializeError):
            loads(b"i:")

    def test_truncated_string(self):
        """Truncated string should raise error."""
        from php_deserialize import PhpDeserializeError, loads

        with pytest.raises(PhpDeserializeError):
            loads(b's:5:"hel')
        with pytest.raises(PhpDeserializeError):
            loads(b's:5:"hello"')  # Missing semicolon
        with pytest.raises(PhpDeserializeError):
            loads(b's:5:"hello')  # Missing closing quote

    def test_truncated_array(self):
        """Truncated array should raise error."""
        from php_deserialize import PhpDeserializeError, loads

        with pytest.raises(PhpDeserializeError):
            loads(b"a:1:{")
        with pytest.raises(PhpDeserializeError):
            loads(b"a:1:{i:0;")
        with pytest.raises(PhpDeserializeError):
            loads(b'a:1:{i:0;s:3:"foo"')

    def test_invalid_type_marker(self):
        """Unknown type marker should raise error."""
        from php_deserialize import PhpDeserializeError, loads

        with pytest.raises(PhpDeserializeError):
            loads(b"X:42;")
        with pytest.raises(PhpDeserializeError):
            loads(b"z:1;")

    def test_string_length_mismatch(self):
        """String with wrong length - auto-fallback recovers by default."""
        from php_deserialize import PhpDeserializeError, loads

        # Default behavior (strict=False): auto-fallback recovers from length mismatch
        result = loads(b's:10:"hello";')  # Declared 10, actual 5
        assert result == "hello"  # Successfully recovered

        # With strict=True: should raise error
        with pytest.raises(PhpDeserializeError):
            loads(b's:10:"hello";', strict=True)

    def test_negative_string_length(self):
        """Negative string length should raise error."""
        from php_deserialize import PhpDeserializeError, loads

        with pytest.raises(PhpDeserializeError):
            loads(b's:-1:"";')

    def test_array_count_mismatch(self):
        """Array with wrong count should raise error or handle gracefully."""
        from php_deserialize import PhpDeserializeError, loads

        # Declared 2 elements, only 1 provided
        with pytest.raises(PhpDeserializeError):
            loads(b'a:2:{i:0;s:3:"foo";}')


class TestBoundaryValues:
    """Tests for boundary/extreme values."""

    def test_max_int64(self):
        """Maximum i64 value."""
        from php_deserialize import loads

        max_i64 = 2**63 - 1
        data = f"i:{max_i64};".encode()
        assert loads(data) == max_i64

    def test_min_int64(self):
        """Minimum i64 value."""
        from php_deserialize import loads

        min_i64 = -(2**63)
        data = f"i:{min_i64};".encode()
        assert loads(data) == min_i64

    def test_very_small_float(self):
        """Very small float value."""
        from php_deserialize import loads

        result = loads(b"d:1e-308;")
        assert result == 1e-308

    def test_very_large_float(self):
        """Very large float value."""
        from php_deserialize import loads

        result = loads(b"d:1e308;")
        assert result == 1e308

    def test_float_zero_variants(self):
        """Different representations of zero."""
        from php_deserialize import loads

        assert loads(b"d:0;") == 0.0
        assert loads(b"d:0.0;") == 0.0
        assert loads(b"d:-0;") == 0.0
        assert loads(b"d:0e0;") == 0.0

    def test_empty_string(self):
        """Empty string."""
        from php_deserialize import loads

        assert loads(b's:0:"";') == ""

    def test_empty_array(self):
        """Empty array."""
        from php_deserialize import loads

        assert loads(b"a:0:{}") == []


class TestSpecialFloatValues:
    """Tests for special float values."""

    def test_positive_infinity(self):
        """Positive infinity."""
        from php_deserialize import loads

        result = loads(b"d:INF;")
        assert math.isinf(result) and result > 0

    def test_negative_infinity(self):
        """Negative infinity."""
        from php_deserialize import loads

        result = loads(b"d:-INF;")
        assert math.isinf(result) and result < 0

    def test_nan(self):
        """NaN value."""
        from php_deserialize import loads

        result = loads(b"d:NAN;")
        assert math.isnan(result)

    def test_scientific_notation(self):
        """Scientific notation variants."""
        from php_deserialize import loads

        assert loads(b"d:1e10;") == 1e10
        assert loads(b"d:1E10;") == 1e10
        assert loads(b"d:1.5e-5;") == 1.5e-5
        assert loads(b"d:-2.5E+3;") == -2.5e3


class TestNullByteHandling:
    """Tests for null byte handling in strings."""

    def test_null_byte_in_string(self):
        """String containing null bytes."""
        from php_deserialize import loads

        # s:5:"a\x00b\x00c";
        data = b's:5:"a\x00b\x00c";'
        result = loads(data, errors="bytes")
        assert b"\x00" in result if isinstance(result, bytes) else "\x00" in result

    def test_all_null_bytes(self):
        """String of only null bytes."""
        from php_deserialize import loads

        data = b's:3:"\x00\x00\x00";'
        result = loads(data, errors="bytes")
        if isinstance(result, bytes):
            assert result == b"\x00\x00\x00"
        else:
            assert result == "\x00\x00\x00"


class TestWhitespaceHandling:
    """Tests for whitespace in various positions."""

    def test_whitespace_in_string(self):
        """Whitespace characters in strings."""
        from php_deserialize import loads

        # Tab, newline, carriage return
        data = b's:5:"\t\n\r  ";'
        result = loads(data)
        assert result == "\t\n\r  "

    def test_string_with_leading_trailing_spaces(self):
        """String with leading/trailing spaces."""
        from php_deserialize import loads

        data = b's:7:"  foo  ";'
        assert loads(data) == "  foo  "


class TestSpecialStringContent:
    """Tests for strings with special content."""

    def test_string_with_php_code(self):
        """String containing PHP code."""
        from php_deserialize import loads

        php_code = '<?php echo "hello"; ?>'
        data = f's:{len(php_code)}:"{php_code}";'.encode()
        assert loads(data) == php_code

    def test_string_with_html(self):
        """String containing HTML."""
        from php_deserialize import loads

        html = '<div class="test">Hello & World</div>'
        data = f's:{len(html)}:"{html}";'.encode()
        assert loads(data) == html

    def test_string_with_json(self):
        """String containing JSON."""
        from php_deserialize import loads

        json_str = '{"key": "value", "arr": [1, 2, 3]}'
        data = f's:{len(json_str)}:"{json_str}";'.encode()
        assert loads(data) == json_str

    def test_string_with_sql(self):
        """String containing SQL."""
        from php_deserialize import loads

        sql = "SELECT * FROM users WHERE name = 'test'"
        data = f's:{len(sql)}:"{sql}";'.encode()
        assert loads(data) == sql

    def test_string_with_regex(self):
        """String containing regex pattern."""
        from php_deserialize import loads

        regex = r'^[a-z]+\d{2,4}$'
        data = f's:{len(regex)}:"{regex}";'.encode()
        assert loads(data) == regex


class TestArrayKeyTypes:
    """Tests for various array key types."""

    def test_negative_integer_keys(self):
        """Array with negative integer keys."""
        from php_deserialize import loads

        data = b'a:2:{i:-1;s:3:"foo";i:-2;s:3:"bar";}'
        result = loads(data)
        assert result == {-1: "foo", -2: "bar"}

    def test_large_integer_keys(self):
        """Array with large integer keys."""
        from php_deserialize import loads

        data = b'a:2:{i:1000000;s:3:"foo";i:2000000;s:3:"bar";}'
        result = loads(data)
        assert result == {1000000: "foo", 2000000: "bar"}

    def test_sparse_integer_keys(self):
        """Array with sparse (non-sequential) integer keys."""
        from php_deserialize import loads

        data = b'a:3:{i:0;s:1:"a";i:10;s:1:"b";i:100;s:1:"c";}'
        result = loads(data)
        assert result == {0: "a", 10: "b", 100: "c"}

    def test_mixed_positive_negative_keys(self):
        """Array with both positive and negative integer keys."""
        from php_deserialize import loads

        data = b'a:3:{i:-1;s:1:"a";i:0;s:1:"b";i:1;s:1:"c";}'
        result = loads(data)
        assert result == {-1: "a", 0: "b", 1: "c"}


class TestObjectEdgeCases:
    """Tests for object edge cases."""

    def test_empty_class_name(self):
        """Object with empty class name."""
        from php_deserialize import loads

        # O:0:"":0:{}
        data = b'O:0:"":0:{}'
        result = loads(data)
        assert result["__class__"] == ""

    def test_class_name_with_namespace(self):
        """Object with namespaced class name."""
        from php_deserialize import loads

        # App\Models\User - 15 chars in Python (backslash is single char)
        class_name = "App\\Models\\User"
        data = f'O:{len(class_name)}:"{class_name}":1:{{s:4:"name";s:5:"Alice";}}'.encode()
        result = loads(data)
        assert result["__class__"] == class_name
        assert result["name"] == "Alice"

    def test_deeply_namespaced_class(self):
        """Object with deeply namespaced class."""
        from php_deserialize import loads

        class_name = "Vendor\\Package\\Sub\\Module\\ClassName"
        data = f'O:{len(class_name)}:"{class_name}":0:{{}}'.encode()
        result = loads(data)
        assert result["__class__"] == class_name


class TestEnumEdgeCases:
    """Tests for PHP 8.1+ enum edge cases."""

    def test_enum_with_namespace(self):
        """Enum with namespaced class."""
        from php_deserialize import loads

        # E:len:"ClassName:CaseName";
        enum_value = "App\\Enums\\Status:Active"  # 23 chars
        data = f'E:{len(enum_value)}:"{enum_value}";'.encode()
        result = loads(data)
        assert result == "App\\Enums\\Status::Active"

    def test_enum_with_numbers(self):
        """Enum case with numbers."""
        from php_deserialize import loads

        data = b'E:12:"Status:Code1";'
        result = loads(data)
        assert result == "Status::Code1"


class TestReferenceEdgeCases:
    """Tests for reference (R/r) edge cases."""

    def test_simple_reference(self):
        """Simple reference marker."""
        from php_deserialize import loads

        # R:1; references first value
        data = b"R:1;"
        result = loads(data)
        assert result == {"__ref__": 1}

    def test_reference_in_array(self):
        """Reference within array."""
        from php_deserialize import loads

        # Array with reference to itself or another element
        data = b"a:2:{i:0;s:3:\"foo\";i:1;R:2;}"
        result = loads(data)
        # Second element references something
        assert result[1] == {"__ref__": 2}


class TestPreprocessEdgeCases:
    """Tests for preprocess function edge cases."""

    def test_preprocess_already_clean(self):
        """Preprocess on already clean data."""
        from php_deserialize import preprocess

        clean = b's:5:"hello";'
        result = preprocess(clean)
        assert result == clean

    def test_preprocess_double_escaped(self):
        """Preprocess double-escaped data."""
        from php_deserialize import preprocess

        escaped = b'"s:5:""hello"";\"'
        result = preprocess(escaped)
        assert result == b's:5:"hello";'

    def test_preprocess_no_outer_quotes(self):
        """Preprocess without outer quotes returns as-is."""
        from php_deserialize import preprocess

        data = b's:5:"hello";'
        result = preprocess(data)
        assert result == data


class TestIsSerialized:
    """Tests for is_serialized function."""

    def test_all_valid_type_markers(self):
        """All valid type markers should be recognized."""
        from php_deserialize import is_serialized

        assert is_serialized(b"N;") is True  # Null
        assert is_serialized(b"b:1;") is True  # Bool
        assert is_serialized(b"i:42;") is True  # Int
        assert is_serialized(b"d:3.14;") is True  # Float
        assert is_serialized(b's:5:"test";') is True  # String
        assert is_serialized(b"a:0:{}") is True  # Array
        assert is_serialized(b'O:8:"stdClass":0:{}') is True  # Object
        assert is_serialized(b"R:1;") is True  # Reference
        assert is_serialized(b"r:1;") is True  # Reference (lowercase)
        assert is_serialized(b'E:10:"Status:OK";') is True  # Enum

    def test_invalid_markers(self):
        """Invalid type markers should be rejected."""
        from php_deserialize import is_serialized

        assert is_serialized(b"X:1;") is False
        assert is_serialized(b"hello") is False
        assert is_serialized(b"123") is False
        assert is_serialized(b"") is False


class TestLargeData:
    """Tests for large data handling."""

    def test_large_string_100kb(self):
        """Large string (100KB)."""
        from php_deserialize import loads

        content = "x" * 100_000
        data = f's:{len(content)}:"{content}";'.encode()
        result = loads(data)
        assert result == content
        assert len(result) == 100_000

    def test_large_array_1000_elements(self):
        """Large array (1000 elements)."""
        from php_deserialize import loads

        count = 1000
        parts = [f"i:{i};i:{i};" for i in range(count)]
        data = f"a:{count}:{{{''.join(parts)}}}".encode()

        result = loads(data)
        assert len(result) == count
        assert result[0] == 0
        assert result[999] == 999

    def test_nested_50_levels(self):
        """Deeply nested structure (50 levels)."""
        from php_deserialize import loads

        depth = 50
        data = b's:4:"leaf";'
        for _ in range(depth):
            data = b"a:1:{i:0;" + data + b"}"

        result = loads(data)

        current = result
        for _ in range(depth):
            current = current[0]
        assert current == "leaf"


class TestJsonOutputEdgeCases:
    """Tests for JSON output edge cases."""

    def test_json_special_chars_escaped(self):
        """Special characters in JSON output should be escaped."""
        import json

        from php_deserialize import loads_json

        # String with characters that need JSON escaping
        special = 'test\n\t\r"\\/'
        data = f's:{len(special)}:"{special}";'.encode()
        json_str = loads_json(data)

        # Should be valid JSON
        parsed = json.loads(json_str)
        assert parsed == special

    def test_json_unicode_escaped(self):
        """Unicode in JSON output."""
        import json

        from php_deserialize import loads_json

        korean = "한글"
        data = f's:{len(korean.encode("utf-8"))}:"{korean}";'.encode("utf-8")
        json_str = loads_json(data)

        parsed = json.loads(json_str)
        assert parsed == korean
