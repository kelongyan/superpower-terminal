# Repository Guidelines

## Project Structure & Module Organization

This is a Rust workspace containing a Windows terminal emulator. Source code is organized into four crates under `crates/`:

- `superpower-core` — Terminal core logic (Cell, Grid, Cursor, Parser, Selection, DamageTracker)
- `superpower-pty` — PTY abstraction layer (PtySession, PtyEvent)
- `superpower-renderer` — GPU rendering pipeline (glyph atlas, text rendering, UI drawing)
- `superpower-app` — Application layer (config, event loop, UI shell)

Configuration files use TOML format. Assets are stored in `assets/`.

## Build, Test, and Development Commands

Run the application in development mode:
```powershell
cargo run -p superpower-app
```

Build optimized release binary:
```powershell
cargo build --release
```

Run tests across all workspace crates:
```powershell
cargo test
```

Check code without building:
```powershell
cargo check
```

## Coding Style & Naming Conventions

Follow standard Rust conventions:
- Use 4-space indentation
- Snake_case for functions, variables, and modules
- PascalCase for types, structs, and enums
- SCREAMING_SNAKE_CASE for constants
- Run `cargo fmt` before committing
- Address all `cargo clippy` warnings

## Testing Guidelines

Place unit tests in the same file as the code being tested using `#[cfg(test)]` modules. Integration tests go in `tests/` directories within each crate. Test naming should clearly describe the scenario being tested (e.g., `test_cursor_movement_wraps_at_boundary`).

## Commit & Pull Request Guidelines

Use conventional commit format with type prefixes:
- `feat:` for new features
- `fix:` for bug fixes
- `refactor:` for code restructuring
- `cleanup:` for non-functional improvements

Example: `feat: add productized desktop terminal shell ui`

Keep commit messages concise and descriptive. Focus on what changed and why, not how.
