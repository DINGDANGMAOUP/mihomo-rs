#!/usr/bin/env bash
set -euo pipefail

COVERAGE_MIN="${COVERAGE_MIN:-80}"

if ! command -v cargo-tarpaulin >/dev/null 2>&1; then
  echo "cargo-tarpaulin is not installed."
  echo "Install it with: cargo install cargo-tarpaulin"
  exit 1
fi

echo "Running coverage with minimum threshold: ${COVERAGE_MIN}%"
cargo tarpaulin \
  --verbose \
  --all-features \
  --workspace \
  --timeout 120 \
  --exclude-files "src/main.rs" \
  --exclude-files "src/cli/*" \
  --out xml \
  --fail-under "${COVERAGE_MIN}"
