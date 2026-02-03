# PHP Serialize Parser - Project Instructions

이 프로젝트는 고성능 PHP serialize 파서를 Rust로 구현하고 Python 바인딩을 제공하는 오픈소스 라이브러리입니다.

## Project Overview

```
phpserialize-rs/
├── crates/
│   ├── php-deserialize-core/     # Rust 코어 라이브러리
│   └── php-deserialize-python/   # Python 바인딩 (PyO3)
├── python/php_deserialize/       # Python 패키지
├── tests/
│   ├── python/                   # Python 테스트
│   └── fixtures/                 # PHP 생성 테스트 데이터
└── benches/                      # 벤치마크
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Core Parser | Rust 1.70+ |
| Python Binding | PyO3 0.22 |
| Build System | Cargo + maturin |
| Testing | cargo test, pytest |
| CI/CD | GitHub Actions |

## Quick Commands

| Command | Description |
|---------|-------------|
| `/build` | Rust 빌드 (release) |
| `/test` | 전체 테스트 실행 |
| `/bench` | 벤치마크 실행 |
| `/python-dev` | Python 개발 모드 빌드 |

## Development Setup

```bash
# Rust 빌드
cargo build --all-features

# Python 개발 모드
pip install maturin
maturin develop

# 테스트
cargo test --all-features
pytest tests/python -v
```

## Architecture

### Parser Design

**Zero-copy 파싱**: `Cow<'a, [u8]>` 사용으로 불필요한 메모리 복사 최소화

```rust
pub enum PhpValue<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Cow<'a, [u8]>),      // Zero-copy
    Array(Vec<(PhpValue<'a>, PhpValue<'a>)>),
    Object { class_name: Cow<'a, str>, properties: Vec<PhpProperty<'a>> },
    Enum { class_name: Cow<'a, str>, case_name: Cow<'a, str> },
    Reference(usize),
}
```

### DB Escape Auto-detection

DB에서 export된 데이터의 이중 따옴표 자동 감지 및 처리:

```
Input:  "a:1:{s:3:""key"";s:5:""value"";}"
Output: a:1:{s:3:"key";s:5:"value";}
```

## Supported PHP Types

| Code | Type | Format | Example |
|------|------|--------|---------|
| `N` | Null | `N;` | `N;` |
| `b` | Boolean | `b:<0\|1>;` | `b:1;` |
| `i` | Integer | `i:<value>;` | `i:42;` |
| `d` | Double | `d:<value>;` | `d:3.14;` |
| `s` | String | `s:<len>:"<data>";` | `s:5:"hello";` |
| `a` | Array | `a:<count>:{...}` | `a:1:{i:0;s:3:"foo";}` |
| `O` | Object | `O:<len>:"<class>":<n>:{...}` | 클래스 객체 |
| `C` | Custom | Serializable 인터페이스 | 커스텀 직렬화 |
| `R/r` | Reference | `R:<index>;` | 순환 참조 |
| `E` | Enum | `E:<len>:"<Class:Case>";` | PHP 8.1+ |

## Code Style

### Rust

- `cargo fmt` 적용
- `cargo clippy --all-features -- -D warnings` 통과 필수
- 공개 API에 문서 주석 필수

### Python

- Type hints 필수
- PEP 8 준수

## Testing Guidelines

### 테스트 추가 시 규칙

1. **문자열 길이 정확히 계산**: PHP serialize는 바이트 길이 사용
   ```rust
   // "한글" = 6 bytes (UTF-8)
   b"s:6:\"\xed\x95\x9c\xea\xb8\x80\";"
   ```

2. **Private/Protected 속성**: Null 바이트 포함
   ```rust
   // Private: \0ClassName\0propName
   // Protected: \0*\0propName
   ```

3. **에러 케이스도 테스트**
   ```rust
   #[test]
   fn test_error_invalid_type() {
       assert!(from_bytes(b"X:1;").is_err());
   }
   ```

## Release Checklist

- [ ] 모든 테스트 통과: `cargo test --all-features`
- [ ] Clippy 경고 없음
- [ ] Python 테스트 통과: `pytest tests/python`
- [ ] 버전 업데이트: `Cargo.toml`, `pyproject.toml`
- [ ] CHANGELOG 업데이트
- [ ] Git tag 생성: `git tag v0.x.x`

## Common Issues

### PyO3 API 변경

PyO3 0.22+에서는 `_bound` 접미사 메서드 사용:
```rust
// Old: PyDict::new(py)
// New: PyDict::new_bound(py)
```

### UTF-8 vs 바이트 길이

PHP serialize는 **바이트 길이**를 사용:
```
"한글" = 6 bytes (not 2 characters)
```
