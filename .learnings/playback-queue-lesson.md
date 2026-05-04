# Learning: Playback Queue Position Management

## Date: 2025-01-XX
## Category: Bug Fix / Architecture

## The Bug
When selecting track N in a playlist, track N would play twice before continuing to track N+1.

## Root Cause
The `PlaybackQueue` uses `context_position` to track which track to return next from `advance()`. However, there was a semantic mismatch:

- `set_context_position(N)` means "tracks 0..N-1 have been played, next `advance()` returns track N"
- We were setting position to N (the selected track) then playing track N directly via API/player
- When track N ended, `advance()` was called and returned track N again (same index)

## The Fix
Instead of setting position to N+1 (which broke the "play from beginning" case), we:

1. Set position to the selected track's index N
2. After successfully starting playback, call `advance()` to "consume" the selected track
3. This moves position to N+1, so when the track ends naturally, `advance()` returns track N+1

## Key Insight
The queue's `advance()` method both returns a track AND increments position. We need to call it explicitly after direct playback starts to keep the queue in sync with what's actually playing.

## Code Pattern
```rust
// Before starting playback
queue.set_context_position(selected_index);

// Start playback directly (not via advance())
player.load_uri(&track.uri, true, 0)?;

// AFTER playback starts, consume the track from queue
let _ = queue.advance(); // Moves position to selected_index + 1
```

## Prevention
- When playing directly via API/player, always advance the queue explicitly
- The queue position represents "what advance() will return next", not "what's currently playing"
- Test both "play from start" and "play from middle" scenarios

## Files Modified
- `src/main.rs` - Enter key handler, mouse click handler, remote advance logic
- Added `advance()` calls after successful playback start in both handlers
- Removed duplicate `playback_next()` call in remote mode (Spotify handles auto-advance)
