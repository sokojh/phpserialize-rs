"""Real-world test cases based on actual production data patterns."""

import pytest


class TestImwebFormData:
    """Tests mimicking imweb site_form.item patterns."""

    def test_form_fields_korean(self):
        """Form fields with Korean labels."""
        from php_deserialize import loads

        # Simulated form field structure
        # "이름" = 6 bytes, "type" = 4, "text" = 4, "label" = 5, "required" = 8
        data = (
            b'a:3:{s:4:"type";s:4:"text";s:5:"label";s:6:"\xec\x9d\xb4\xeb\xa6\x84";'
            b's:8:"required";b:1;}'
        )
        result = loads(data)
        assert result["type"] == "text"
        assert result["label"] == "이름"  # Korean: "name"
        assert result["required"] is True

    def test_nested_form_structure(self):
        """Nested form with multiple fields."""
        from php_deserialize import loads

        # Simpler nested structure test
        data = (
            b'a:1:{s:6:"fields";a:2:{i:0;a:1:{s:4:"type";s:4:"text";}'
            b'i:1;a:1:{s:4:"type";s:5:"email";}}}'
        )
        result = loads(data)
        assert len(result["fields"]) == 2
        assert result["fields"][0]["type"] == "text"
        assert result["fields"][1]["type"] == "email"


class TestImwebPermissions:
    """Tests mimicking imweb permission arrays."""

    def test_permission_array(self):
        """Permission level array."""
        from php_deserialize import loads

        data = b'a:3:{i:0;s:4:"read";i:1;s:5:"write";i:2;s:6:"delete";}'
        result = loads(data)
        assert result == ["read", "write", "delete"]

    def test_role_permissions(self):
        """Role-based permissions."""
        from php_deserialize import loads

        data = (
            b'a:2:{s:5:"admin";a:3:{i:0;s:4:"read";i:1;s:5:"write";i:2;s:6:"delete";}'
            b's:4:"user";a:1:{i:0;s:4:"read";}}'
        )
        result = loads(data)
        assert result["admin"] == ["read", "write", "delete"]
        assert result["user"] == ["read"]


class TestImwebWidgetData:
    """Tests mimicking imweb widget data patterns."""

    def test_widget_html_content(self):
        """Widget with HTML content."""
        from php_deserialize import loads

        # Simulated widget data with HTML
        # '<p>Hello</p>' = 12 bytes
        data = b'a:2:{s:8:"textHtml";s:12:"<p>Hello</p>";s:7:"visible";b:1;}'
        result = loads(data)
        assert result["textHtml"] == "<p>Hello</p>"
        assert result["visible"] is True

    def test_widget_with_special_chars(self):
        """Widget data with special characters."""
        from php_deserialize import loads

        # "Test Co" = 7 bytes, "phone" = 5, "company" = 7
        # "02-1234-5678" = 12 bytes
        data = (
            b'a:2:{s:7:"company";s:7:"Test Co";'
            b's:5:"phone";s:12:"02-1234-5678";}'
        )
        result = loads(data)
        assert result["company"] == "Test Co"
        assert result["phone"] == "02-1234-5678"


class TestDbEscapedRealWorld:
    """Tests for real-world DB-escaped data."""

    def test_escaped_widget_data(self):
        """Simulated DB-exported widget data."""
        from php_deserialize import loads

        # Simple escaped test
        escaped = b'"a:1:{s:4:""name"";s:5:""Alice"";}"'
        result = loads(escaped)
        assert result["name"] == "Alice"

    def test_deeply_nested_escaped(self):
        """Deeply nested structure with escaping."""
        from php_deserialize import loads

        # Single level escaping
        escaped = b'"a:1:{s:4:""user"";a:1:{s:4:""name"";s:5:""Alice"";}}"'
        result = loads(escaped)
        assert result["user"]["name"] == "Alice"


class TestLargeData:
    """Tests for larger data structures."""

    def test_large_array(self):
        """Array with many elements."""
        from php_deserialize import loads

        # Build a large SEQUENTIAL array (0, 1, 2, ..., 99)
        # Sequential arrays become lists in Python
        count = 100
        parts = [f'i:{i};i:{i * 2};' for i in range(count)]
        data = f'a:{count}:{{{"".join(parts)}}}'.encode()

        result = loads(data)
        # Sequential 0..99 keys become a list
        assert isinstance(result, list)
        assert len(result) == 100
        assert result[0] == 0
        assert result[50] == 100
        assert result[99] == 198

    def test_nested_depth(self):
        """Deeply nested structure."""
        from php_deserialize import loads

        # Create nested structure using integer keys for simplicity
        depth = 50
        data = b's:4:"leaf";'
        for i in range(depth):
            data = b'a:1:{i:0;' + data + b'}'

        result = loads(data)
        # Navigate to leaf
        current = result
        for _ in range(depth):
            current = current[0]
        assert current == "leaf"


class TestEdgeCases:
    """Edge case tests."""

    def test_empty_string_key(self):
        """Array with empty string key."""
        from php_deserialize import loads

        data = b'a:1:{s:0:"";s:5:"value";}'
        result = loads(data)
        assert result[""] == "value"

    def test_numeric_string_key(self):
        """Array with numeric string key."""
        from php_deserialize import loads

        data = b'a:1:{s:1:"0";s:5:"value";}'
        result = loads(data)
        assert result["0"] == "value"

    def test_mixed_int_string_keys(self):
        """Array with both integer and string keys."""
        from php_deserialize import loads

        data = b'a:2:{i:0;s:3:"foo";s:3:"bar";s:3:"baz";}'
        result = loads(data)
        assert result[0] == "foo"
        assert result["bar"] == "baz"

    def test_boolean_values_in_array(self):
        """Array with boolean values."""
        from php_deserialize import loads

        data = b'a:2:{s:6:"active";b:1;s:7:"deleted";b:0;}'
        result = loads(data)
        assert result["active"] is True
        assert result["deleted"] is False

    def test_null_in_array(self):
        """Array containing null values."""
        from php_deserialize import loads

        data = b'a:2:{s:4:"name";s:5:"Alice";s:5:"email";N;}'
        result = loads(data)
        assert result["name"] == "Alice"
        assert result["email"] is None


class TestJsonOutput:
    """Tests specifically for JSON output mode."""

    def test_json_indexed_array(self):
        """JSON output for indexed array."""
        from php_deserialize import loads_json
        import json

        data = b'a:3:{i:0;s:1:"a";i:1;s:1:"b";i:2;s:1:"c";}'
        result = json.loads(loads_json(data))
        assert result == ["a", "b", "c"]

    def test_json_nested(self):
        """JSON output for nested structure."""
        from php_deserialize import loads_json
        import json

        data = b'a:1:{s:4:"user";a:2:{s:4:"name";s:5:"Alice";s:3:"age";i:30;}}'
        result = json.loads(loads_json(data))
        assert result == {"user": {"name": "Alice", "age": 30}}

    def test_json_unicode(self):
        """JSON output with Unicode characters."""
        from php_deserialize import loads_json
        import json

        # "한글" = 6 bytes
        data = b'a:1:{s:4:"name";s:6:"\xed\x95\x9c\xea\xb8\x80";}'
        result = json.loads(loads_json(data))
        assert result == {"name": "한글"}
