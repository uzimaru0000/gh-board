# Contributing to gh-board

Thanks for considering a contribution! This guide covers everything you need to
get a change landed.

## Development setup

Required tooling:

- Rust (edition 2024) — install via [rustup](https://rustup.rs/)
- [`gh`](https://cli.github.com/) CLI, authenticated (`gh auth login`)
  - Used at build time to download `schema.graphql` if missing
  - Used at runtime to fetch your auth token

Clone and build:

```bash
git clone https://github.com/uzimaru0000/gh-board
cd gh-board
cargo build
cargo run -- --owner <org-or-user> <project-number>
```

## Project layout

See [`CLAUDE.md`](CLAUDE.md) for a tour of the codebase, the
Functional Core / Imperative Shell architecture, and where each concern lives.

Quick map:

- `src/app_state/` — pure state machine and tests (the "core")
- `src/app.rs` — async shell that turns `Command` values into side effects
- `src/ui/` — ratatui rendering
- `src/github/` — GraphQL client + queries
- `src/cli.rs` — `gh board <subcommand>` for scripting / agents

## Workflow

1. **Open an issue first** for non-trivial changes so we can align on scope.
2. Create a branch from `main`: `feat/<short-name>` or `fix/<short-name>`.
3. Follow the **TDD loop** for new behavior:
   - Add a test in `src/app_state/mod.rs` (`#[cfg(test)] mod tests`) that
     fails first, then make it pass. See `CLAUDE.md` for examples.
4. Run the checks before pushing:
   ```bash
   cargo test
   cargo clippy --tests -- -D warnings
   ```
5. Conventional-ish commit messages: `feat:`, `fix:`, `refactor:`, `docs:`,
   `chore:`. Japanese summaries are fine.
6. Open a PR and fill out the template. UI changes should include a screenshot
   or short clip.

## Code style

- Prefer editing existing files over creating new ones.
- Don't add comments that just restate the code — only when *why* is non-obvious.
- Side effects belong in `App` (`src/app.rs`), not `AppState`. State transitions
  return a `Command` value; the shell executes it.
- Keep tests deterministic — no real network calls in `app_state` tests.

## Releasing (maintainers)

See `.claude/skills/release/SKILL.md` for the release flow. Briefly:

1. Run `/release` in Claude Code (or follow the skill manually).
2. Push the tag — `release.yml` builds binaries and creates the GitHub Release.

## License

By contributing, you agree your contributions will be licensed under the MIT
License (same as the project).
