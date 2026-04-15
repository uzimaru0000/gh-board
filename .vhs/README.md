# Demo recording

The README image (`assets/demo.gif`) is generated from `demo.tape` with [vhs](https://github.com/charmbracelet/vhs).

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

2. From the repository root, run:

   ```
   vhs .vhs/demo.tape
   ```

   (vhs itself spawns bash internally via `Set Shell "bash"` in the tape, so the recorded commands work regardless of your login shell.)

3. Verify `assets/demo.gif` looks right, then commit it.

If the file grows too large for GitHub to render, adjust `Set Width` / `Set Height` or `Set PlaybackSpeed` in `demo.tape` and regenerate.

## Tearing down the demo project

The setup script never deletes anything. When you are done recording, remove the project from the GitHub web UI (or `gh project delete <number> --owner @me`).
