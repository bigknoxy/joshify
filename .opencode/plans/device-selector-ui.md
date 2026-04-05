# Device Selector UI Implementation Plan

## Goal
Allow users to select which Spotify Connect device to play music on, instead of always using the currently active device.

## User Experience

### Current Behavior
- Press Enter on track → plays on whatever device is currently active
- No way to switch devices from TUI
- Must use phone/desktop app to change devices

### New Behavior
- Press 'd' → Opens device selector overlay
- Shows list of all available Spotify Connect devices
- Shows which device is currently active
- User navigates list with j/k or arrow keys
- Press Enter on device → transfers playback to that device
- Press Esc → closes overlay without changing device
- Shows device type icon (📱 phone, 💻 computer, 🔊 speaker, etc.)

## API Requirements

### Endpoints Used
1. **GET /me/player/devices** - Get available devices
   - Scope: `user-read-playback-state` (already have)
   - Returns: Array of Device objects
   
2. **PUT /me/player** - Transfer playback
   - Scope: `user-modify-playback-state` (already have)
   - Body: `{ "device_ids": ["<device_id>"], "play": true }`

### Device Object Structure (from Spotify API)
```json
{
  "id": "74ASZWbe4lXaubB36ztrGX",
  "is_active": false,
  "is_private_session": false,
  "is_restricted": false,
  "name": "Kitchen speaker",
  "type": "speaker",
  "volume_percent": 59,
  "supports_volume": true
}
```

## Implementation Steps

### 1. Add DeviceSelector to ContentState
**File:** `src/state/app_state.rs`

```rust
pub enum ContentState {
    // ... existing variants ...
    DeviceSelector(Vec<rspotify::model::Device>),
}
```

### 2. Add 'd' Key Handler
**File:** `src/main.rs`

Add key handler in the match statement:
```rust
crossterm::event::KeyCode::Char('d') => {
    // Open device selector
    app.content_state = ContentState::Loading(LoadAction::Devices);
}
```

### 3. Add LoadAction Variant
**File:** `src/state/app_state.rs` or `src/state/load_coordinator.rs`

```rust
pub enum LoadAction {
    // ... existing variants ...
    Devices,
}
```

### 4. Implement Device Loading
**File:** `src/main.rs` (in the tokio::spawn block)

```rust
LoadAction::Devices => {
    let guard = c.lock().await;
    match guard.available_devices().await {
        Ok(devices) => {
            let _ = tx_clone.send(ContentState::DeviceSelector(devices)).await;
        }
        Err(e) => {
            let _ = tx_clone.send(ContentState::Error(format!(
                "Failed to load devices: {}", e
            ))).await;
        }
    }
}
```

### 5. Render Device Selector UI
**File:** `src/ui/mod.rs` or new `src/ui/device_selector.rs`

```rust
pub fn render_device_selector(
    frame: &mut ratatui::Frame,
    area: Rect,
    devices: &[rspotify::model::Device],
    selected_index: usize,
) {
    // Split area into title + device list
    // Each device shows:
    //   - Icon based on type (📱 💻 🔊 🎮 🖥️)
    //   - Device name
    //   - Active indicator (▶ if active)
    //   - Volume if supported
    // Highlight selected device
    // Show instructions at bottom
}
```

### 6. Handle Device Selection
**File:** `src/main.rs` (Enter key handler)

```rust
ContentState::DeviceSelector(devices) => {
    if !devices.is_empty() && app.selected_index < devices.len() {
        let device = &devices[app.selected_index];
        if let Some(ref device_id) = device.id {
            if let Some(ref client) = client {
                let c = client.lock().await;
                match c.transfer_playback(device_id).await {
                    Ok(_) => {
                        app.status_message = Some(format!(
                            "Switched to {}", device.name
                        ));
                    }
                    Err(e) => {
                        app.status_message = Some(format!(
                            "Failed to switch: {}", e
                        ));
                    }
                }
            }
        }
        app.content_state = ContentState::Home;
    }
}
```

### 7. Handle Navigation in Device Selector
**File:** `src/main.rs` (j/k key handlers)

Need to handle navigation when `app.content_state` is `DeviceSelector`:
```rust
ContentState::DeviceSelector(devices) => {
    let len = devices.len();
    if len > 0 {
        // j/Down: app.selected_index = (app.selected_index + 1).min(len - 1);
        // k/Up: app.selected_index = app.selected_index.saturating_sub(1);
    }
}
```

### 8. Handle Esc to Close
**File:** `src/main.rs` (Esc key handler)

```rust
crossterm::event::KeyCode::Esc => {
    if matches!(app.content_state, ContentState::DeviceSelector(_)) {
        app.content_state = ContentState::Home;
        app.selected_index = 0;
    } else {
        // existing Esc behavior
    }
}
```

### 9. Add Device Type Icons
**File:** `src/ui/device_selector.rs`

```rust
fn device_icon(device_type: &DeviceType) -> &'static str {
    match device_type {
        DeviceType::Computer => "💻",
        DeviceType::Smartphone => "📱",
        DeviceType::Speaker => "🔊",
        DeviceType::Tv => "📺",
        DeviceType::Avr => "🎵",
        DeviceType::Stb => "📡",
        DeviceType::AudioDongle => "🔌",
        DeviceType::GameConsole => "🎮",
        DeviceType::CastVideo => "📹",
        DeviceType::CastAudio => "🔈",
        DeviceType::Automobile => "🚗",
        DeviceType::Unknown => "📻",
    }
}
```

## Testing Checklist

- [ ] Press 'd' opens device selector overlay
- [ ] Shows all available devices from Spotify
- [ ] Shows correct device type icons
- [ ] Highlights currently active device
- [ ] Can navigate with j/k or arrow keys
- [ ] Press Enter transfers playback to selected device
- [ ] Press Esc closes overlay without changing device
- [ ] Shows status message after successful transfer
- [ ] Shows error message if transfer fails
- [ ] Works with 0 devices (shows "No devices found")
- [ ] Works with 1 device (shows that device)
- [ ] Works with many devices (scrolls if needed)
- [ ] Restricted devices shown but marked as unavailable
- [ ] Volume shown for devices that support it

## UI Layout

```
╭──────────────────────────────────────────────╮
│         Select Playback Device               │
├──────────────────────────────────────────────┤
│ ▶ 📱 Josh's iPhone          [active]  75% ████│
│   💻 Josh's MacBook Pro              50% ██  │
│   🔊 Living Room Speaker           100% ████ │
│   📺 Samsung TV                   [restricted]│
│   🎮 PlayStation 5                    30% █   │
├──────────────────────────────────────────────┤
│ Enter: Switch  │ Esc: Cancel  │ j/k: Navigate│
╰──────────────────────────────────────────────╯
```

## Edge Cases

1. **No devices available**: Show "No devices found. Open Spotify on another device first."
2. **Device with no ID**: Skip or show as unavailable (can't transfer)
3. **Restricted device**: Show but mark as unavailable (is_restricted = true)
4. **Transfer fails**: Show error message, stay on device selector
5. **Device goes offline**: Spotify API handles this, won't appear in list

## Dependencies

- rspotify 0.16 (already upgraded) ✅
- DeviceType enum (in rspotify-model) ✅
- Scopes already have: `user-read-playback-state`, `user-modify-playback-state` ✅

## Files to Modify

1. `src/state/app_state.rs` - Add ContentState::DeviceSelector, LoadAction::Devices
2. `src/main.rs` - Add 'd' key handler, device loading, selection logic
3. `src/ui/mod.rs` - Export device selector render function
4. `src/ui/device_selector.rs` - NEW FILE - Device selector UI rendering
5. `src/api/playback.rs` - Already has available_devices() and transfer_playback()

## Success Criteria

✅ User can press 'd' to see all Spotify Connect devices  
✅ User can select a device and transfer playback to it  
✅ UI shows device type, name, active status, and volume  
✅ Works with all device types (phone, computer, speaker, etc.)  
✅ Graceful handling of edge cases (no devices, restricted, etc.)  

---

## Album Art Enhancement (Added by User Request)

### Current Issue
- Album art not displaying in player bar
- Duration shows 00:00 instead of actual track length

### Investigation Needed
1. Check if `track.duration.num_milliseconds()` returns 0 or valid value
2. Check if album art fetch is being triggered
3. Check if image data is being passed to renderer
4. Check if `show_album_art` config flag is being used

### Implementation Steps

1. **Debug Duration Issue**
   - Add logging to see what `track.duration` returns
   - Check if rspotify 0.16 uses different Duration type
   - Fix conversion to milliseconds

2. **Debug Album Art Fetch**
   - Verify `get_or_fetch()` is called when track changes
   - Check if image data is returned from fetch
   - Verify `app.player_state.current_album_art_data` is set

3. **Add Config Flag Support**
   - Add `show_album_art` field to OAuthConfig
   - Add `--no-album-art` CLI flag
   - Skip album art fetch if flag is false
   - Default to `true`

4. **Fix Player Bar Rendering**
   - Ensure album art data is passed to `render_player_bar()`
   - Check if `AlbumArtWidget` receives the data
   - Verify protocol detection works (kitty/sixel/iTerm2/ASCII)

5. **Testing**
   - Verify duration shows correct time (e.g., "03:45")
   - Verify album art appears for tracks with cover art
   - Verify `--no-album-art` flag disables fetching
   - Test on different terminals (kitty, iTerm2, regular)

### Files to Modify for Album Art

1. `src/state/player_state.rs` - Fix duration conversion
2. `src/auth.rs` - Add `show_album_art` config field
3. `src/main.rs` - Add CLI flag, conditionally fetch art
4. `src/ui/player_bar.rs` - Ensure art data is rendered
5. `src/ui/image_renderer.rs` - Verify protocol detection

### Success Criteria for Album Art

✅ Duration shows correct track length (not 00:00)  
✅ Album art displays when available  
✅ `--no-album-art` flag disables fetching (saves bandwidth)  
✅ Falls back gracefully when no art available  
✅ Works across different terminal emulators  
