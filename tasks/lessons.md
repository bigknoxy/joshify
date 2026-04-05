# Lessons Learned

## 2026-04-03: librespot dev branch API

### Failure Mode: crates.io librespot 0.8.0 has vergen build bug
- **Signal**: `cargo build` fails with vergen compilation errors
- **Detection**: Issue #1681 on librespot repo
- **Prevention Rule**: Always use dev branch for librespot: `librespot = { git = "https://github.com/librespot-org/librespot", branch = "dev" }`

### Failure Mode: librespot uses unified `librespot` crate, not individual crates
- **Signal**: Import errors for `librespot_core`, `librespot_playback` etc.
- **Detection**: Check librespot examples on dev branch - they use `librespot::core`, `librespot::playback`, `librespot::connect`
- **Prevention Rule**: Use `librespot` as the main crate with feature flags, not individual sub-crates

### Failure Mode: OAuth token not available for local player init
- **Signal**: "Local player not initialized" message in UI despite successful OAuth
- **Root Cause**: Local player initialization only checked CLI args and env vars for access token, not the token stored inside the rspotify client after OAuth flow
- **Detection Signal**: `app.playback_mode == Remote` when it should be `Local` after successful OAuth
- **Prevention Rule**: When initializing local playback, extract token from ALL sources: CLI args → env vars → rspotify client token → librespot cache
- **Fix**: Added token extraction from `client.oauth.token` after SpotifyClient creation

### Failure Mode: MutexGuard lifetime issues with nested locks
- **Signal**: `E0597: borrowed value does not live long enough` when chaining `.lock().await` calls
- **Root Cause**: The temporary `Result` from `.lock().await` holds a borrow of the outer guard
- **Prevention Rule**: Split nested lock chains into separate statements:
  ```rust
  // BAD: if let Ok(guard) = outer.inner.lock().await { ... }
  // GOOD: let result = outer.inner.lock().await; if let Ok(guard) = result { ... }
  ```

### Failure Mode: Duplicate type definitions across modules
- **Signal**: `E0308: mismatched types` between ostensibly identical enums
- **Root Cause**: `DeviceEntry` was defined in both `app_state.rs` and `device_selector.rs` - Rust treats them as different types
- **Prevention Rule**: Define shared types in ONE module (preferably the state module) and import everywhere else

### Failure Mode: `take_event_channel()` returns Option
- **Signal**: `expected UnboundedReceiver, found Option<UnboundedReceiver>`
- **Root Cause**: `player.take_event_channel()` returns `Option<UnboundedReceiver>`, not the receiver directly
- **Prevention Rule**: Always check return type of librespot methods - many return Options

---

## 2026-04-03: librespot API patterns

### Session creation
```rust
let credentials = Credentials::with_access_token(token);
let session = Session::new(SessionConfig::default(), Some(cache));
session.connect(credentials, false).await?;
```

### Player creation
```rust
let backend = audio_backend::find(None)?;
let mixer = mixer::find(None)?(MixerConfig::default())?;
let player = Player::new(PlayerConfig::default(), session, mixer.get_soft_volume(), move || backend(None, AudioFormat::default()));
let event_rx = player.get_player_event_channel();
```

### Spirc (Spotify Connect) creation
```rust
let (spirc, spirc_task) = Spirc::new(
    ConnectConfig { name: "My Device", ..Default::default() },
    session, credentials, player, mixer
).await?;
tokio::spawn(async move { spirc_task.await; });
```

---

## 2026-04-03: Testing approach

### Binary crate testing
- joshify is a binary crate (no lib.rs), so integration tests can't use `joshify::` imports
- Use `#[path = "../src/module.rs"]` to include source files directly in tests
- Unit tests inside source files (`#[cfg(test)] mod tests`) work fine with `cargo test --bin joshify`
- Pre-existing test `tests/playback_api.rs` is broken (uses `joshify::` imports) - needs fixing separately
