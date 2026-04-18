# gh-board demo
#
# Before recording:
#   1. Make sure `gh auth login` has been run (scope: project, read:project).
#   2. Bootstrap the demo project:  ./.vhs/setup.sh
#      The script prints the project number. Set it via DEMO_PROJECT below
#      (or export GH_BOARD_DEMO_PROJECT=<number> in your shell).
#   3. From the repository root, run:  vhs .vhs/demo.tape
#
# Output is written to assets/demo.gif and embedded from README.md / README.ja.md.

Output assets/demo.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "nord"
Set TypingSpeed 60ms

# Force English UI (help overlay etc.) regardless of host locale
Env LANG "en_US.UTF-8"
Env LC_ALL "en_US.UTF-8"

Type `gh board --owner @me 2`
Sleep 500ms
Enter
Sleep 3s

# Navigate cards and columns
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "l"
Sleep 300ms
Type "j"
Sleep 200ms
Type "k"
Sleep 200ms
Type "h"
Sleep 500ms

# Open card detail, scroll, and close
Enter
Sleep 2s
Type "j"
Sleep 150ms
Type "j"
Sleep 150ms
Type "j"
Sleep 150ms
Type "k"
Sleep 800ms
Escape
Sleep 1s

# Filter by free text
Type "/"
Sleep 400ms
Type "CI"
Sleep 700ms
Enter
Sleep 1500ms
Ctrl+u
Sleep 1s

# Grab mode: pick a card, move it right, drop
Space
Sleep 600ms
Type "l"
Sleep 400ms
Space
Sleep 1s

# Bulk archive the Done column: jump there, select all, archive in one shot
Tab
Sleep 400ms
Type "v"
Sleep 800ms
Type "*"
Sleep 1s
Type "a"
Sleep 1200ms
Enter
Sleep 1500ms

# Help overlay
Type "?"
Sleep 2500ms
Escape
Sleep 500ms

# Quit
Type "q"
Sleep 400ms
