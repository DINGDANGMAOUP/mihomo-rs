# Contributing to mihomo-rs

This project follows an agile, incremental workflow: small PRs, quick feedback, and verifiable steps.

## Principles

- Keep changes small and focused.
- Prefer iterative delivery over large one-shot refactors.
- Ensure each step is buildable and testable.
- Update docs/examples together with behavior changes.

## Development Setup

```bash
git clone https://github.com/DINGDANGMAOUP/mihomo-rs.git
cd mihomo-rs
cargo build
cargo test
```

## Agile Contribution Flow

1. Define one sprint-sized goal (single concern).
2. Split it into 1-3 incremental commits.
3. Validate after each increment (`fmt`, `clippy`, targeted tests).
4. Open PR early if design/API direction needs feedback.

Recommended branch names:

- `feat/<topic>`
- `fix/<topic>`
- `refactor/<topic>`
- `docs/<topic>`
- `test/<topic>`

## Definition of Done (Per PR)

A PR is considered done when all are true:

- Code compiles.
- Tests for changed behavior exist or are updated.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo test` passes.
- User-facing changes are reflected in `README.md` and `README_CN.md` if relevant.

## Commit Style

Use Conventional Commits:

- `feat: ...`
- `fix: ...`
- `refactor: ...`
- `test: ...`
- `docs: ...`
- `chore: ...`

Examples:

- `feat: add close-by-process command for connection management`
- `fix: validate profile names in config subcommands`
- `docs: refresh progressive examples section`

## PR Expectations

Please include:

- What changed.
- Why it changed.
- How to verify.
- Any compatibility or migration notes.

Keep PRs reviewable:

- Prefer < 400 lines when possible.
- Separate refactor and behavior changes into different commits.
- Avoid mixing unrelated features.

## Testing Guidance

Use the smallest meaningful test scope first, then full regression:

```bash
# unit/integration quick loop
cargo test <name_fragment>

# full project checks
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

If adding API behavior, also add or update an example in `examples/` when practical.

## Documentation Policy

When changing command behavior, public APIs, or workflow:

- Update `README.md` (English).
- Update `README_CN.md` (Chinese).
- Keep examples aligned with the actual CLI/API behavior.

## Security

Do not disclose vulnerabilities in public issues. Follow [SECURITY.md](./SECURITY.md).

## Questions

Open an issue or draft PR for early discussion when uncertain about direction.
