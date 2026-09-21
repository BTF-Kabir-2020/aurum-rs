# Contributing

Thanks for contributing to Aurum.

## Ground rules

- Rust first. No Python/Node sidecars in the core app.
- No fake features. Everything claimed must run.
- No AI/ML wording for rule-based calculations.
- Small, focused changes. Preserve existing behavior unless the change is intentional.

## Setup

```
rustup install 1.98.1
cargo run -- demo
```

The toolchain is pinned in `rust-toolchain.toml`.

## Quality gates before any pull request

Run these locally; CI enforces the same:

```
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

or once:

```
make gates          # all four gates
make smoke          # gates + doctor + backup create/verify
```

Smoke tests for changes touching runtime behavior:
`aurum demo`, `aurum doctor`, `aurum server` + `/health/live` + `/api/v1/signal/XAUUSD`, `aurum backup create` + `verify`.

See `docs/QA.md` for the full matrix.

## Workflow

1. Branch from `main`: `feat/...`, `fix/...`, `docs/...`, `chore/...`
2. Reference the issue (if any) in the PR description.
3. Run the quality gates above.
4. `HANDOFF.md` is the progress tracker — if your change starts or completes a build phase, update its rows.
5. Open a PR against `main`. CI runs fmt/check/clippy/test on Linux/Windows/macOS, dependency security, and a Docker smoke test.
6. Don't commit `.env`, `data/`, `backups/`, `target/`, or secrets.

## Definition of done

A PR is done only when all of the following hold:

- formatted and clippy-clean
- tests pass
- demo works offline
- docs updated where behavior/usage changed
- no secrets committed

## Questions

Open a normal issue for questions and feature discussions; use private reporting per `SECURITY.md` for vulnerabilities.