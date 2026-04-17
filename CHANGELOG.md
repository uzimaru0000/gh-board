# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [1.1.0] - 2026-04-17

### Added

- Release update notification on the status bar (#64)
- Version info and self-update instructions in skill output (#62)

### Changed

- Faster initial load via lighter queries, lazy fetching, and progressive rendering
- Show spinner on column titles while the board is progressively rendering
- Show spinner on Table and Roadmap views during progressive rendering
- Center the Detail view loading indicator with a spinner

### Fixed

- Shift+Tab (BackTab) not working

### Removed

- Archived list TUI screen; `v` now opens the archive page in the browser

## [1.0.0] - 2026-04-16

### Added

- `item list` CLI subcommand with custom field values (#56)

### Changed

- **BREAKING**: Remove `card create-issue` subcommand; use `card create --type issue` instead (#57)
- Include field name in `item list` custom field output for readability

## [0.4.0] - 2026-04-16

### Added

- CLI subcommands for coding agent integration (#46)
- Preset color themes and theme.toml support (#44)

## [0.3.0] - 2026-04-15

### Added

- Roadmap view layout for Iteration fields (#28)
- Table view layout with per-view configuration (#27)

## [0.2.0] - 2026-04-15

### Added

- Sub-issue relationships with navigable sidebar modal (#25)
- Fuzzy filter for project selection (#2)
- Archive/unarchive support (#23)
- Grouping axis switching for kanban columns — SingleSelect / Iteration (#24)
- Server-side filtering with board cache (#22)
- Emoji reactions on issue/PR body and comments (#26)
- Linked PRs display in sidebar
- Custom field display and editing (#8)
- PR CI and review status on cards
- Scrollable state indicators with arrows and title counters (#35)

### Changed

- Replace Ctrl+S submit with Submit button in new card modal (#34)
- Use Nerd Font glyphs in detail view title

### Fixed

- Clear card area before rendering to fix phantom title
- Only include values for registered custom fields

### Removed

- Delete card action (replaced by archive)

## [0.1.1] - 2026-04-15

### Fixed

- Publish raw binaries for `gh extension install` compatibility

## [0.1.0] - 2026-04-15

### Added

- Kanban board view for GitHub Projects V2
- Card grab mode for reordering within and across columns (#6)
- Card detail view with 2-column layout and Markdown rendering (#3)
- Card editing for DraftIssue, Issue, and PR (#1)
- Comment posting, editing, and pagination (#4, #5)
- Draft issue and Issue creation with type selector and `$EDITOR` body editing
- Compound filters with AND/OR and milestone display/filter (#9, #10)
- Configurable key bindings via config file (#20)
- View feature: saved filter presets with tab bar (#19)
- Config file support for theme customization (#18)
- Skip project list loading when project number is specified (#16)
- Board UI design improvements (#17)
- English README
- CI/Release workflows
- MIT License

### Fixed

- Wide character bleeding through detail modal left border

[1.1.0]: https://github.com/uzimaru0000/gh-board/releases/tag/v1.1.0
[1.0.0]: https://github.com/uzimaru0000/gh-board/releases/tag/v1.0.0
[0.4.0]: https://github.com/uzimaru0000/gh-board/releases/tag/v0.4.0
[0.3.0]: https://github.com/uzimaru0000/gh-board/releases/tag/v0.3.0
[0.2.0]: https://github.com/uzimaru0000/gh-board/releases/tag/v0.2.0
[0.1.1]: https://github.com/uzimaru0000/gh-board/releases/tag/v0.1.1
[0.1.0]: https://github.com/uzimaru0000/gh-board/releases/tag/v0.1.0
