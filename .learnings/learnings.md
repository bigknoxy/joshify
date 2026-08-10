# Learnings - joshify

## 2025-05-03

### Category: Bug Fix
**Learned**: Continuous playlist playback wasn't working because we never populated the PlaybackQueue with context tracks when starting playback.

**Context**: When user presses Enter on a playlist track, we set `current_context` but never called `queue.set_context()` with the full track list. This meant:
1. `playback_queue.context_tracks` was empty
2. `remaining_context_tracks()` returned 0
3. Auto-advance logic in `EndOfTrack` handler couldn't find next tracks

**Fix**: In `main.rs`, when Enter is pressed on a playlist track:
1. Extract track URIs from the tracks list
2. Call `queue_state.playback_queue_mut().set_context()` with context + URIs
3. Advance queue position to selected_index
4. Sync local_queue with the domain queue

**Prevention**: Always populate queue state when starting context playback. Add debug logging to verify queue state.

**Files**: `src/main.rs` (lines ~2187-2220)

---

### Category: Bug Fix
**Learned**: Silent fallback to single-track playback in `play_with_context()` was hiding parse failures.

**Context**: If `PlaylistId::from_id()` failed, code silently fell through to `play_track_simple()`, which uses `uris: [track_uri]` with NO context. This played exactly one track.

**Fix**: Changed to explicit `match` on parse result:
- Parse success → Try context playback, only fallback on API failure
- Parse failure → Return `PlaybackError::InvalidContext` with clear error message
- Added `InvalidContext` error variant

**Prevention**: Never silently fall back to degraded behavior. Always error loudly with context.

**Files**: `src/playback/service.rs` (lines ~274-400)

---

### Category: Bug Fix
**Learned**: Local mode `EndOfTrack` handler only checked `local_queue`, not context tracks.

**Context**: When playing a playlist in local mode, `local_queue` is empty (no user-added tracks). The `EndOfTrack` handler only checked `!local_queue.is_empty()`, so it never advanced.

**Fix**: Restructured handler with three phases:
1. Check `local_queue` (user-added tracks, highest priority)
2. If empty, check `playback_queue.remaining_context_tracks() > 0`
3. If context tracks exist, call `playback_queue.advance()` and load next URI
4. Log decisions at each phase for debugging

**Prevention**: When implementing queue logic, always check both user queue AND context tracks.

**Files**: `src/main.rs` (lines ~826-900)

---

### Category: Testing
**Learned**: All 6 auto-advance tests already existed from previous work and pass.

**Context**: Tests for queue advancement were already written:
- `test_queue_remaining_tracks_calculation`
- `test_queue_user_interruption_during_playback`
- `test_queue_exhaustion_detection`
- `test_queue_advance_source_tracking`
- `test_queue_shuffle_preserves_up_next`
- `test_queue_total_remaining_count`

**Verification**: All 451 library tests + 18 performance tests pass.

---

### Category: Borrow Checker
**Learned**: Be careful with match arm borrows that extend past the block.

**Context**: In the fix for populating the queue, I had:
```rust
if let Some(PlaybackContext::Playlist { uri, name, .. }) = &app.current_context {
    app.current_context = Some(PlaybackContext::Playlist {
        uri: uri.clone(),  // ERROR: uri borrowed in match arm
        ...
    });
    // use uri here
}
```

**Fix**: Clone values at start of match arm:
```rust
if let Some(PlaybackContext::Playlist { uri, name, .. }) = &app.current_context {
    let uri = uri.clone();  // Clone first
    let name = name.clone();
    app.current_context = Some(PlaybackContext::Playlist {
        uri: uri.clone(),
        ...
    });
    // use uri here - now it's a clone, not a borrow
}
```

**Prevention**: When mutating a field that's borrowed in a match arm, clone the borrowed values immediately.

---

### Category: Bug Fix
**Learned**: Selected track plays twice because calling `advance()` multiple times to "position" the queue consumes tracks.

**Context**: When user selects track 3, I called `advance()` 3 times to position the queue. But `advance()` returns AND consumes the track:
- Call 1: returns track 1, position=1
- Call 2: returns track 2, position=2  
- Call 3: returns track 3, position=3

When track 3 ends:
- Spotify auto-advances to track 4
- We call `handle_remote_track_advance()` → `advance()`
- This returns track 4, position=4 ✓

BUT we had a SECOND bug: duplicate queue population blocks. The first block set context, the second block set it AGAIN (resetting position to 0), then called `advance()` 3 times. This positioned the queue at track 4 instead of track 3.

**Fix**: 
1. Removed duplicate queue population code
2. Added `set_context_position()` method to set position without consuming tracks
3. Changed to use `set_context_position(selected_index)` instead of calling `advance()` in a loop

**Prevention**: 
- Don't duplicate code blocks
- Methods that consume should be clearly named (advance vs set_position)
- Test edge cases where starting position != 0

**Files**: 
- `src/playback/domain.rs` - Added `set_context_position()` method
- `src/main.rs` - Fixed to use new method, removed duplicate code

---

## Summary

Fixed three distinct bugs preventing continuous playlist playback:
1. ✅ Queue never populated with context tracks
2. ✅ Silent fallback to single-track playback
3. ✅ Local mode only checked user queue

All tests pass (451 lib + 18 perf). Clippy warnings unchanged (~38).

---

## 2026-08-08

### Category: Bug
**Learned**: Volume arithmetic `volume as u16 * 65535` overflows `u16` for any volume >= 2 (panic in debug, garbage in release). Must compute in `u32` then cast: `((volume * 65535) / 100) as u16`.

**Context**: All four keyboard volume paths in `src/main.rs` (2675, 2734, 2818, 2836) cast `u32` volume to `u16` before multiplying by 65535. The mouse path (3400) already used `u32` correctly.

**Prevention**: When scaling into `u16` (0-65535) from a percent, always do arithmetic in a wider type; grep for `as u16 * 65535`. Extract a single `percent_to_volume()` helper.

**File**: `src/main.rs`, filed as issue #10

---

### Category: Bug
**Learned**: The project advertises features that are stubs — CLI playback commands, daemon playback, desktop notifications, and MPRIS media control all print/log placeholder text and never touch Spotify or the OS. The README/CHANGELOG claim them as working.

**Context**: `src/cli.rs` `cmd_play`/`cmd_pause`/`cmd_status` etc. just `writeln!`; `src/daemon.rs` `execute_command` fabricates `Track from {uri}` / `Unknown Artist` / `duration_ms: 180000`; `src/notifications.rs:152` returns `StubNotifier`; `src/media_control.rs` logs "would initialize here".

**Prevention**: During code review, always verify advertised features actually call the real API/OS — grep for stub markers (`"would initialize"`, `"for now"`, `"Mock status"`, fabricated data). File as issues with mockall-based AC.

**File**: `src/cli.rs`, `src/daemon.rs`, `src/notifications.rs`, `src/media_control.rs`, filed as issues #12, #13, #23

---

### Category: Bug
**Learned**: `current_playback()` in `src/api/playback.rs` swallows real API errors (parse failures, 401/429, schema drift) by converting any error containing "player"/"device"/"404"/"400" into `Ok(None)`, so the TUI shows "Nothing playing" instead of the real error.

**Context**: The "any deserialization error = no playback" fallback (lines 60-61) would hide playback entirely if the Spotify schema changed.

**Prevention**: Distinguish genuine "no active playback" (empty/null/204/NO_ACTIVE_DEVICE) from parse/network/auth errors; propagate the latter.

**File**: `src/api/playback.rs`, filed as issue #17

---

### Category: Bug
**Learned**: Remote mode auto-advance pops the local queue (`queue.advance()` -> `next_uri`) but then calls Spotify's `playback_next()` instead of playing `next_uri`, silently discarding the user's queued track.

**Context**: `trigger_remote_advance` at `src/main.rs:296-330`. Spotify's server-side queue advances instead of the local entry.

**Prevention**: When a local queue entry is advanced, play that exact URI via `start_playback(vec![next_uri], None)`; only use `playback_next()` when no local items remain.

**File**: `src/main.rs`, filed as issue #14

---

### Category: Bug
**Learned**: The `'D'` remove-from-queue handler removes from `local_queue` only, never from `playback_queue.up_next`, so the two mirrors diverge and `next_track()` pops mismatched entries.

**Context**: `src/main.rs:1858-1875`, `src/state/queue_state.rs:55-64`. `next_track()` uses `local_queue.remove(0)` (O(n)) with no reconciliation.

**Prevention**: Removal must be atomic across both stores — add `QueueState::remove(uri)` that removes from both and unit test ordering consistency after removal.

**File**: `src/state/queue_state.rs`, `src/main.rs`, filed as issue #16

---

### Category: Pattern
**Learned**: `poll_playback` holds the shared `Arc<Mutex<SpotifyClient>>` across the entire network await every 2s, blocking all other API tasks (search, liked-songs, playlist loads) for the request duration.

**Context**: `src/main.rs:153-275`. Every background task needs the same lock.

**Prevention**: Never hold a client Mutex across a network await; release after setup or snapshot the needed data, or run poll in its own scoped task.

**File**: `src/main.rs`, filed as issue #18

---

### Category: Gotcha
**Learned**: `install.sh` fails on machines without `chafa` because `ratatui-image`'s `build.rs` hard-requires `chafa >= 1.8.0` via pkg-config, and the installer neither installs it nor uses the prebuilt release tarballs that `release.yml` already publishes.

**Context**: Live failure: 504 crates compiled (~6 min) then panicked with `Failed to find chafa via pkg-config`. CI installs `libchafa-dev` but `install.sh` does not.

**Prevention**: Installers should prefer downloading prebuilt release assets; when falling back to source builds, install native deps per distro (`libchafa-dev`/`chafa`). Reproduce installer in Docker containers in CI.

**File**: `install.sh`, `.github/workflows/release.yml`, filed as issue #11

---

### Category: Pattern
**Learned**: GitHub issue backlog is the source of truth for the audit. No issues existed before this audit; 17 were filed (#10-#26) with testable AC and labels (bug/build/cli/daemon/playback/queue/api/performance/priority:high|medium|low).

**Context**: Full repo audit on 2026-08-08 covering code review, feature review (stubs), build tooling, CI health, and docs drift.

**Prevention**: Keep `tasks/AUDIT_BACKLOG.md` and `tasks/todo.md` in sync with the GH issues; mark issues closed only when AC is verified by tests.

**File**: `tasks/AUDIT_BACKLOG.md`, `tasks/todo.md`

---

## 2026-08-09

### Category: Pattern
**Learned**: To unit-test a concrete struct that wraps a network client (e.g. `SpotifyClient` wrapping `AuthCodeSpotify`), introduce a small mockable async trait (`CliClient`) implemented by the concrete type, and have the consumer (`CliHandler`) depend on the trait. This lets mockall generate a mock without touching the real client.

**Context**: P0-3 (issue #12) — CLI commands were stubs printing mock text. Wired them to the real API via a `CliClient` trait + mockall tests.

**Prevention**: When a feature needs to be testable against a network client, abstract the needed methods behind a trait first. Use `async-trait` for async trait methods.

**File**: `src/api/cli_client.rs`, `src/cli.rs`

### Category: Gotcha
**Learned**: mockall's `mock!` macro for async traits requires `async_trait` to be in scope, and cannot mock methods with `Option<&str>` parameters without explicit lifetime annotations. Use owned types (`Option<String>`) in the trait signature instead.

**Context**: P0-3 — `seek(&self, position_ms, device_id: Option<&str>)` failed to compile in the mock with E0106/E0637.

**Prevention**: Keep trait method params owned (`Option<String>`) and convert to borrowed form inside the impl.

**File**: `src/api/cli_client.rs`

### Category: Gotcha
**Learned**: `Box<dyn Write>` cannot be downcast to recover a test buffer. To capture CLI output in tests, use a writer that shares an `Arc<Mutex<Vec<u8>>>` and read the buffer through the Arc.

**Context**: P0-3 — tried `downcast::<Cursor<Vec<u8>>>()` on the boxed output; `Box<dyn Write>` has no `downcast`.

**Prevention**: For output-capturing tests, use a shared-buffer writer (`Arc<Mutex<Vec<u8>>>`) rather than trying to downcast the boxed trait object.

**File**: `src/cli.rs`
