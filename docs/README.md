# docs

Project documentation. Concise and accurate; future features are clearly labeled.

- `../HANDOFF.md` (repo root) — **READ FIRST**: live progress tracker, phase checklist, frozen decisions
- `SPEC.md` — master build specification (source of truth for the build agent)
- `ARCHITECTURE.md` — overall architecture and data flow
- `BUILD.md` — crate settings, dependency rationale, `[[bin]] name = "aurum"`
- `CONFIGURATION.md` — config.toml, env vars, profiles, config commands
- `PROVIDERS.md` — Massive adapter, resilience, fixtures, licensing
- `OPERATIONS.md` — run/deploy/upgrade/troubleshoot
- `BACKUPS.md` — backup / verify / restore
- `RELEASE.md` — versioning, cargo-dist, release checklist
- `RESILIENCE.md` — fault isolation & graceful degradation
- `QA.md` — Definition-of-Done, QA and release checklists
- `./CHANGELOG.md` (repo root) — version history

The root `README.md` is the visitor-facing product README. The master build specification lives in `docs/SPEC.md`.