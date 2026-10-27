# v3.0.0

## Breaking API migration

This major release replaces deprecated `serde_yaml` / unmaintained `unsafe-libyaml`
with maintained `serde_norway 0.9.42` / `unsafe-libyaml-norway 0.2.15`.
`MihomoError::Yaml` now contains `serde_norway::Error`; conversions from the old
`serde_yaml::Error`, including `?` propagation, no longer compile. Update the
parser dependency/imports as described in [English](README.md#migrating-to-30)
or [中文](README_CN.md#迁移到-30). CLI commands and configuration formats are unchanged.

## Security and compatibility

- Update rustls-webpki to 0.103.16 and webpki-roots to 1.0.10, plus compatible
  rustls, h2, quinn-proto and ctutils security fixes.
- Update ZIP to 9.1.2 and zstd to 0.14.3 with all default compression features retained.
- Update Tokio to 1.53.3, anyhow to 1.0.103 and AES to the non-yanked 0.9.3 patch.
- Preserve YAML aliases, literal merge keys, tags, complex keys and semantic roundtrips.
  Add config/profile and archive regressions, including Windows CRLF fixtures.
- Audit and dependency policy checks pass without advisory suppression.

Real HTTPS core-download smoke in the cloud environment failed with UnknownIssuer
on both baseline and patched TLS dependencies. No certificate checks were bypassed;
this release does not claim a successful end-to-end live download from that environment.
