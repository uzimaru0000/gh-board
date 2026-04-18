# Demo recording script

A 30–60 second loop showcasing what makes gh-board interesting at a glance.
Aim for short, punchy beats — viewers should grasp each feature in <3s.

## Setup

- Terminal: 120×32 columns, large font (16pt+), dark background
- Theme: Catppuccin Mocha (`cp docs/themes/catppuccin-mocha.toml ~/.config/gh-board/theme.toml`)
- Project: pre-populated with ~12 cards across 3 columns (Todo / In Progress / Done),
  at least one Issue with sub-issues, one PR with a CI status, one with linked PRs
- Recording tool: `vhs` (preferred for reproducibility) or `asciinema` + `agg`,
  or screen recording at 60fps
- Crop to the terminal area only, export as 1080p mp4 + 720p gif (≤6MB)

## Shot list (≈45s total)

| # | Beat | Duration | Keys / actions |
|---|------|----------|----------------|
| 1 | Cold start (cache hit, instant render) | 2s | `gh board <num>` — emphasize the board appearing immediately |
| 2 | Navigate columns | 2s | `l l l h` — sweep across columns |
| 3 | Card grab & move | 4s | `Space l l Space` — visibly drag a card to another column |
| 4 | Open Detail | 3s | `Enter` — show Markdown, sidebar, comments |
| 5 | Scroll Markdown body | 2s | `j j j j` |
| 6 | Sub-issue drill-down | 3s | `j` to Sub-issues section, `Enter` on a sub-issue, then `Esc` |
| 7 | Reaction | 2s | `+` (or your binding) and pick 🚀 |
| 8 | Back to board, switch to Roadmap | 3s | `Esc`, then layout switch key — show iteration timeline |
| 9 | Filter with new syntax | 4s | `/` then type `is:open -label:bug`, `Enter` |
| 10 | Bulk select & move | 5s | enter bulk mode, mark 3 cards, move them together |
| 11 | Help overlay (i18n) | 3s | `?` — show the rich help screen |
| 12 | Quit | 1s | `q` |

## Captions to overlay (optional)

- 0:02 "Instant startup with on-disk cache"
- 0:05 "Grab & drop with `Space`"
- 0:12 "Markdown detail with sub-issues, reactions, linked PRs"
- 0:22 "Roadmap & Table layouts built-in"
- 0:28 "Powerful filters: `is:`, `no:`, `-label:`, `AND`/`OR`"
- 0:36 "Bulk operations"

## Output paths

- `assets/demo.gif` — README hero (replace existing)
- `assets/demo.mp4` — for X/Bluesky uploads (better quality than gif)
- `assets/og.png` — 1200×630 still extracted from frame ~0:04 for social previews

## VHS template

Save as `assets/demo.tape`, then `vhs assets/demo.tape`:

```tape
Output assets/demo.gif
Set FontSize 16
Set Width 1280
Set Height 720
Set Theme "Catppuccin Mocha"
Set Padding 20

Type "gh board 1"
Enter
Sleep 1500ms

Right Right Right Sleep 400ms
Left Sleep 400ms

# Grab & drop
Type " " Sleep 200ms
Right Right Sleep 400ms
Type " " Sleep 600ms

# Detail
Enter Sleep 1500ms
Down Down Down Down Sleep 800ms

# Back, then filter
Escape Sleep 400ms
Type "/"
Type "is:open -label:bug"
Enter Sleep 1500ms

# Help
Type "?"
Sleep 1500ms
Escape

Type "q"
```

Tweak sleeps to match your project content.
