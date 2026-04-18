# gh-board — Config-driven views demo (≈25s)
#
# Renders assets/demo-views.gif.
# Shows ~/.config/gh-board/config.toml, then switches between the two
# configured views with `1` / `2`.
#
# Prerequisites: same as demo.tape. `~/.config/gh-board/config.toml` should
# define at least two [[view]] blocks. Example:
#
#   [[view]]
#   name = "Backlog"
#   layout = "table"
#   filter = "no:status"
#
#   [[view]]
#   name = "My Tasks"
#   filter = "assignee:@me"

Output assets/demo-views.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "nord"
Set TypingSpeed 60ms

Env LANG "en_US.UTF-8"
Env LC_ALL "en_US.UTF-8"

# 1. Show the user's view configuration
Type `cat ~/.config/gh-board/config.toml`
Enter
Sleep 2500ms

# 2. Launch gh-board against the demo project
Type `gh board --owner @me 2`
Enter
Sleep 2500ms

# 3. Jump to the first configured view (Backlog: table layout, no:status filter)
Type "1"
Sleep 2500ms

# 4. Switch to the second view (My Tasks: assignee:@me)
Type "2"
Sleep 2500ms

# 5. Back to the default board
Escape
Sleep 1200ms

# 6. Quit
Type "q"
Sleep 500ms
