## Description

_What you did and why. Link the issue if there is one._

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] Chore (CI, deps, tooling)
- [ ] Other

## Quality gates (must pass before review)

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --all-targets --all-features`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-targets --all-features`

## Smoke tests (if behavior changed)

- [ ] `aurum demo` works offline
- [ ] `aurum doctor` diagnostics sensible
- [ ] API handles `:symbol` variants (XAUUSD / XAU/USD / C:XAUUSD)
- [ ] backup create/verify/restore still valid

## Checklist

- [ ] No secrets or real API keys committed
- [ ] No fake claims (no AI/ML wording for rule-based logic)
- [ ] Docs updated where behavior/usage changed
- [ ] `HANDOFF.md` progress rows updated if this PR completes/finishes a phase