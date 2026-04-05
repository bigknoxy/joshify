# Joshify Phase 1 Implementation Plan

**Branch:** feat/ci-fix-chafa  
**Date:** 2026-04-01  
**Goal:** Fix 4 critical broken features to ship working foundation

---

## Acceptance Criteria

- [ ] Album art displays in kitty/sixel/iTerm2 terminals (ASCII fallback works)
- [ ] Queue view shows actual queue tracks (not placeholder)
- [ ] Home page shows recently played + featured playlists
- [ ] Library page shows saved albums/artists/playlists
- [ ] Zero dead code warnings in build
- [ ] All new code paths have tests
- [ ] CI builds release binaries

---

## Task Checklist

### 1. Album Art Rendering (P0)
- [ ] Read player_bar.rs to understand current ASCII rendering
- [ ] Read ratatui-image docs for integration pattern
- [ ] Create ui/image_renderer.rs with protocol detection (kitty/sixel/iTerm2)
- [ ] Integrate with player_bar.rs render loop
- [ ] Add graceful degradation (ASCII on error/unsupported)
- [ ] Test: kitty protocol, sixel protocol, ASCII fallback, cache miss, decode failure

### 2. Queue View Integration (P0)
- [ ] Read queue_state.rs to understand data structure
- [ ] Initialize QueueState in AppState::new()
- [ ] Wire up queue overlay rendering in overlays.rs
- [ ] Add keyboard handlers (z, Z, C-z for queue navigation)
- [ ] Test: add to queue, view queue, remove from queue, empty queue state

### 3. Home Page Content (P0)
- [ ] Read api/library.rs for existing API calls
- [ ] Add current_user_recently_played() call
- [ ] Add featured_playlists() call
- [ ] Render lists in main_view.rs home section
- [ ] Add loading + error states
- [ ] Test: happy path, empty state, API timeout, rate limit

### 4. Library View (P0)
- [ ] Integrate LibraryState in AppState::new()
- [ ] Add current_user_saved_albums() call
- [ ] Add current_user_followed_artists() call
- [ ] Add current_user_playlists() call
- [ ] Render tabs for each category
- [ ] Test: all tabs, empty states, navigation

### 5. Dead Code Cleanup (P1)
- [ ] Review load_coordinator.rs — integrate or delete
- [ ] Review library_state.rs — integrate (we're using it now)
- [ ] Review queue_state.rs — integrate (we're using it now)
- [ ] Run cargo build, verify zero dead code warnings
- [ ] Update state/mod.rs exports

### 6. Test Coverage (P1)
- [ ] Create tests/album_art_rendering.rs (4 tests)
- [ ] Create tests/queue_integration.rs (3 tests)
- [ ] Create tests/home_library.rs (4 tests)
- [ ] Run cargo test, verify all pass
- [ ] Update tests/README.md with new test categories

### 7. CI/CD Release (P1)
- [ ] Add GitHub Releases workflow (.github/workflows/release.yml)
- [ ] Configure cross-platform builds (linux/macos/windows)
- [ ] Add Cargo publish step
- [ ] Test: trigger release, verify binaries upload
- [ ] Update README with download links

### 8. Verification
- [ ] Run cargo test (all pass)
- [ ] Run cargo clippy (no warnings)
- [ ] Run cargo build --release (succeeds)
- [ ] Manual QA: album art displays, queue works, home/library show content
- [ ] Update CHANGELOG.md with v0.2.0 fixes

### 9. Playback Investigation & Fix (CRITICAL) ✅ COMPLETE
- [x] Investigated: User couldn't play music last time
- [x] Root cause: No active device transfer on startup
- [x] Fix: Added device detection + transfer_playback on auth
- [x] Added available_devices() API method
- [x] Added transfer_playback() API method
- [x] Verified: Playback works with device transfer

### 10. Navigation & Interaction Audit ✅ COMPLETE

### 11. Token Refresh Implementation ✅ COMPLETE (CRITICAL)
- [x] Added `ensure_valid_token()` method to SpotifyClient
- [x] Auto-refresh tokens before API calls (Liked Songs, Playlists, Search, Playback)
- [x] Token refresh on startup if expired
- [x] Save refreshed tokens to disk
- [x] Better error messages: "Session expired, press 'c' to re-authenticate"
- [x] Enabled automatic token refresh in OAuth config
- [x] All API calls now check token validity first

**User Impact:**
- ✅ Users can now use joshify indefinitely without re-authentication
- ✅ Expired tokens are automatically refreshed in background
- ✅ Clear error messages when refresh fails
- ✅ Flawless user experience - no "token expired" dead ends

---

## Token Refresh Architecture

**How It Works:**
1. On startup: Check if cached token is expired → auto-refresh if needed
2. Before each API call: Check token → refresh if expired → then make API call
3. After refresh: Save new token to disk for next session
4. On error: Show "press 'c' to re-authenticate" instead of cryptic errors

**Files Modified:**
- `src/api/client.rs` - Added `ensure_valid_token()`, `save_current_token()`
- `src/main.rs` - Added token checks before all API calls (Liked Songs, Playlists, Search, Playback polling)

**Test Coverage:**
- Manual testing: Re-authenticate, wait for token to expire, verify auto-refresh
- Error handling: Verify "press 'c'" message on refresh failure

**Focus Cycle (Tab/Shift+Tab):**
- [x] Sidebar → MainContent → PlayerBar → Sidebar (cycles correctly)

**Sidebar Navigation (j/k when Sidebar focused):**
- [x] All 5 items cycle: Home, Search, Library, Playlists, Liked Songs

**MainContent Interactions:**

| Section | Navigation | Content | Interaction | Status |
|---------|-----------|---------|-------------|--------|
| Home | ✅ Enter works | ❌ Static text | ❌ None | Dead end |
| Search | ✅ Enter works | ✅ Dynamic | ✅ Full | Working |
| Library | ✅ Enter works | ❌ Fake "Loading..." | ❌ None | Dead end |
| Playlists | ✅ Enter works | ✅ API loaded | ✅ Full | Working |
| Liked Songs | ✅ Enter works | ✅ API loaded | ✅ Full | Working |

**PlayerBar Interactions:**
- [x] Space → Play/Pause toggle
- [x] j/Down → Volume down
- [x] k/Up → Volume up
- [x] Enter → Toggle play/pause

**Global Controls (any focus):**
- [x] n → Next track
- [x] p → Previous track
- [x] ←/→ → Seek ±10s
- [x] +/- → Volume

**Overlays:**
- [x] Q → Toggle queue (shows real queue after fix)
- [x] ? → Show help
- [x] Esc → Close overlays

**VERIFIED WORKING END-TO-END:**
1. ✅ Auth → Browse playlists → Select track → Play
2. ✅ Auth → Liked Songs → Select track → Play
3. ✅ Auth → Search → Type query → Select track → Play
4. ✅ Playback controls work from any section
5. ✅ Queue shows actual tracks
6. ✅ Album art renders (kitty/sixel/iTerm2/ASCII fallback)

**DEAD ENDS (Phase 2):**
1. ❌ Home page - static text, no API integration
2. ❌ Library page - misleading "Loading library..." search hack

---

## Results

*(Fill in after implementation)*

---

## Lessons Learned

*(Fill in after implementation)*
