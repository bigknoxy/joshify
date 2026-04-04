# AGENTS.md — joshify

Remote Spotify Client TUI with local playback support.

## Quick Commands

```bash
cargo run                          # Run dev build
cargo build --release              # Release binary (validate perf)
cargo test --bin joshify --test performance_tests  # Run all tests (81 pass)
cargo test --bin joshify --test performance_tests -- test_event_batch  # Single test
cargo clippy --message-format=short  # Lint (100 warnings exist, don't regress)
cargo fmt                          # Format (no rustfmt.toml, uses defaults)
```

**Integration tests** (`tests/playback_api.rs` etc.) fail to compile due to `use of unresolved module or unlinked crate joshify`. Bin + perf tests pass cleanly.

## Architecture

```
src/
  main.rs          — Event loop, key handling, playback orchestration
  player/          — Local playback (librespot integration)
  session.rs       — Local session management
  api/             — Spotify REST client (rate_limit, playback, library)
  auth/            — OAuth flow, credential management
  state/           — app_state, player_state, queue_state, library_state, load_coordinator
  ui/              — sidebar, main_view, player_bar, overlays, image_renderer, theme
  album_art.rs     — Art fetching + caching
  keyring_store.rs — Secure credential storage (OS keyring)
```

## Code Style

### Imports
- Group: std → external crates → crate modules (blank line between groups)
- Use explicit `use crate::` prefix for internal modules
- Re-export in `mod.rs` for public API surface (see `state/mod.rs`, `ui/mod.rs`)

### Types & Naming
- `PascalCase` for structs/enums, `snake_case` for functions/variables
- Derive `Debug, Clone, Default` where applicable
- Use `Option<T>` for nullable fields, never `unwrap()` in hot paths
- Enums use `PascalCase` variants with explicit `#[default]` attribute

### Error Handling
- Use `anyhow::Result` for top-level operations
- Use `tracing` for structured logging (not `println!`)
- Degrade gracefully — never crash on API failure or missing data

### Performance (CRITICAL)
- **Pre-process heavy work once**: Album art decoded/resized/encoded on arrival, stored as pre-formatted string for per-frame rendering
- **Batch event processing**: Max 32 events per loop iteration
- **Cooldown timers**: 2-second minimum between album art fetches
- **Poll interval**: 150ms for player events (not every frame)
- **Never** do heavy computation (image processing, network I/O) in the render loop
- Use `saturating_*` math to prevent overflow panics

### TUI Conventions
- All text must truncate with `…` to fit within borders at any terminal width
- Global quit (`q`/`Ctrl+C`) at top of key handler — works from ANY state
- Catppuccin Mocha color theme (`ui/theme.rs`)
- Non-transparent modals for overlays

## Testing

- Unit tests live in `#[cfg(test)] mod tests` blocks alongside source
- Performance regression tests in `tests/performance_tests.rs`
- Test naming: `test_module_function_scenario` (e.g., `test_queue_add_single_track`)
- Use `mockall` for mocking, `serial_test` for ordered tests
- Add smallest test that would have caught the bug

## Key Patterns to Follow

1. **State isolation**: Each domain has its own state module with clear boundaries
2. **Coordinator pattern**: `load_coordinator` manages async data loading
3. **Queue auto-advance**: On `EndOfTrack`/`Stopped`, pull from local queue
4. **Context playback**: Use `Offset::Uri(track_uri)` for playlist context (not `Offset::Position`)
5. **Single-level tokio spawn**: Avoid nested `tokio::spawn` — flatten async boundaries

## Known Issues

- 100 clippy warnings (pre-existing, mostly deprecated fields/unused imports)
- Integration test crate linking broken (bin + perf tests work)
- `cargo clippy --fix` can apply 24 suggestions safely
