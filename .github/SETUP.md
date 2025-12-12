# GitHub Actions Setup Guide

This guide helps you set up and configure GitHub Actions for mihomo-rs.

## Quick Start

All workflows are ready to use! Just push your code to GitHub and they will run automatically.

## Workflows Overview

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| **CI** | Push/PR | Build, test, lint | ✅ Ready |
| **Release** | Tag push | Build & publish releases | ⚠️ Needs secrets |
| **Coverage** | Push/PR | Code coverage | ⚠️ Needs token |
| **Docs** | Push/PR | Build documentation | ✅ Ready |
| **Benchmark** | Push/PR | Performance tests | ✅ Ready |
| **Dependabot** | Weekly | Dependency updates | ✅ Ready |

## Required Setup

### 1. Enable GitHub Actions (Automatic)

GitHub Actions are enabled by default for public repositories. No action needed.

### 2. Add Secrets for Release Workflow

To enable automatic releases to crates.io:

1. Get your crates.io API token:
   - Visit https://crates.io/settings/tokens
   - Click "New Token"
   - Name it "GitHub Actions"
   - Copy the token

2. Add to GitHub:
   - Go to repository Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `CARGO_TOKEN`
   - Value: Paste your token
   - Click "Add secret"

### 3. Add Codecov Token (Optional)

For code coverage reports:

1. Sign up at https://codecov.io with your GitHub account
2. Add your repository
3. Copy the upload token
4. Add to GitHub secrets:
   - Name: `CODECOV_TOKEN`
   - Value: Your codecov token

### 4. Enable GitHub Pages (Optional)

For automatic documentation deployment:

1. Go to Settings → Pages
2. Source: Deploy from a branch
3. Branch: `gh-pages` / `/ (root)`
4. Click Save

Documentation will be available at: `https://DINGDANGMAOUP.github.io/mihomo-rs/`

## Testing Workflows Locally

### Test CI Checks

```bash
# Run all checks that CI runs
cargo test --all-features
cargo fmt --check
cargo clippy -- -D warnings
cargo build --release
cargo build --examples
```

### Test Release Build

```bash
# Build for current platform
cargo build --release

# Check binary
./target/release/mihomo-rs --version
```

## Creating Your First Release

### 1. Update Version

Edit `Cargo.toml`:
```toml
[package]
version = "1.0.0"  # Update this
```

### 2. Update Changelog

Create or update `CHANGELOG.md`:
```markdown
## [1.0.0] - 2024-01-01

### Added
- Initial release
- Version management
- Configuration management
- Service management
- Proxy management
```

### 3. Commit Changes

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 1.0.0"
git push
```

### 4. Create and Push Tag

```bash
git tag v1.0.0
git push origin v1.0.0
```

### 5. Monitor Release

1. Go to Actions tab
2. Watch "Release" workflow
3. Check Releases page for artifacts

## Workflow Details

### CI Workflow

**Runs on:**
- Every push to main/master/develop
- Every pull request

**What it does:**
- Tests on Ubuntu, macOS, Windows
- Tests with stable and beta Rust
- Checks code formatting
- Runs clippy linter
- Builds release binaries
- Runs security audit

**Caching:**
- Cargo registry
- Cargo index
- Build artifacts

### Release Workflow

**Runs on:**
- Tag push (v*)

**What it does:**
- Creates GitHub release
- Builds for multiple platforms:
  - Linux (x86_64, ARM64)
  - macOS (x86_64, ARM64)
  - Windows (x86_64)
- Uploads compressed binaries
- Publishes to crates.io

**Artifacts:**
- `mihomo-rs-linux-amd64.tar.gz`
- `mihomo-rs-linux-arm64.tar.gz`
- `mihomo-rs-darwin-amd64.tar.gz`
- `mihomo-rs-darwin-arm64.tar.gz`
- `mihomo-rs-windows-amd64.exe.zip`

### Coverage Workflow

**Runs on:**
- Push to main/master
- Pull requests

**What it does:**
- Generates code coverage with tarpaulin
- Uploads to codecov.io
- Comments on PRs with coverage changes

### Docs Workflow

**Runs on:**
- Push to main/master
- Pull requests

**What it does:**
- Builds Rust documentation
- Deploys to GitHub Pages (on main)

### Benchmark Workflow

**Runs on:**
- Push to main/master
- Pull requests

**What it does:**
- Runs performance benchmarks
- Stores results for comparison

### Dependabot

**Runs:**
- Weekly (Monday)

**What it does:**
- Checks for dependency updates
- Creates PRs for updates
- Separate PRs for Cargo and GitHub Actions

## Troubleshooting

### CI Fails on Format Check

```bash
# Fix formatting
cargo fmt

# Commit
git add .
git commit -m "style: fix formatting"
```

### CI Fails on Clippy

```bash
# See warnings
cargo clippy

# Auto-fix
cargo clippy --fix

# Commit
git add .
git commit -m "fix: address clippy warnings"
```

### Release Fails to Publish

**Problem:** `CARGO_TOKEN` not set or invalid

**Solution:**
1. Verify token in repository secrets
2. Ensure token has publish permissions
3. Check crates.io account status

### Coverage Upload Fails

**Problem:** `CODECOV_TOKEN` not set

**Solution:**
1. Sign up at codecov.io
2. Add repository
3. Copy token to GitHub secrets

### Documentation Not Deploying

**Problem:** GitHub Pages not enabled

**Solution:**
1. Enable GitHub Pages in settings
2. Set source to `gh-pages` branch
3. Wait a few minutes for deployment

## Best Practices

### Before Pushing

```bash
# Run local checks
cargo test
cargo fmt
cargo clippy
```

### Before Creating Release

1. Update version in Cargo.toml
2. Update CHANGELOG.md
3. Test locally
4. Create tag with `v` prefix

### Monitoring

- Check Actions tab regularly
- Review Dependabot PRs weekly
- Monitor security advisories

## Advanced Configuration

### Add More Test Platforms

Edit `.github/workflows/ci.yml`:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    rust: [stable, beta, nightly]  # Add nightly
```

### Add More Release Targets

Edit `.github/workflows/release.yml`:

```yaml
- os: ubuntu-latest
  target: x86_64-unknown-linux-musl
  artifact_name: mihomo-rs
  asset_name: mihomo-rs-linux-musl-amd64
```

### Change Dependabot Schedule

Edit `.github/dependabot.yml`:

```yaml
schedule:
  interval: "daily"  # Change from weekly
```

## Support

- **Issues**: https://github.com/DINGDANGMAOUP/mihomo-rs/issues
- **Discussions**: https://github.com/DINGDANGMAOUP/mihomo-rs/discussions
- **Workflows**: `.github/workflows/README.md`

## Resources

- [GitHub Actions Docs](https://docs.github.com/en/actions)
- [Rust CI/CD Guide](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Dependabot Docs](https://docs.github.com/en/code-security/dependabot)
- [Codecov Docs](https://docs.codecov.com/)
