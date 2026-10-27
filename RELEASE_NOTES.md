# v2.2.1

## Compatibility

This patch preserves the 2.x public API. `MihomoError::Yaml` continues to contain
`serde_yaml::Error`, with `From<serde_yaml::Error>`, `.into()` and `?` propagation
supported. The unreleased 3.0.0 YAML migration is reverted; no 3.0.0 release/tag was
published. CLI commands and configuration formats remain unchanged.

## Security fixes

- Update rustls-webpki to 0.103.16 and webpki-roots to 1.0.10, plus compatible
  rustls, h2, quinn-proto and ctutils security fixes.
- Update ZIP to 9.1.2 and zstd to 0.14.3, retaining all default compression features.
- Update Tokio to 1.53.3, anyhow to 1.0.103 and AES to non-yanked 0.9.3.
- Retain archive and YAML/config regression tests, including Windows CRLF fixtures;
  add an explicit regression for the original public YAML error conversions.

## Known dependency policy warning

To preserve the original public error type, this patch retains `serde_yaml` and
its mandatory `unsafe-libyaml 0.2.11` dependency. RUSTSEC-2024-0449 reports that
`unsafe-libyaml` is **unmaintained**, with no patched version available.
`cargo audit` reports zero vulnerabilities but retains this maintenance warning;
`cargo deny` fails on that one advisory. This known tradeoff is explicitly accepted
for the compatibility patch; no advisory ignores or weakened policy checks are added.
A maintained YAML parser migration remains a future breaking API change.

Real HTTPS core-download smoke in the cloud environment failed with UnknownIssuer
on both baseline and patched TLS dependencies. No certificate checks were bypassed;
this release does not claim successful live download validation in that environment.
