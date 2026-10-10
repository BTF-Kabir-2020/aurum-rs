# assets

Visual assets used by the top-level [`README.md`](../README.md).

## `demo.svg`

A faithful terminal rendering of the real `aurum watch XAUUSD` screen
(same content the TUI draws, per `src/tui/ui.rs`). It is a hand-authored SVG
so it always renders on GitHub (no broken image) and stays tiny in the repo.

Regenerate / replace with a real capture:

```powershell
# after: cargo build --release
.\target\release\aurum.exe demo --speed 10     # deterministic offline replay
.\target\release\aurum.exe watch XAUUSD        # the TUI screen (screenshot this)
```

- **Screenshot:** capture the `aurum watch` window (Windows Terminal: `Win+Shift+S`)
  and save as `assets/demo.png`, then reference it in the README.
- **GIF / asciinema:** record `aurum demo --speed 10` with
  [`asciinema`](https://asciinema.org) (`asciinema rec assets/demo.cast`) or
  [asciinema → GIF](../../) tooling.

Keep the demo screen honest: it reflects offline fixture data only.
