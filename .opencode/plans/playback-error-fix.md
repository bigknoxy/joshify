# Playback Error Fix - Test Plan & Implementation

## Problem Identified
**Symptom:** "Playback error: Failed to get current playback state" even though music started playing on phone.

**Root Cause Analysis:**
1. OAuth token was cached with limited scopes
2. New scopes added to code don't apply to existing cached tokens
3. Error handling wasn't catching all "no device" error patterns

## Fixes Implemented

### 1. Improved Error Matching ✅
**File:** `src/api/playback.rs`

**Before:**
```rust
if err_str.contains("NO_ACTIVE_DEVICE") || err_str.contains("no active device") {
    Ok(None)
}
```

**After:**
```rust
let err_lower = err_str.to_lowercase();
let is_no_device_error = err_lower.contains("no active device") 
    || err_str.contains("NO_ACTIVE_DEVICE")
    || err_lower.contains("no device found")
    || err_lower.contains("no player found")
    || (err_lower.contains("player") && err_lower.contains("inactive"));

if is_no_device_error {
    Ok(None)
}
```

**Added Debug Logging:**
```rust
eprintln!("DEBUG: Playback API error: {}", err_str);
eprintln!("DEBUG: Treating as no-device error (returning Ok(None))");
```

### 2. Full OAuth Scopes ✅
**File:** `src/api/client.rs`

**Added missing scopes:**
- `user-read-birthdate`
- Ensured all playback/library scopes are present

### 3. Token Refresh ✅
**Already implemented** - tokens now auto-refresh before API calls

## Test Coverage

### Unit Tests Created
**File:** `tests/playback_error.rs`

```rust
✅ test_error_string_matching
✅ test_error_matching_case_insensitive
✅ test_error_matching_underscore
```

**Run:** `cargo test --test playback_error`

### Integration Tests (Manual)
**File:** `tests/playback_api.rs` (requires credentials)

```rust
#[ignore] test_current_playback_no_device
#[ignore] test_available_devices
#[ignore] test_transfer_playback
```

**Run:** `cargo test --test playback_api -- --ignored` (needs env vars)

## Verification Steps

### For User to Test

1. **Clear old credentials** (CRITICAL - old token has limited scopes):
   ```bash
   rm ~/.config/joshify/credentials.json
   ```

2. **Run joshify**:
   ```bash
   cargo run
   ```

3. **Re-authenticate**:
   - Browser opens automatically
   - Log into Spotify
   - Accept ALL permissions (you'll see more scopes now)
   - Redirects back to terminal

4. **Verify playback**:
   - Should see "✓ Loaded cached credentials" or "✓ Token refreshed successfully"
   - Should see "Connected to Spotify on 'Your Device' - Press ? for help"
   - Music should be playing on your phone
   - Player bar at bottom should show track info
   - NO "Playback error" message

5. **Test each section**:
   ```
   Enter on "Liked Songs" → Should load tracks
   Enter on "Playlists" → Should load playlists
   Press Q → Queue should show
   Press Space → Should toggle play/pause
   Press n/p → Next/previous track
   ```

## Expected Output

**On successful startup:**
```
⏳ Cached token expired, attempting refresh...
✓ Token refreshed successfully
✓ Loaded cached credentials
Connected to Spotify on 'Josh's iPhone' - Press ? for help
```

**On playback polling (every second):**
```
DEBUG: Playback API error: ...  (only if error occurs)
DEBUG: Treating as no-device error (returning Ok(None))  (if no device)
```

**If still getting errors:**
- Look for the DEBUG output to see the ACTUAL error message
- This will tell us exactly what Spotify is returning

## Files Modified

| File | Changes | Lines |
|------|---------|-------|
| `src/api/playback.rs` | Improved error matching + debug logging | +15 |
| `src/api/client.rs` | Added full OAuth scopes | +2 |
| `tests/playback_error.rs` | NEW - Error matching tests | +40 |
| `tests/playback_api.rs` | NEW - Integration tests | +70 |

## Success Criteria

- [ ] No "Playback error" message on startup
- [ ] Player bar shows current track
- [ ] Space bar toggles play/pause
- [ ] n/p keys change tracks
- [ ] Liked Songs loads without error
- [ ] Playlists load without error
- [ ] Queue shows real tracks
- [ ] Album art renders

## Next Steps

1. User clears old credentials
2. User re-authenticates
3. User verifies playback works
4. If errors persist → check DEBUG output for actual error message
5. Remove debug logging once confirmed working
