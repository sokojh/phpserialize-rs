"""Property-based tests using Hypothesis.

These tests generate random inputs to find edge cases that manual tests might miss.
Run with: pytest tests/python/test_property.py -m hypothesis
"""

import pytest
from hypothesis import given, strategies as st, assume, settings, HealthCheck

# Mark all tests in this module as hypothesis tests
pytestmark = pytest.mark.hypothesis


class TestNeverCrashes:
    """Parser should never crash on any input - only raise PhpDeserializeError."""

    @given(st.binary(max_size=10000))
    @settings(max_examples=500, suppress_health_check=[HealthCheck.too_slow])
    def test_random_bytes_never_crash(self, data: bytes):
        """Any random bytes should either parse or raise PhpDeserializeError."""
        from php_deserialize import loads, PhpDeserializeError

        try:
            loads(data)
        except PhpDeserializeError:
            pass  # Expected for invalid input
        except Exception as e:
            pytest.fail(f"Unexpected exception type: {type(e).__name__}: {e}")

    @given(st.text(max_size=10000))
    @settings(max_examples=500, suppress_health_check=[HealthCheck.too_slow])
    def test_random_text_never_crash(self, text: str):
        """Any random text (encoded as UTF-8) should never crash."""
        from php_deserialize import loads, PhpDeserializeError

        try:
            loads(text.encode("utf-8"))
        except PhpDeserializeError:
            pass
        except Exception as e:
            pytest.fail(f"Unexpected exception type: {type(e).__name__}: {e}")


class TestIntegerRoundtrip:
    """Integer values should survive serialize->deserialize."""

    @given(st.integers(min_value=-(2**63), max_value=2**63 - 1))
    @settings(max_examples=1000)
    def test_integer_roundtrip(self, n: int):
        """Any i64 integer should parse correctly."""
        from php_deserialize import loads

        data = f"i:{n};".encode()
        result = loads(data)
        assert result == n, f"Expected {n}, got {result}"

    @given(st.integers(min_value=0, max_value=2**63 - 1))
    @settings(max_examples=500)
    def test_positive_integers(self, n: int):
        """Positive integers should parse correctly."""
        from php_deserialize import loads

        data = f"i:{n};".encode()
        assert loads(data) == n

    @given(st.integers(min_value=-(2**63), max_value=-1))
    @settings(max_examples=500)
    def test_negative_integers(self, n: int):
        """Negative integers should parse correctly."""
        from php_deserialize import loads

        data = f"i:{n};".encode()
        assert loads(data) == n


class TestFloatRoundtrip:
    """Float values should survive serialize->deserialize."""

    @given(st.floats(allow_nan=False, allow_infinity=False))
    @settings(max_examples=500)
    def test_float_roundtrip(self, f: float):
        """Regular floats should parse correctly."""
        from php_deserialize import loads

        data = f"d:{f};".encode()
        result = loads(data)
        assert abs(result - f) < 1e-10 or result == f

    @given(st.floats(allow_nan=False, allow_infinity=True))
    @settings(max_examples=200)
    def test_float_with_infinity(self, f: float):
        """Infinity values should be handled."""
        from php_deserialize import loads
        import math

        if math.isinf(f):
            if f > 0:
                data = b"d:INF;"
            else:
                data = b"d:-INF;"
            result = loads(data)
            assert math.isinf(result)
            assert (result > 0) == (f > 0)


class TestStringRoundtrip:
    """String values should survive serialize->deserialize."""

    @given(st.text(max_size=1000))
    @settings(max_examples=500, suppress_health_check=[HealthCheck.too_slow])
    def test_utf8_string_roundtrip(self, s: str):
        """Any valid UTF-8 string should parse correctly."""
        from php_deserialize import loads

        encoded = s.encode("utf-8")
        data = f's:{len(encoded)}:"{s}";'.encode("utf-8")
        result = loads(data)
        assert result == s, f"Expected {s!r}, got {result!r}"

    @given(st.binary(max_size=1000))
    @settings(max_examples=500, suppress_health_check=[HealthCheck.too_slow])
    def test_binary_string_with_bytes_mode(self, b: bytes):
        """Binary data should be preserved in bytes mode."""
        from php_deserialize import loads

        data = f's:{len(b)}:"'.encode() + b + b'";'
        result = loads(data, errors="bytes")
        # Result might be str if valid UTF-8, or bytes if not
        if isinstance(result, str):
            assert result.encode("utf-8") == b or result == b.decode("utf-8", errors="replace")
        else:
            assert result == b

    @given(st.text(alphabet=st.characters(blacklist_categories=("Cs",)), max_size=500))
    @settings(max_examples=300)
    def test_unicode_strings(self, s: str):
        """Unicode strings (excluding surrogates) should parse correctly."""
        from php_deserialize import loads

        encoded = s.encode("utf-8")
        data = f's:{len(encoded)}:"{s}";'.encode("utf-8")
        result = loads(data)
        assert result == s


class TestBooleanRoundtrip:
    """Boolean values should survive serialize->deserialize."""

    @given(st.booleans())
    @settings(max_examples=100)
    def test_boolean_roundtrip(self, b: bool):
        """Booleans should parse correctly."""
        from php_deserialize import loads

        data = f"b:{1 if b else 0};".encode()
        result = loads(data)
        assert result is b


class TestArrayRoundtrip:
    """Array structures should survive serialize->deserialize."""

    @given(st.lists(st.integers(min_value=-1000, max_value=1000), max_size=50))
    @settings(max_examples=300, suppress_health_check=[HealthCheck.too_slow])
    def test_integer_list_roundtrip(self, items: list):
        """Lists of integers should parse correctly."""
        from php_deserialize import loads

        # Build PHP array format
        parts = [f"i:{i};i:{v};" for i, v in enumerate(items)]
        data = f"a:{len(items)}:{{{''.join(parts)}}}".encode()

        result = loads(data)
        assert result == items

    @given(st.lists(st.text(max_size=50), max_size=20))
    @settings(max_examples=200, suppress_health_check=[HealthCheck.too_slow])
    def test_string_list_roundtrip(self, items: list):
        """Lists of strings should parse correctly."""
        from php_deserialize import loads

        # Build PHP array format
        parts = []
        for i, s in enumerate(items):
            encoded = s.encode("utf-8")
            parts.append(f'i:{i};s:{len(encoded)}:"{s}";')
        data = f"a:{len(items)}:{{{''.join(parts)}}}".encode("utf-8")

        result = loads(data)
        assert result == items

    @given(
        st.dictionaries(
            keys=st.text(min_size=1, max_size=20, alphabet=st.characters(whitelist_categories=("L", "N"))),
            values=st.integers(min_value=-1000, max_value=1000),
            min_size=1,  # Avoid empty dict (becomes empty list in PHP)
            max_size=20,
        )
    )
    @settings(max_examples=200, suppress_health_check=[HealthCheck.too_slow])
    def test_dict_roundtrip(self, d: dict):
        """Dicts with string keys and integer values should parse correctly."""
        from php_deserialize import loads

        # Build PHP array format
        parts = []
        for k, v in d.items():
            encoded_key = k.encode("utf-8")
            parts.append(f's:{len(encoded_key)}:"{k}";i:{v};')
        data = f"a:{len(d)}:{{{''.join(parts)}}}".encode("utf-8")

        result = loads(data)
        assert result == d


class TestNestedStructures:
    """Nested structures should be handled correctly."""

    @given(st.integers(min_value=1, max_value=30))
    @settings(max_examples=50)
    def test_nested_depth(self, depth: int):
        """Deeply nested arrays should parse correctly."""
        from php_deserialize import loads

        # Build nested structure
        data = b's:4:"leaf";'
        for _ in range(depth):
            data = b"a:1:{i:0;" + data + b"}"

        result = loads(data)

        # Navigate to leaf
        current = result
        for _ in range(depth):
            assert isinstance(current, list), f"Expected list, got {type(current)}"
            current = current[0]
        assert current == "leaf"

    @given(st.integers(min_value=1, max_value=100))
    @settings(max_examples=50)
    def test_wide_arrays(self, width: int):
        """Wide arrays should parse correctly."""
        from php_deserialize import loads

        parts = [f"i:{i};i:{i * 2};" for i in range(width)]
        data = f"a:{width}:{{{''.join(parts)}}}".encode()

        result = loads(data)
        assert len(result) == width
        assert result[0] == 0
        assert result[width - 1] == (width - 1) * 2


class TestEdgeCaseStrings:
    """Edge cases for string parsing."""

    @given(st.integers(min_value=0, max_value=100))
    @settings(max_examples=100)
    def test_repeated_quotes(self, n: int):
        """Strings with many quotes should parse correctly."""
        from php_deserialize import loads

        s = '"' * n
        data = f's:{n}:"{s}";'.encode()
        result = loads(data)
        assert result == s

    @given(st.integers(min_value=0, max_value=100))
    @settings(max_examples=100)
    def test_repeated_semicolons(self, n: int):
        """Strings with many semicolons should parse correctly."""
        from php_deserialize import loads

        s = ";" * n
        data = f's:{n}:"{s}";'.encode()
        result = loads(data)
        assert result == s

    @given(st.integers(min_value=0, max_value=100))
    @settings(max_examples=100)
    def test_repeated_braces(self, n: int):
        """Strings with many braces should parse correctly."""
        from php_deserialize import loads

        s = "{}" * n
        length = n * 2
        data = f's:{length}:"{s}";'.encode()
        result = loads(data)
        assert result == s


class TestDbEscaping:
    """DB escape handling edge cases."""

    @given(st.text(min_size=1, max_size=100, alphabet=st.characters(whitelist_categories=("L", "N"))))
    @settings(max_examples=200)
    def test_db_escaped_string(self, s: str):
        """DB-escaped strings should be unescaped correctly."""
        from php_deserialize import loads

        encoded = s.encode("utf-8")
        # Create DB-escaped format: "s:len:""value"";"
        escaped = f'"s:{len(encoded)}:""{s}"";\"'.encode("utf-8")

        result = loads(escaped)
        assert result == s

    @given(
        st.dictionaries(
            keys=st.text(min_size=1, max_size=10, alphabet=st.characters(whitelist_categories=("L",))),
            values=st.text(min_size=1, max_size=10, alphabet=st.characters(whitelist_categories=("L",))),
            min_size=1,
            max_size=5,
        )
    )
    @settings(max_examples=100, suppress_health_check=[HealthCheck.too_slow])
    def test_db_escaped_dict(self, d: dict):
        """DB-escaped dicts should be unescaped correctly."""
        from php_deserialize import loads

        # Build normal PHP array
        parts = []
        for k, v in d.items():
            ek = k.encode("utf-8")
            ev = v.encode("utf-8")
            parts.append(f's:{len(ek)}:"{k}";s:{len(ev)}:"{v}";')
        normal = f"a:{len(d)}:{{{''.join(parts)}}}"

        # Escape for DB
        escaped_inner = normal.replace('"', '""')
        escaped = f'"{escaped_inner}"'.encode("utf-8")

        result = loads(escaped)
        assert result == d


class TestMixedTypes:
    """Mixed type arrays."""

    @given(
        st.lists(
            st.one_of(
                st.integers(min_value=-1000, max_value=1000),
                st.text(max_size=20),
                st.booleans(),
                st.none(),
            ),
            max_size=20,
        )
    )
    @settings(max_examples=200, suppress_health_check=[HealthCheck.too_slow])
    def test_mixed_type_array(self, items: list):
        """Arrays with mixed types should parse correctly."""
        from php_deserialize import loads

        parts = []
        for i, v in enumerate(items):
            if v is None:
                parts.append(f"i:{i};N;")
            elif isinstance(v, bool):
                parts.append(f"i:{i};b:{1 if v else 0};")
            elif isinstance(v, int):
                parts.append(f"i:{i};i:{v};")
            elif isinstance(v, str):
                encoded = v.encode("utf-8")
                parts.append(f'i:{i};s:{len(encoded)}:"{v}";')

        data = f"a:{len(items)}:{{{''.join(parts)}}}".encode("utf-8")
        result = loads(data)
        assert result == items


class TestKoreanAndMultibyte:
    """Korean and multibyte character handling."""

    @given(st.text(alphabet="가나다라마바사아자차카타파하한글테스트", max_size=100))
    @settings(max_examples=200)
    def test_korean_strings(self, s: str):
        """Korean strings should parse correctly with byte length."""
        from php_deserialize import loads

        encoded = s.encode("utf-8")
        data = f's:{len(encoded)}:"{s}";'.encode("utf-8")
        result = loads(data)
        assert result == s

    @given(st.text(alphabet="あいうえおかきくけこ日本語", max_size=100))
    @settings(max_examples=200)
    def test_japanese_strings(self, s: str):
        """Japanese strings should parse correctly."""
        from php_deserialize import loads

        encoded = s.encode("utf-8")
        data = f's:{len(encoded)}:"{s}";'.encode("utf-8")
        result = loads(data)
        assert result == s

    @given(st.text(alphabet="中文字符测试", max_size=100))
    @settings(max_examples=200)
    def test_chinese_strings(self, s: str):
        """Chinese strings should parse correctly."""
        from php_deserialize import loads

        encoded = s.encode("utf-8")
        data = f's:{len(encoded)}:"{s}";'.encode("utf-8")
        result = loads(data)
        assert result == s

    @given(st.text(alphabet="🎉🎊🎁🎄🎅🤖👻💀", max_size=50))
    @settings(max_examples=200)
    def test_emoji_strings(self, s: str):
        """Emoji strings (4-byte UTF-8) should parse correctly."""
        from php_deserialize import loads

        encoded = s.encode("utf-8")
        data = f's:{len(encoded)}:"{s}";'.encode("utf-8")
        result = loads(data)
        assert result == s


class TestJsonOutput:
    """JSON output mode tests."""

    @given(st.dictionaries(
        st.text(min_size=1, max_size=10),
        st.integers(min_value=-(2**63), max_value=2**63 - 1),  # i64 range
        min_size=1,  # Avoid empty dict
        max_size=10
    ))
    @settings(max_examples=100, suppress_health_check=[HealthCheck.too_slow])
    def test_json_output_dict(self, d: dict):
        """JSON output should be valid JSON."""
        import json

        from php_deserialize import loads_json

        # Build PHP array
        parts = []
        for k, v in d.items():
            ek = k.encode("utf-8")
            parts.append(f's:{len(ek)}:"{k}";i:{v};')
        data = f"a:{len(d)}:{{{''.join(parts)}}}".encode("utf-8")

        json_str = loads_json(data)
        parsed = json.loads(json_str)
        assert parsed == d

    @given(st.lists(st.integers(min_value=-(2**63), max_value=2**63 - 1), max_size=20))
    @settings(max_examples=100)
    def test_json_output_list(self, items: list):
        """JSON output for lists should be valid."""
        import json

        from php_deserialize import loads_json

        parts = [f"i:{i};i:{v};" for i, v in enumerate(items)]
        data = f"a:{len(items)}:{{{''.join(parts)}}}".encode()

        json_str = loads_json(data)
        parsed = json.loads(json_str)
        assert parsed == items
