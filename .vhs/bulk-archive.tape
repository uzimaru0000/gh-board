# gh-board — Bulk archive of the Done column (≈20s)
#
# Renders assets/demo-bulk-archive.gif.
# Walks across to the Done column, enters bulk-select mode, selects every
# card in the column with `*`, and archives them in one action.
#
# Prerequisites: same as demo.tape. The demo project must have at least
# one card in the "Done" column (setup.sh seeds two by default).

Output assets/demo-bulk-archive.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "nord"
Set TypingSpeed 60ms

Env LANG "en_US.UTF-8"
Env LC_ALL "en_US.UTF-8"

# 1. Launch gh-board
Type `gh board --owner @me 2`
Enter
Sleep 2500ms

# 2. Move to the Done column (Tab cycles columns; two presses lands on Done
#    in a Todo / In Progress / Done layout)
Tab Sleep 400ms
Tab Sleep 700ms

# 3. Enter bulk-select mode
Type "v"
Sleep 1000ms

# 4. Select every card in this column
Type "*"
Sleep 1200ms

# 5. Bulk archive — confirmation dialog appears
Type "a"
Sleep 1500ms

# 6. Confirm
Enter
Sleep 2000ms

# 7. Quit
Type "q"
Sleep 500ms
