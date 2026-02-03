<?php
/**
 * Generate test fixtures for PHP deserialize library.
 *
 * Run with: php generate_fixtures.php > fixtures.json
 */

$fixtures = [];

// Basic types
$fixtures['null'] = ['input' => serialize(null), 'expected' => null];
$fixtures['bool_true'] = ['input' => serialize(true), 'expected' => true];
$fixtures['bool_false'] = ['input' => serialize(false), 'expected' => false];
$fixtures['int_zero'] = ['input' => serialize(0), 'expected' => 0];
$fixtures['int_positive'] = ['input' => serialize(42), 'expected' => 42];
$fixtures['int_negative'] = ['input' => serialize(-123), 'expected' => -123];
$fixtures['int_large'] = ['input' => serialize(PHP_INT_MAX), 'expected' => PHP_INT_MAX];
$fixtures['float_zero'] = ['input' => serialize(0.0), 'expected' => 0.0];
$fixtures['float_positive'] = ['input' => serialize(3.14), 'expected' => 3.14];
$fixtures['float_negative'] = ['input' => serialize(-2.5), 'expected' => -2.5];
$fixtures['float_scientific'] = ['input' => serialize(1.5e10), 'expected' => 1.5e10];

// Strings
$fixtures['string_empty'] = ['input' => serialize(''), 'expected' => ''];
$fixtures['string_simple'] = ['input' => serialize('hello'), 'expected' => 'hello'];
$fixtures['string_spaces'] = ['input' => serialize('hello world'), 'expected' => 'hello world'];
$fixtures['string_korean'] = ['input' => serialize('한글'), 'expected' => '한글'];
$fixtures['string_mixed'] = ['input' => serialize('hello 한글 world'), 'expected' => 'hello 한글 world'];
$fixtures['string_special'] = ['input' => serialize('hello & world'), 'expected' => 'hello & world'];
$fixtures['string_quotes'] = ['input' => serialize('say "hello"'), 'expected' => 'say "hello"'];
$fixtures['string_newline'] = ['input' => serialize("line1\nline2"), 'expected' => "line1\nline2"];

// Arrays
$fixtures['array_empty'] = ['input' => serialize([]), 'expected' => []];
$fixtures['array_indexed'] = ['input' => serialize(['foo', 'bar', 'baz']), 'expected' => ['foo', 'bar', 'baz']];
$fixtures['array_associative'] = [
    'input' => serialize(['name' => 'Alice', 'age' => 30]),
    'expected' => ['name' => 'Alice', 'age' => 30]
];
$fixtures['array_mixed'] = [
    'input' => serialize([0 => 'zero', 'one' => 1, 2 => 'two']),
    'expected' => [0 => 'zero', 'one' => 1, 2 => 'two']
];
$fixtures['array_nested'] = [
    'input' => serialize(['user' => ['name' => 'Alice', 'tags' => ['admin', 'active']]]),
    'expected' => ['user' => ['name' => 'Alice', 'tags' => ['admin', 'active']]]
];

// Objects
class TestClass {
    public $publicProp = 'public';
    protected $protectedProp = 'protected';
    private $privateProp = 'private';
}

$obj = new TestClass();
$fixtures['object_simple'] = [
    'input' => serialize($obj),
    'expected_class' => 'TestClass',
    'expected_props' => ['publicProp' => 'public']
];

$fixtures['object_stdclass'] = [
    'input' => serialize((object)['name' => 'Alice', 'age' => 30]),
    'expected_class' => 'stdClass',
    'expected_props' => ['name' => 'Alice', 'age' => 30]
];

// Complex nested structures
$fixtures['complex_form'] = [
    'input' => serialize([
        'fields' => [
            ['type' => 'text', 'label' => '이름', 'required' => true],
            ['type' => 'email', 'label' => '이메일', 'required' => true],
            ['type' => 'textarea', 'label' => '내용', 'required' => false],
        ],
        'settings' => [
            'submit_text' => '제출하기',
            'success_message' => '감사합니다!',
        ]
    ]),
    'expected_field_count' => 3
];

$fixtures['complex_permissions'] = [
    'input' => serialize([
        'admin' => ['read', 'write', 'delete', 'manage'],
        'editor' => ['read', 'write'],
        'viewer' => ['read'],
    ]),
    'expected' => [
        'admin' => ['read', 'write', 'delete', 'manage'],
        'editor' => ['read', 'write'],
        'viewer' => ['read'],
    ]
];

// PHP 8.1+ Enums (only if PHP 8.1+)
if (version_compare(PHP_VERSION, '8.1.0', '>=')) {
    // Note: Enums require actual enum definition, skipping for compatibility
}

// Output as JSON
echo json_encode($fixtures, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE);
echo "\n";
