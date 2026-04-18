# Demo recording

GIFs under `assets/` are generated from the `*.tape` files in this directory with [vhs](https://github.com/charmbracelet/vhs).

| Tape | Output | Purpose |
|------|--------|---------|
| `demo.tape` | `assets/demo.gif` | Headline demo embedded in the README |
| `views.tape` | `assets/demo-views.gif` | Showcases `~/.config/gh-board/config.toml` views and `1` / `2` switching |
| `bulk-archive.tape` | `assets/demo-bulk-archive.gif` | Bulk-select the Done column and archive in one action |

## Prerequisites

- `vhs` installed (`brew install vhs` on macOS)
- A Nerd Font installed locally — the tape requests `FiraCode Nerd Font Mono` so sub-issue / grab / status icons render correctly
- `gh` CLI authenticated with `project` scope: `gh auth refresh -s project`
- `jq` available on `PATH`

## Recording workflow

1. Bootstrap (or reuse) the throwaway demo project and export its number.

   bash / zsh:

   ```
   export GH_BOARD_DEMO_OWNER=@me
   export GH_BOARD_DEMO_PROJECT=$(./.vhs/setup.sh)
   ```

   fish:

   ```
   set -x GH_BOARD_DEMO_OWNER @me
   set -x GH_BOARD_DEMO_PROJECT (./.vhs/setup.sh)
   ```

   `setup.sh` is idempotent — it reuses a project titled `gh-board demo` if it already exists, and seeds draft issues across `Todo` / `In Progress` / `Done` the first time.

2. From the repository root, render the tape you want:

   ```
   vhs .vhs/demo.tape           # main demo
   vhs .vhs/views.tape          # config + view switching
   vhs .vhs/bulk-archive.tape   # bulk archive of the Done column
   ```

   (vhs itself spawns bash internally via `Set Shell "bash"` in the tape, so the recorded commands work regardless of your login shell.)

3. Verify the generated GIF under `assets/` looks right, then commit it.

> **Note**: `views.tape` and `bulk-archive.tape` hardcode `gh board --owner @me 2`. Adjust the project number if your demo project lives elsewhere. The bulk-archive tape mutates the project (cards in `Done` get archived), so re-run `setup.sh` to reseed before re-recording.

If the file grows too large for GitHub to render, adjust `Set Width` / `Set Height` or `Set PlaybackSpeed` in `demo.tape` and regenerate.

## Tearing down the demo project

The setup script never deletes anything. When you are done recording, remove the project from the GitHub web UI (or `gh project delete <number> --owner @me`).
