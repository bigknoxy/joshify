# Joshify UI Redesign - Implementation Tasks

## Current Phase: Phase 1 - Foundation
**Status**: Preparing to start
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

## Phase 1: Foundation (IN PROGRESS)

### 1.1 Data Models & State

#### Create src/state/home_state.rs
- [ ] Define `HomeState` struct with:
  - `recently_played: Vec<RecentlyPlayedItem>`
  - `jump_back_in: Vec<ContinueContext>`
  - `is_loading: bool`
  - `last_updated: Option<Instant>`
- [ ] Define `RecentlyPlayedItem` struct
- [ ] Define `ContinueContext` struct with progress tracking
- [ ] Define `ContextType` enum (Album, Playlist)
- [ ] Add tests for state structures

#### Expand src/state/library_state.rs
- [ ] Add `AlbumListItem` struct
- [ ] Add `ArtistListItem` struct
- [ ] Add methods for library data management
- [ ] Add tests

#### Update src/state/app_state.rs
- [ ] Add `HomeDashboard(HomeState)` to `ContentState` enum
- [ ] Add `Library` variant with albums/artists/tabs
- [ ] Add `AlbumDetail` variant
- [ ] Add `ArtistDetail` variant
- [ ] Add `LibraryTab` enum

#### Update src/state/load_coordinator.rs
- [ ] Add `HomeData` action
- [ ] Add `LibraryAlbums` action
- [ ] Add `LibraryArtists` action
- [ ] Add `AlbumDetail { album_id: String }` action
- [ ] Add `ArtistDetail { artist_id: String }` action

### 1.2 API Layer

#### Update src/api/ (check existing structure)
- [ ] Add `get_recently_played(limit: u32)` method
- [ ] Add `get_user_albums()` method
- [ ] Add `get_user_artists()` method
- [ ] Add `get_album_tracks(album_id: &str)` method
- [ ] Add `get_artist_top_tracks(artist_id: &str)` method
- [ ] Add error handling for each endpoint
- [ ] Add tests with mock responses

### 1.3 Navigation Changes

#### Update src/state/app_state.rs
- [ ] Remove `Search` from `NavItem` enum
- [ ] Ensure `Library` exists and is functional

#### Update src/ui/sidebar.rs
- [ ] Remove Search from sidebar rendering
- [ ] Ensure all nav items are functional
- [ ] Update tests for new navigation

### Phase 1 Verification
- [ ] `cargo test --lib` passes
- [ ] `cargo clippy --message-format=short` passes
- [ ] `cargo fmt` passes
- [ ] All new files have tests
- [ ] Navigation structure verified manually

---

## Phase 2: Home Dashboard

### 2.1 Home Data Loading

#### Update src/state/load_coordinator.rs
- [ ] Implement HomeData load action handler
- [ ] Calculate "Jump Back In" from recently played
- [ ] Implement cache with 5-minute staleness check
- [ ] Add offline support: persist to disk
- [ ] Add tests

#### Create src/ui/home_view.rs
- [ ] Implement `render_home_dashboard()` function
- [ ] Implement `RecentlyPlayed` section
- [ ] Implement `JumpBackIn` section
- [ ] Implement `QuickAccess` section
- [ ] Implement empty states
- [ ] Add tests

### 2.2 Interactions

#### Update src/main.rs
- [ ] Handle HomeData load results
- [ ] Handle Home view interactions (clicks, keyboard)
- [ ] Update key handler for Home-specific shortcuts

#### Update src/ui/mouse_handler.rs
- [ ] Add ClickableArea variants for Home sections
- [ ] Implement click handlers for recently played items
- [ ] Implement click handlers for jump back in cards
- [ ] Implement click handlers for quick access buttons
- [ ] Add tests

### Phase 2 Verification
- [ ] Home loads with real data within 2 seconds
- [ ] Recently Played shows last 20 tracks with timestamps
- [ ] Jump Back In shows unfinished contexts with progress
- [ ] Quick Access buttons navigate correctly
- [ ] Empty states display for new users
- [ ] All interactions work
- [ ] Tests pass

---

## Phase 3: Library View

### 3.1 Library Structure

#### Create src/ui/library_view.rs
- [ ] Implement tab bar (Albums | Artists)
- [ ] Implement Albums grid view (4-6 columns)
- [ ] Implement Artists list view (alphabetical)
- [ ] Add tests

### 3.2 Albums Tab

#### Update API layer
- [ ] Implement album grid rendering
- [ ] Album art placeholders
- [ ] Album name truncation
- [ ] Artist name display
- [ ] Click to view album tracks
- [ ] Right-click context menu
- [ ] Add tests

### 3.3 Artists Tab

#### Update API layer
- [ ] Implement artist list rendering
- [ ] Artist name display
- [ ] Click to view artist top tracks
- [ ] Right-click context menu
- [ ] Add tests

### Phase 3 Verification
- [ ] Library loads user albums
- [ ] Library loads followed artists
- [ ] Tab switching works
- [ ] Grid layout responsive to terminal width
- [ ] Click interactions work
- [ ] Tests pass

---

## Phase 4: Detail Views

### 4.1 Album Detail

#### Create src/ui/album_view.rs
- [ ] Album header (name, artist, year, tracks)
- [ ] Track list with numbers
- [ ] Play album button
- [ ] Individual track play
- [ ] Add tests

### 4.2 Artist Detail

#### Create src/ui/artist_view.rs
- [ ] Artist header (name, genres, followers)
- [ ] Top tracks list
- [ ] Play top tracks button
- [ ] Radio button (shuffle play)
- [ ] Add tests

### Phase 4 Verification
- [ ] Album detail loads tracks
- [ ] Artist detail loads top tracks
- [ ] Play buttons work
- [ ] Navigation back to library works
- [ ] Tests pass

---

## Phase 5: Interactions & Polish

### 5.1 Keyboard Navigation

#### Update src/main.rs
- [ ] `h` - Go Home
- [ ] `l` - Go Library
- [ ] `p` - Go Playlists
- [ ] `Shift+L` - Go Liked Songs
- [ ] `Tab` - Switch tabs (in Library)
- [ ] Arrow keys navigate sections
- [ ] Document shortcuts in help

### 5.2 Smart Refresh

#### Update src/state/home_state.rs
- [ ] Auto-refresh on app focus detection
- [ ] Manual refresh with `r` key
- [ ] Stale data indicator
- [ ] Cache persistence across sessions

### 5.3 Loading States

#### Create/update loading components
- [ ] Skeleton screens for Home sections
- [ ] Spinner for initial load
- [ ] Progress indicators for long operations
- [ ] Empty states for each section

### Phase 5 Verification
- [ ] All keyboard shortcuts work
- [ ] Refresh works
- [ ] Loading states display correctly
- [ ] Empty states display correctly
- [ ] No regressions in existing features
- [ ] All tests pass

---

## Final Verification & Shipping

### Pre-Commit Checklist
- [ ] All phases complete
- [ ] `cargo test --lib` passes
- [ ] `cargo test --test performance_tests` passes
- [ ] `cargo clippy --message-format=short` passes
- [ ] `cargo fmt` passes
- [ ] No unwrap() in hot paths
- [ ] All text truncates with …
- [ ] Manual testing at 80x24 and larger terminals

### Documentation Updates
- [ ] Update learnings.md with new discoveries
- [ ] Update history.md with implementation details
- [ ] Update README.md if user-facing changes
- [ ] Update AGENTS.md if workflow changes

### Code Review
- [ ] Self-review complete
- [ ] @code-simplifier review (if available)
- [ ] Address all feedback

### QA Testing
- [ ] Run /qa or manual QA checklist
- [ ] Verify no regressions
- [ ] Verify new features work

### Commit & Ship
- [ ] Clean commit message
- [ ] Push to remote
- [ ] Verify CI passes

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

---

## Learnings to Capture

As we implement, watch for:
- API behavior surprises
- Performance bottlenecks
- UI/UX friction points
- Test coverage gaps
- Code patterns that should be reused

Update `.learnings/learnings.md` as we go!
