# Continuous Playback Fix - PLAN MODE

## Goal
Fix Spotify-style continuous playback so playing a track from a playlist continues through the rest of the playlist.

## Root Cause Analysis

After investigation, there are **three distinct issues**:

### Issue 1: Queue Never Populated (Remote Mode)
In `main.rs` lines 2174-2251, when Enter is pressed on a playlist track:
- Sets `current_context` with playlist info and `start_index`
- Calls `start_context_playback()` with `Offset::Uri`
- **NEVER calls `queue.set_context()` to populate context tracks**
- Result: `playback_queue.context_tracks` is empty, can't track position

### Issue 2: Silent Fallback to Single Track (Remote Mode)
In `playback/service.rs` lines 274-327:
- If `PlaylistId::from_id()` fails, silently falls through to `play_track_simple()`
- `play_track_simple()` uses `start_uris_playback()` with single track URI
- Result: No context, no auto-advance, plays just one track

### Issue 3: Local Mode Only Checks local_queue (Local Mode)
In `main.rs` lines 826-859:
- `EndOfTrack` handler only checks `!app.queue_state.local_queue.is_empty()`
- Never checks `playback_queue.remaining_context_tracks()`
- Result: When playing playlist (empty local_queue), no auto-advance

## Implementation Plan

### Phase 1: Populate Queue When Starting Playback
**File**: `src/main.rs` around lines 2174-2251

When user presses Enter on a track in `PlaylistTracks` view:
1. Get the full track list URIs from `ContentState::PlaylistTracks`
2. Call `app.queue_state.playback_queue_mut().set_context()` with:
   - The `PlaybackContext::Playlist`
   - Full list of track URIs from the playlist
3. Add debug logging to confirm queue population

### Phase 2: Fix Silent Fallback
**File**: `src/playback/service.rs` lines 274-327

Change `play_with_context()` to:
1. Return error instead of falling back to simple playback on parse failure
2. Only fallback to `play_track_simple()` on API call failure (after trying context)
3. Add explicit warning log when fallback occurs

### Phase 3: Fix Local Mode Auto-Advance
**File**: `src/main.rs` lines 826-859

In `EndOfTrack` event handler:
1. First check `local_queue` (existing behavior - user-added tracks)
2. If empty, check `playback_queue.remaining_context_tracks() > 0`
3. If context tracks remain, call `playback_queue.advance()` to get next URI
4. Load that URI via `player.load_uri()`

### Phase 4: Add Comprehensive Debug Logging
Add logging at key points:
- When `set_context()` is called (with track count)
- When playback starts (with context info)
- When fallback to simple playback occurs
- When track ends and what advance decision is made
- Queue state before/after advance

### Phase 5: Write Tests
- Test queue population on playlist playback start
- Test local mode auto-advance through context tracks
- Test fallback behavior logs warning
- Test queue + context interleaving

## Verification Checklist
- [ ] Play track 3 of playlist, verify track 4 plays automatically
- [ ] Verify logs show queue population and advance decisions
- [ ] Add track to queue mid-playlist, verify it plays next then resumes playlist
- [ ] All 451 library tests pass
- [ ] All 18 performance tests pass
