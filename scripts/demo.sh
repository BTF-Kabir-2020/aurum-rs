#!/usr/bin/env bash
# scripts/demo.sh — one-command showcase for Linux/macOS (README §30, OPERATIONS).
# Usage: ./scripts/demo.sh [speed 1..10]
set -euo pipefail

# cargo lives under ~/.cargo/bin on rustup installs (HANDOFF §4a).
export PATH="$HOME/.cargo/bin:$PATH"

SPEED="${1:-10}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXE="$ROOT/target/release/aurum"

if [ ! -x "$EXE" ]; then
  echo "building release binary (first run only)..."
  (cd "$ROOT" && cargo build --release --quiet)
fi

cd "$ROOT"
"$EXE" version
"$EXE" demo --speed "$SPEED"
echo
"$EXE" doctor
echo
echo "Try next: \"$EXE\" server   # API at http://127.0.0.1:8080/api/v1/*"
