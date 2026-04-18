**English** | [日本語](README.ja.md)

# gh-board

A [gh](https://cli.github.com/) CLI extension to manage GitHub Projects V2 as a kanban board in your terminal.

![demo](assets/demo.gif)

## Quick Start

```bash
gh extension install uzimaru0000/gh-board
gh board                       # pick from your projects
gh board --owner <org> <num>   # open a specific project
```

## Why gh-board?

| | gh-board | `gh project` (official) | Web UI |
|---|---|---|---|
| Kanban board view | ✅ | ❌ (list only) | ✅ |
| Card detail with Markdown / comments / reactions | ✅ | ❌ | ✅ |
| Drag & reorder with the keyboard | ✅ (`Space`) | ❌ | drag-only |
| Roadmap / Iteration timeline | ✅ | ❌ | ✅ |
| Bulk operations | ✅ | ❌ | partial |
| Sub-issue navigation | ✅ | ❌ | ✅ |
| Works fully offline-cached | ✅ | ❌ | ❌ |
| No browser required | ✅ | ✅ | ❌ |

If you live in the terminal but `gh project` feels too thin and the Web UI breaks
your flow, gh-board fills the gap.

## Features

- **Board / Table / Roadmap views** — switch between kanban board, table, and roadmap layouts
- **Card grab & reorder** — move cards between columns or reorder within a column
- **Card detail** — Markdown rendering with table support, comments, emoji reactions, linked PRs, CI/review status
- **Sub-issues** — parent/child relationship display with navigable sidebar drill-down
- **Custom fields** — display and edit SingleSelect, Iteration, and other project fields
- **Grouping axis** — switch the kanban column grouping by SingleSelect or Iteration fields (`Ctrl+g`)
- **Archive / Unarchive** — archive cards from the board or restore them from the archive list
- **Comments** — post new comments and edit your own (`$EDITOR` integration)
- **Filter** — free text, `label:`, `assignee:`, `milestone:`, compound `AND`/`OR`
- **Fuzzy project selection** — quickly find projects with fuzzy matching
- **Local board cache** — XDG-compliant on-disk cache with stale-while-revalidate for instant startup
- **i18n** — help screen available in English and Japanese (auto-detects from `LANG` / `LC_ALL`)
- **Configurable key bindings & theme** — customize via `config.toml` / `theme.toml`

## Installation

```
gh extension install uzimaru0000/gh-board
```

To upgrade later:

```
gh extension upgrade gh-board
```

## Usage

```
gh board --owner <org-or-user> <number>
```

Running without arguments lets you choose from your accessible projects.

### Options

| Flag | Description |
|------|-------------|
| `--owner <LOGIN>` | Owner login (org or user). Use `@me` for the current viewer. |
| `--no-cache` | Skip the local board cache for this run (forces a fresh fetch). |

### Cache

Board responses are cached at `$XDG_CACHE_HOME/gh-board/board/` (e.g. `~/.cache/gh-board/board/` on Linux, `~/Library/Caches/gh-board/board/` on macOS). On startup, a cached board is rendered immediately while the latest data is fetched in the background. Mutations (move, archive, etc.) invalidate the cache automatically. Cache entries expire after 24 hours. Pass `--no-cache` to bypass this for a single run.

## Key Bindings

### Board

| Key | Action |
|-----|--------|
| `h` / `l` | Move between columns |
| `j` / `k` | Move between cards |
| `g` / `G` | First / last card |
| `Tab` / `S-Tab` | Next / previous column (wraps) |
| `Space` | Grab card (reorder with h/j/k/l, release with Space/Esc) |
| `Enter` | Open card detail |
| `n` | Create draft issue |
| `a` | Archive card (with confirmation) |
| `v` | View archived cards |
| `Ctrl+g` | Switch grouping axis |
| `/` | Filter (`C-u` to clear) |
| `p` | Switch project |
| `r` | Refresh |
| `?` | Help |
| `q` / `Esc` | Quit |

### Card Detail

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll vertically |
| `h` / `l` | Scroll table horizontally |
| `c` | Post new comment (Issue/PR only) |
| `C` | Open comment list |
| `Enter` / `o` | Open in browser |
| `Esc` / `q` | Back (pops detail stack) |

### Comment List

| Key | Action |
|-----|--------|
| `j` / `k` | Move between comments |
| `e` | Edit your own comment |
| `c` | Post new comment |
| `Esc` | Back to detail |

### Archive List

| Key | Action |
|-----|--------|
| `j` / `k` | Move between cards |
| `Enter` | Open in browser |
| `u` | Unarchive card |
| `r` | Reload |
| `Esc` / `q` | Back to board |

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

### Theme

Place a theme file at `~/.config/gh-board/theme.toml` to change the color theme.
See [`docs/themes/`](docs/themes/) for preset themes.

```bash
# Example: apply Catppuccin Mocha
cp docs/themes/catppuccin-mocha.toml ~/.config/gh-board/theme.toml
```

### Views

```toml
[[view]]
name = "Bugs"
filter = "label:bug"

[[view]]
name = "My Tasks"
filter = "assignee:@me"
layout = "table"
```

## Building

```
cargo build
```

`schema.graphql` (GitHub GraphQL API schema) is automatically downloaded on the first build. Requires `gh` CLI to be installed and authenticated.

## License

MIT
