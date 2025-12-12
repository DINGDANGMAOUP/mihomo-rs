# GitHub Actions Workflows

This directory contains GitHub Actions workflows for CI/CD automation.

## Workflows

### 1. CI (`ci.yml`)

**Triggers:** Push and PR to main/master/develop branches

**Jobs:**
- **Test**: Run tests on multiple OS (Ubuntu, macOS, Windows) and Rust versions (stable, beta)
- **Format**: Check code formatting with `rustfmt`
- **Clippy**: Run linter checks with `clippy`
- **Build**: Build release binaries and examples
- **Security**: Run security audit with `cargo-audit`

**Features:**
- Caching for faster builds
- Cross-platform testing
- Comprehensive checks

### 2. Release (`release.yml`)

**Triggers:** Push tags matching `v*` (e.g., `v1.0.0`)

**Jobs:**
- **Create Release**: Create GitHub release
- **Build Release**: Build binaries for multiple platforms:
  - Linux (x86_64, ARM64)
  - macOS (x86_64, ARM64)
  - Windows (x86_64)
- **Publish Crate**: Publish to crates.io

**Artifacts:**
- Compressed binaries for each platform
- Automatic release notes

**Usage:**
```bash
# Create and push a tag
git tag v1.0.0
git push origin v1.0.0
```

### 3. Coverage (`coverage.yml`)

**Triggers:** Push and PR to main/master branches

**Jobs:**
- Generate code coverage with `cargo-tarpaulin`
- Upload to codecov.io

**Setup Required:**
- Add `CODECOV_TOKEN` to repository secrets

### 4. Documentation (`docs.yml`)

**Triggers:** Push and PR to main/master branches

**Jobs:**
- Build Rust documentation
- Deploy to GitHub Pages (on push to main)

**Setup Required:**
- Enable GitHub Pages in repository settings
- Set source to `gh-pages` branch

### 5. Benchmark (`benchmark.yml`)

**Triggers:** Push and PR to main/master branches

**Jobs:**
- Run benchmarks with `cargo bench`
- Store and compare results

**Note:** Requires benchmark tests in the project

### 6. Dependabot (`../dependabot.yml`)

**Schedule:** Weekly

**Updates:**
- Cargo dependencies
- GitHub Actions versions

**Features:**
- Automatic PR creation
- Labeled and organized updates

## Required Secrets

Add these secrets in repository settings (Settings → Secrets and variables → Actions):

| Secret | Description | Required For |
|--------|-------------|--------------|
| `CARGO_TOKEN` | crates.io API token | Release workflow |
| `CODECOV_TOKEN` | Codecov upload token | Coverage workflow |
| `GITHUB_TOKEN` | Automatically provided | All workflows |

## Setup Instructions

### 1. Enable GitHub Actions

GitHub Actions are enabled by default for public repositories.

### 2. Add Required Secrets

```bash
# Get crates.io token
# Visit: https://crates.io/settings/tokens

# Get Codecov token
# Visit: https://codecov.io/gh/YOUR_USERNAME/mihomo-rs
```

### 3. Enable GitHub Pages (Optional)

1. Go to Settings → Pages
2. Set source to `gh-pages` branch
3. Documentation will be available at: `https://YOUR_USERNAME.github.io/mihomo-rs/`

### 4. Create First Release

```bash
# Ensure version in Cargo.toml is correct
git tag v1.0.0
git push origin v1.0.0
```

## Workflow Status Badges

Add these badges to your README.md:

```markdown
[![CI](https://github.com/YOUR_USERNAME/mihomo-rs/workflows/CI/badge.svg)](https://github.com/YOUR_USERNAME/mihomo-rs/actions/workflows/ci.yml)
[![Release](https://github.com/YOUR_USERNAME/mihomo-rs/workflows/Release/badge.svg)](https://github.com/YOUR_USERNAME/mihomo-rs/actions/workflows/release.yml)
[![Coverage](https://codecov.io/gh/YOUR_USERNAME/mihomo-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_USERNAME/mihomo-rs)
[![Documentation](https://github.com/YOUR_USERNAME/mihomo-rs/workflows/Documentation/badge.svg)](https://YOUR_USERNAME.github.io/mihomo-rs/)
```

## Customization

### Modify Test Matrix

Edit `ci.yml` to add/remove OS or Rust versions:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    rust: [stable, beta, nightly]  # Add nightly
```

### Add More Release Targets

Edit `release.yml` to add more platforms:

```yaml
- os: ubuntu-latest
  target: x86_64-unknown-linux-musl
  artifact_name: mihomo-rs
  asset_name: mihomo-rs-linux-musl-amd64
```

### Adjust Dependabot Schedule

Edit `dependabot.yml`:

```yaml
schedule:
  interval: "daily"  # Change from weekly to daily
```

## Troubleshooting

### CI Fails on Windows

- Check for path separator issues (`/` vs `\`)
- Ensure commands are cross-platform compatible

### Release Build Fails

- Verify all targets are installed
- Check cross-compilation dependencies

### Coverage Upload Fails

- Ensure `CODECOV_TOKEN` is set correctly
- Check codecov.io project settings

### Documentation Not Deploying

- Verify GitHub Pages is enabled
- Check branch name matches workflow configuration

## Best Practices

1. **Always test locally before pushing**
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

2. **Use semantic versioning for releases**
   - Major: Breaking changes (v2.0.0)
   - Minor: New features (v1.1.0)
   - Patch: Bug fixes (v1.0.1)

3. **Keep workflows updated**
   - Dependabot will help with this
   - Review and merge dependency updates regularly

4. **Monitor workflow runs**
   - Check Actions tab regularly
   - Fix failures promptly

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Dependabot Documentation](https://docs.github.com/en/code-security/dependabot)
