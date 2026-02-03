# Contributing to phpserialize-rs

Thank you for your interest in contributing! This document provides guidelines and information for contributors.

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Python 3.8 or later
- maturin (`pip install maturin`)

### Building

```bash
# Build Rust library
cargo build

# Build Python package (development mode)
maturin develop

# Build Python package (release mode)
maturin build --release
```

### Testing

```bash
# Run Rust tests
cargo test --all-features

# Run Python tests
maturin develop
pytest tests/python -v
```

### Benchmarks

```bash
# Run Rust benchmarks
cargo bench --all-features
```

## Code Style

### Rust

- Follow standard Rust formatting (`cargo fmt`)
- Pass clippy with no warnings (`cargo clippy --all-features -- -D warnings`)
- Add documentation for public items

### Python

- Use type hints
- Follow PEP 8
- Use ruff for linting

## Pull Request Process

1. Fork the repository and create a branch from `main`
2. Make your changes
3. Add tests for new functionality
4. Ensure all tests pass
5. Update documentation if needed
6. Submit a pull request

## Commit Messages

Use clear, descriptive commit messages:

```
feat: add support for PHP 8.1 enums
fix: handle empty strings correctly
docs: update API documentation
perf: optimize array parsing
test: add edge case tests for Korean strings
```

## Reporting Issues

When reporting issues, please include:

- PHP serialize data that causes the issue (if applicable)
- Expected behavior
- Actual behavior
- Rust/Python version
- Operating system

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).
