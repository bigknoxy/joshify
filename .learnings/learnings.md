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
