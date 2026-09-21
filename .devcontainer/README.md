# Dev Container

A VS Code environment ready to code in, no manual setup:

- Rust `1.98.1` stable (matches `rust-toolchain.toml`)
- rust-analyzer + TOML extensions
- format on save, full-feature check on save
- `cargo fetch` + `cargo check` run automatically on container creation

## Open

In VS Code with the Dev Containers extension: `Dev Containers: Reopen in Container` (or `: Rebuild Container`).

## Windows/Docker Desktop

On this machine, Windows drives mount read-write under `/drives` (`C:\` -> `/drives/c`). For native Windows workflows (VS Code on Windows, PowerShell, dotnet, npm, .exe), run outside the container — see AGENTS.md and the `win-host` skill. Inside the container, builds run on its Linux filesystem.

## What it is for

Pure-Rust development, the quality gates, and Docker (`docker compose up -d`) — all inside the container.