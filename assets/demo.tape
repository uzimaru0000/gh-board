# gh-board demo recording (≈45s)
#
# Render with: vhs assets/demo.tape
# Output:      assets/demo.gif
#
# Prerequisites:
#   - gh extension install uzimaru0000/gh-board (or `cargo build` then alias `gh-board`)
#   - A populated project: ~12 cards across 3 columns, 1 issue with sub-issues,
#     1 PR with CI status, 1 with linked PRs
#   - Catppuccin Mocha theme installed at ~/.config/gh-board/theme.toml
#
# Tweak the project number on the `Type "gh board 1"` line and adjust sleeps
# to match your content.

Output assets/demo.gif

# Force English help / messages even when the host shell is non-English
Env LANG "en_US.UTF-8"
Env LC_ALL "en_US.UTF-8"

Set Shell "bash"
Set FontSize 16
Set Width 1280
Set Height 720
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 60ms

# 1. Cold start — cache hit makes the board appear instantly
Type "gh board 1"
Enter
Sleep 1500ms

# 2. Sweep across columns
Right Sleep 300ms
Right Sleep 300ms
Right Sleep 300ms
Left  Sleep 400ms

# 3. Grab a card and drop it in the next column
Space Sleep 250ms
Right Sleep 350ms
Right Sleep 350ms
Space Sleep 600ms

# 4. Open detail
Enter Sleep 1500ms

# 5. Scroll the Markdown body
Down Sleep 200ms
Down Sleep 200ms
Down Sleep 200ms
Down Sleep 400ms

# 6. Drill into a sub-issue and back out
Down Sleep 200ms
Down Sleep 200ms
Enter Sleep 1500ms
Escape Sleep 600ms

# 7. Add a reaction (adjust if your binding differs)
Type "+" Sleep 600ms
Enter Sleep 800ms

# 8. Back to board, switch to Roadmap layout
Escape Sleep 400ms
Tab Sleep 800ms

# 9. Filter using the new is:/no:/-(NOT) syntax
Type "/"
Type "is:open -label:bug"
Enter Sleep 1500ms

# 10. Help overlay (in English)
Type "?"
Sleep 1800ms
Escape Sleep 400ms

# 11. Quit
Type "q"
Sleep 500ms
