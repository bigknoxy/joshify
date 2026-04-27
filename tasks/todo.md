# Joshify UI Redesign - Implementation Tasks

## Current Phase: Phase 2 - Home Dashboard (IN PROGRESS)
**Status**: API methods added, Home/Library data loading implemented
**Last Updated**: 2026-04-25

---

## Phase 0: Pre-Implementation (DONE)

### ✅ Commit Current Changes
- [x] Stash and restore mouse interaction fixes
- [x] Create .learnings/ directory structure
- [x] Create learnings.md template
- [x] Create history.md template
- [x] Update AGENTS.md with workflow requirements

### ✅ Planning
- [x] Research existing Spotify TUIs (spotify-tui, spotify-player, ncspot)
- [x] Define MVP scope with user decisions
- [x] Create detailed implementation plan (ui-redesign-plan.md)
- [x] Create task tracking (this file)

---

## Phase 1: Foundation (COMPLETE)

### 1.1 Data Models & State (COMPLETE)

#### ✅ Create src/state/home_state.rs
- [x] Define `HomeState` struct with:
  - `recently_played: Vec<RecentlyPlayedItem>`
  - `jump_back_in: Vec<ContinueContext>`
  - `is_loading: bool`
  - `last_updated: Option<Instant>`
- [x] Define `RecentlyPlayedItem` struct with `TrackSummary` and `PlayContext`
- [x] Define `ContinueContext` struct with progress tracking
- [x] Define `ContextType` enum (Album, Playlist, Artist)
- [x] Add 9 tests for state structures

#### ✅ Expand src/state/app_state.rs
- [x] Add `HomeDashboard(HomeState)` to `ContentState` enum
- [x] Add `Library` variant with albums/artists/tabs
- [x] Add `LibraryTab` enum (Albums, Artists)
- [x] Add `AlbumListItem` struct (with artist field)
- [x] Add `ArtistListItem` struct

#### ✅ Update src/state/load_coordinator.rs
- [x] Add `HomeData` action
- [x] Add `LibraryAlbums` action
- [x] Add `LibraryArtists` action
- [x] Add `AlbumTracks { album_id: String, name: String }` action
- [x] Add `ArtistTopTracks { artist_id: String, name: String }` action

### 1.2 API Layer (COMPLETE)

#### ✅ Update src/api/library.rs
- [x] Add `get_recently_played(limit: u32)` method
- [x] Add `get_user_albums(limit: u32)` method
- [x] Add `get_user_artists(limit: u32)` method
- [x] Add `get_album_tracks(album_id: &str)` method
- [x] Add `get_artist_top_tracks(artist_id: &str)` method
- [x] Add error handling for each endpoint

### 1.3 Navigation Changes (COMPLETE)

#### ✅ Update src/state/app_state.rs
- [x] Remove `Search` from `NavItem` enum
- [x] Ensure `Library` exists and is functional

#### ✅ Update src/ui/sidebar.rs
- [x] Remove Search from sidebar rendering
- [x] Update tests for new navigation (4 items instead of 5)

### Phase 1 Verification (COMPLETE)
- [x] `cargo test --lib` passes (195 tests)
- [x] `cargo clippy --message-format=short` passes
- [x] All new files have tests

---

## Phase 2: Home Dashboard (IN PROGRESS)

### 2.1 Home Data Loading (COMPLETE)

#### ✅ Update src/main.rs
- [x] Implement HomeData load action handler in main.rs
- [x] Map rspotify PlayHistory to RecentlyPlayedItem
- [x] Calculate "Jump Back In" from recently played
- [x] Implement cache with 5-minute staleness check in HomeState

#### ✅ Create src/ui/home_view.rs (COMPLETE)
- [x] Implement `render_home_dashboard()` function
- [x] Implement `JumpBackIn` section with progress bars
- [x] Implement `RecentlyPlayed` section with timestamps
- [x] Implement `QuickAccess` buttons
- [x] Implement loading state with spinner
- [x] Implement empty state for new users
- [x] Add 3 tests for text truncation

### 2.2 Library View (COMPLETE - Basic)

#### ✅ Create/update library rendering in src/ui/main_view.rs
- [x] Add `render_library()` function with tab bar (Albums | Artists)
- [x] Implement Albums list view
- [x] Implement Artists list view placeholder
- [x] Add empty states for both tabs

#### ✅ Update src/main.rs
- [x] Implement LibraryAlbums load action handler
- [x] Map rspotify SavedAlbum to AlbumListItem
- [x] Add stub handlers for LibraryArtists, AlbumTracks, ArtistTopTracks

### Phase 2 Verification (IN PROGRESS)
- [ ] Home loads with real data within 2 seconds
- [ ] Recently Played shows last 20 tracks with timestamps
- [ ] Jump Back In shows unfinished contexts with progress
- [ ] Quick Access buttons navigate correctly
- [ ] Empty states display for new users
- [ ] Library shows saved albums
- [ ] All interactions work
- [ ] Tests pass

---

## Phase 3: Detail Views (PENDING)

### 3.1 Album Detail (PENDING)

#### Create src/ui/album_view.rs
- [ ] Album header (name, artist, year, tracks)
- [ ] Track list with numbers
- [ ] Play album button
- [ ] Individual track play
- [ ] Add tests

### 3.2 Artist Detail (PENDING)

#### Create src/ui/artist_view.rs
- [ ] Artist header (name, genres, followers)
- [ ] Top tracks list
- [ ] Play top tracks button
- [ ] Radio button (shuffle play)
- [ ] Add tests

---

## Phase 4: Interactions & Polish (PENDING)

### 4.1 Keyboard Navigation (PENDING)

#### Update src/main.rs
- [ ] `h` - Go Home
- [ ] `l` - Go Library
- [ ] `p` - Go Playlists
- [ ] `Shift+L` - Go Liked Songs
- [ ] `Tab` - Switch tabs (in Library)
- [ ] Arrow keys navigate sections
- [ ] Document shortcuts in help

### 4.2 Smart Refresh (PENDING)

#### Update src/state/home_state.rs
- [ ] Auto-refresh on app focus detection
- [ ] Manual refresh with `r` key
- [ ] Stale data indicator
- [ ] Cache persistence across sessions

---

## Final Verification & Shipping

### Pre-Commit Checklist
- [ ] All phases complete
- [x] `cargo test --lib` passes (195 tests)
- [x] `cargo clippy --message-format=short` passes
- [ ] `cargo fmt` passes
- [ ] No unwrap() in hot paths
- [ ] All text truncates with …
- [ ] Manual testing at 80x24 and larger terminals

### Documentation Updates
- [ ] Update learnings.md with new discoveries
- [ ] Update history.md with implementation details
- [ ] Update README.md if user-facing changes
- [ ] Update AGENTS.md if workflow changes

---

## Blocked Items

None currently

---

## Notes & Decisions

- Album art: Use placeholders for MVP, full rendering in Phase 2+
- Pagination: Start with simple limit/offset, cursor-based if needed
- Error handling: Graceful fallback to cached data or empty state
- Performance: Use virtual scrolling for long lists
- Offline: Cache to disk, load on startup
- **API Discovery**: rspotify uses `current_user_recently_played()` not `_manual`
- **Type Mapping**: rspotify Context has `_type: Type` field, not enum variants
- **PlayHistory**: Has `track: FullTrack`, `played_at: DateTime<Utc>`, `context: Option<Context>`

---

## Learnings to Capture

As we implement, watch for:
- API behavior surprises
- Performance bottlenecks
- UI/UX friction points
- Test coverage gaps
- Code patterns that should be reused

Update `.learnings/learnings.md` as we go!

(End of file - total 304 lines)
