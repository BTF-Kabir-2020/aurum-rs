# workflows

- `ci.yml` — fmt / check / clippy / test on Linux, Windows, macOS + Docker smoke test
- `release.yml` — release binaries for Windows/Linux/macOS targets via tag `v*`

Complementary GitHub config lives in the parent `.github/` directory: `dependabot.yml` (dependency updates), `CODEOWNERS` (ownership).