# Double-Click Support and Playlist Context Playback

## Plan

### 1. Modify `src/ui/mouse_handler.rs`
- Add `PlayTrack` and `OpenPlaylist` actions to `MouseAction` enum
- Update `handle_left_click` to detect double-clicks and return appropriate actions
- Single-click: select item (existing behavior)
- Double-click: activate item (play track / open playlist)

### 2. Modify `src/main.rs`
- Add `PlaySelectedTrack` action handler for mouse events
- When playing a track from a playlist view, set playlist context
- Update playback logic to use playlist context with offset
- Store current playlist context in `app.current_context`

### 3. Add Unit Tests
- Test double-click detection in `mouse_handler.rs`
- Test playlist context playback setup

## Implementation Details

### MouseAction Changes
Add new actions:
- `PlayTrack(usize)` - Play track at index with context
- `OpenPlaylist(usize)` - Open playlist at index

### Double-Click Logic
- First click: select item (existing `SelectTrack`/`SelectPlaylist`)
- Second click (within threshold): `PlayTrack`/`OpenPlaylist`

### Playlist Context Playback
When double-clicking a track in a playlist view:
1. Get current playlist ID from `ContentState::PlaylistTracks(playlist_id, tracks)`
2. Create playlist URI: `spotify:playlist:{id}`
3. Call `start_context_playback` with playlist context and track offset

## Verification
```bash
cargo check
cargo test --lib
cargo test --bin joshify --test performance_tests
```
