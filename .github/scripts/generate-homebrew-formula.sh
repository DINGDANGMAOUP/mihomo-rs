#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 3 ]]; then
  echo "usage: $0 <owner/repo> <tag> <formula-path>" >&2
  exit 1
fi

repo="$1"
tag="$2"
formula_path="$3"
version="${tag#v}"
base_url="https://github.com/${repo}/releases/download/${tag}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

fetch_sha() {
  local asset="$1"
  curl -fsSL "${base_url}/${asset}.sha256" | awk '{print $1}'
}

darwin_amd64_sha="$(fetch_sha "mihomo-rs-darwin-amd64.tar.gz")"
darwin_arm64_sha="$(fetch_sha "mihomo-rs-darwin-arm64.tar.gz")"
linux_amd64_sha="$(fetch_sha "mihomo-rs-linux-amd64.tar.gz")"
linux_arm64_sha="$(fetch_sha "mihomo-rs-linux-arm64.tar.gz")"

mkdir -p "$(dirname "${formula_path}")"

cat > "${formula_path}" <<EOF
class MihomoRs < Formula
  desc "Rust SDK and CLI tool for mihomo proxy management"
  homepage "https://github.com/${repo}"
  version "${version}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "${base_url}/mihomo-rs-darwin-arm64.tar.gz"
      sha256 "${darwin_arm64_sha}"
    else
      url "${base_url}/mihomo-rs-darwin-amd64.tar.gz"
      sha256 "${darwin_amd64_sha}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "${base_url}/mihomo-rs-linux-arm64.tar.gz"
      sha256 "${linux_arm64_sha}"
    else
      url "${base_url}/mihomo-rs-linux-amd64.tar.gz"
      sha256 "${linux_amd64_sha}"
    end
  end

  livecheck do
    url :stable
    regex(/^v?(\\d+(?:\\.\\d+)+)$/i)
  end

  def install
    bin.install "mihomo-rs"
  end

  test do
    assert_match "mihomo-rs", shell_output("#{bin}/mihomo-rs --help")
  end
end
EOF

echo "Generated ${formula_path}"
