# Contributing to mihomo-rs

Thank you for your interest in contributing to mihomo-rs! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- A GitHub account

### Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/DINGDANGMAOUP/mihomo-rs.git
cd mihomo-rs

# Build the project
cargo build

# Run tests
cargo test

# Run examples
cargo run --example list_proxies
```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

### 2. Make Changes

- Write clear, concise code
- Follow Rust conventions and idioms
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with verbose output
cargo test -- --nocapture

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings

# Build examples
cargo build --examples
```

### 4. Commit Your Changes

Use conventional commit messages:

```bash
# Feature
git commit -m "feat: add proxy delay testing"

# Bug fix
git commit -m "fix: resolve connection timeout issue"

# Documentation
git commit -m "docs: update README with examples"

# Refactor
git commit -m "refactor: simplify config manager"

# Test
git commit -m "test: add integration tests for proxy switching"

# Chore
git commit -m "chore: update dependencies"
```

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

## Pull Request Guidelines

### PR Title

Use conventional commit format:
- `feat: description` - New feature
- `fix: description` - Bug fix
- `docs: description` - Documentation
- `refactor: description` - Code refactoring
- `test: description` - Tests
- `chore: description` - Maintenance

### PR Description

Include:
- **What**: What changes were made
- **Why**: Why these changes are needed
- **How**: How the changes work
- **Testing**: How you tested the changes

Example:
```markdown
## What
Add support for custom home directory via MIHOMO_HOME environment variable

## Why
Users need to run multiple isolated instances or use custom storage locations

## How
- Added `get_home_dir()` helper function
- Updated all managers to use the helper
- Added `with_home()` constructors for programmatic usage

## Testing
- Added examples/custom_home_sdk.rs
- Tested with multiple isolated instances
- All existing tests pass
```

### PR Checklist

- [ ] Code follows Rust conventions
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] Examples added (if applicable)
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `cargo test` passes
- [ ] Commit messages follow conventions

## Code Style

### Formatting

Use `rustfmt` with default settings:

```bash
cargo fmt
```

### Linting

Fix all clippy warnings:

```bash
cargo clippy --fix
```

### Naming Conventions

- **Functions**: `snake_case`
- **Types**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`

### Documentation

Add doc comments for public APIs:

```rust
/// Get the external controller URL from config
///
/// # Returns
///
/// Returns the URL as a String (e.g., "http://127.0.0.1:9090")
///
/// # Errors
///
/// Returns error if config file cannot be read or parsed
pub async fn get_external_controller(&self) -> Result<String> {
    // ...
}
```

## Testing

### Unit Tests

Add tests in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        // Test implementation
    }
}
```

### Integration Tests

Add tests in `tests/` directory:

```rust
// tests/integration_test.rs
use mihomo_rs::*;

#[tokio::test]
async fn test_version_manager() {
    // Test implementation
}
```

### Examples

Add examples in `examples/` directory:

```rust
// examples/my_example.rs
use mihomo_rs::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Example implementation
    Ok(())
}
```

## Documentation

### README Updates

Update README.md when:
- Adding new features
- Changing public APIs
- Adding new examples

### API Documentation

Update doc comments when:
- Adding new public functions
- Changing function signatures
- Modifying behavior

### Examples

Add examples when:
- Implementing new features
- Demonstrating complex usage
- Showing best practices

## Release Process

Releases are automated via GitHub Actions:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Commit changes
4. Create and push tag:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```
5. GitHub Actions will:
   - Build binaries for all platforms
   - Create GitHub release
   - Publish to crates.io

## Getting Help

- **Issues**: Open an issue on GitHub
- **Discussions**: Use GitHub Discussions
- **Questions**: Ask in issues with `question` label

## Recognition

Contributors will be:
- Listed in release notes
- Mentioned in CHANGELOG.md
- Credited in documentation

Thank you for contributing to mihomo-rs! 🎉
