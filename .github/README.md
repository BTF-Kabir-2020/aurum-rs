# .github

GitHub repository configuration.

- `workflows/ci.yml` — full CI: fmt, check, clippy, test (Linux/Windows/macOS), dependency security (cargo-audit + cargo-deny), Docker smoke test
- `workflows/release.yml` — tagged releases via `cargo build --release` for the target matrix
- `dependabot.yml` — automated dependency updates (cargo + GitHub Actions)
- `CODEOWNERS` — repository ownership for reviews
- `ISSUE_TEMPLATE/` — bug report & feature request forms
- `PULL_REQUEST_TEMPLATE.md` — PR checklist bound to the QA gates

CI never depends on a personal API key; provider tests use recorded fixtures.
See `docs/QA.md` for the enforced quality gates.