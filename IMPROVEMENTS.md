# 코드 개선 권장사항

## 높은 우선순위

### 1. tracing 지원 추가

```toml
# Cargo.toml
[features]
default = []
serde = ["dep:serde", "dep:serde_json"]
tracing = ["dep:tracing"]

[dependencies]
tracing = { version = "0.1", optional = true }
```

```rust
// parser.rs
#[cfg(feature = "tracing")]
use tracing::{debug, instrument, trace, warn};

#[cfg_attr(feature = "tracing", instrument(skip(self), fields(pos = self.pos)))]
fn parse_value(&mut self) -> Result<PhpValue<'a>> {
    #[cfg(feature = "tracing")]
    trace!("parse_value at position {}", self.pos);

    // ... existing code ...
}
```

### 2. unsafe 코드 제거

```rust
// 현재 (위험)
unsafe { std::mem::transmute::<&[u8], &'a [u8]>(&owned) }

// 개선안: OwnedParser 타입 분리
pub enum ParseResult<'a> {
    Borrowed(PhpValue<'a>),
    Owned(PhpValue<'static>),
}

pub fn parse(&mut self) -> Result<ParseResult<'a>> {
    let data = if self.config.auto_unescape {
        preprocess(self.data)
    } else {
        Cow::Borrowed(self.data)
    };

    match data {
        Cow::Borrowed(_) => Ok(ParseResult::Borrowed(self.parse_value()?)),
        Cow::Owned(owned) => {
            let mut parser = OwnedParser::new(owned, self.config.clone());
            Ok(ParseResult::Owned(parser.parse()?))
        }
    }
}
```

### 3. 메모리 한도 추가

```rust
/// Parser configuration options.
#[derive(Debug, Clone)]
pub struct ParserConfig {
    pub max_depth: usize,
    pub auto_unescape: bool,
    pub strict_utf8: bool,
    /// Maximum total bytes to allocate (DoS protection)
    pub max_allocation: usize,  // NEW
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_depth: 512,
            auto_unescape: true,
            strict_utf8: false,
            max_allocation: 100 * 1024 * 1024,  // 100MB default
        }
    }
}
```

### 4. 에러 컨텍스트 개선

```rust
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub struct PhpDeserializeError {
    pub kind: ErrorKind,
    pub position: usize,
    pub context: Option<String>,
    /// Preview of input around error position
    pub input_preview: Option<String>,  // NEW
}

impl PhpDeserializeError {
    pub fn with_input_preview(mut self, data: &[u8]) -> Self {
        let start = self.position.saturating_sub(20);
        let end = (self.position + 20).min(data.len());
        if let Ok(preview) = std::str::from_utf8(&data[start..end]) {
            self.input_preview = Some(preview.to_string());
        }
        self
    }
}
```

## 중간 우선순위

### 5. Python 바인딩 로깅

```rust
// lib.rs (python bindings)
use pyo3::Python;

#[pyfunction]
fn loads(py: Python<'_>, data: &[u8], errors: &str, auto_unescape: bool) -> PyResult<PyObject> {
    // Log parsing attempt (if Python logging available)
    #[cfg(debug_assertions)]
    eprintln!("[php_deserialize] Parsing {} bytes", data.len());

    // ... existing code ...
}
```

### 6. 추가 테스트 케이스

```rust
// tests for parser state recovery
#[test]
fn test_parser_can_be_reused() {
    let data1 = b"i:42;";
    let data2 = b"s:5:\"hello\";";

    let mut parser = Parser::new(data1);
    let _ = parser.parse();

    // Parser should be reset for new data
    parser = Parser::new(data2);
    let result = parser.parse().unwrap();
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn test_memory_limit_respected() {
    let config = ParserConfig {
        max_allocation: 1024,  // 1KB limit
        ..Default::default()
    };

    // Try to parse very large array
    let huge = format!("a:100000:{{{}}}", "i:0;i:0;".repeat(100000));
    let result = from_bytes_with_config(huge.as_bytes(), config);
    assert!(matches!(result, Err(e) if matches!(e.kind, ErrorKind::AllocationLimitExceeded)));
}
```

## 낮은 우선순위

### 7. serde Deserialize 지원

```rust
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for PhpValue<'static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize from JSON/other formats
        unimplemented!("PhpValue can only be created from PHP serialize format")
    }
}
```

### 8. no_std 지원

```toml
[features]
default = ["std"]
std = []
alloc = []  # Use only alloc crate

[dependencies]
# Use alloc-compatible types
```

## 검증 체크리스트

- [ ] `cargo clippy --all-features -- -D warnings` 통과
- [ ] `cargo test --all-features` 통과
- [ ] `cargo doc --all-features` 경고 없음
- [ ] `maturin develop && pytest` 통과
- [ ] 벤치마크 성능 회귀 없음
