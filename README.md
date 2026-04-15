**English** | [日本語](README.ja.md)

# gh-board

A [gh](https://cli.github.com/) CLI extension to manage GitHub Projects V2 as a kanban board in your terminal.

## Installation

```
gh extension install uzimaru0000/gh-board
```

## Usage

```
gh board --owner <org-or-user> <number>
```

Running without arguments lets you choose from your accessible projects.

## Key Bindings

### Board

| Key | Action |
|-----|--------|
| `h` / `l` | Move between columns |
| `j` / `k` | Move between cards |
| `g` / `G` | First / last card |
| `Tab` / `S-Tab` | Next / previous column (wraps) |
| `H` / `L` | Move card to left / right column |
| `Space` | Grab card (reorder with h/j/k/l) |
| `Enter` | Open card detail |
| `n` | Create draft issue |
| `d` | Delete card |
| `/` | Filter |
| `p` | Switch project |
| `r` | Refresh |
| `?` | Help |
| `q` / `Esc` | Quit |

### Card Detail

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll vertically |
| `h` / `l` | Scroll table horizontally |
| `c` | Post new comment |
| `C` | Open comment list |
| `Enter` / `o` | Open in browser |
| `Esc` / `q` | Close |

### Filter

- Free text: partial match on card title
- `label:<name>`: filter by label
- `assignee:<name>`: filter by assignee (`@` prefix supported)
- `milestone:<name>`: filter by milestone
- Compound: `label:bug AND assignee:me`, `label:bug OR label:feature`

## Configuration

Customize settings in `~/.config/gh-board/config.toml`.

### Key Bindings

Override key bindings in `[keys.<mode>]` sections.

```toml
[keys.board]
move_down = ["n", "Down"]    # Use n instead of j to move down
refresh = ["R"]              # Use R instead of r to refresh
start_filter = ["/", "f"]   # Use f in addition to / for filter

[keys.global]
force_quit = ["C-q"]         # Use Ctrl+q instead of Ctrl+c to quit
```

Modes: `global`, `board`, `project_select`, `detail_content`, `detail_sidebar`, `card_grab`, `confirm`, `comment_list`, `status_select`, `sidebar_edit`, etc.

Key notation: `j`, `Enter`, `Esc`, `Tab`, `S-Tab`, `Space`, `Up`, `Down`, `Left`, `Right`, `Backspace`, `C-c` (Ctrl), `A-x` (Alt)

### Theme & Views

```toml
[theme]
accent = "red"

[[view]]
name = "Bugs"
filter = "label:bug"
```

## Building

```
cargo build
```

`schema.graphql` (GitHub GraphQL API schema) is automatically downloaded on the first build. Requires `gh` CLI to be installed and authenticated.

## License

MIT
