# Joshify Development History

## 2026-04-25: UI Redesign Project Initiated

### Completed: Mouse Interaction Fixes (Phase 0)
**Status**: Complete, stashed for integration
**Files Modified**:
- `src/ui/sidebar.rs` - Border offset fix + tests
- `src/ui/layout_cache.rs` - Playlist hit testing + tests
- `src/ui/mouse_handler.rs` - Double-click detection + tests
- `src/main.rs` - Playlist context playback, local volume control
- `src/ui/mod.rs` - Export updates
- `tests/ui.rs` - Integration tests
- `tests/state.rs` - Additional tests

**Key Features**:
- Double-click support (300ms threshold, ±2px tolerance)
- Playlist item hit testing
- Local volume control for librespot
- Playlist context playback (local and remote)
- 58 new tests added (315 total passing)

### Started: UI Redesign Planning
**Goal**: Transform Home view from static welcome to "Living Room" dashboard
**Market Research**: Analyzed spotify-tui, spotify-player, ncspot, spotifyd
**Decisions Made**:
- MVP approach: validate then iterate
- Use Spotify API for recommendations
- Keep current player bar
- Full album/artist browsing
- Podcasts out of scope
- Offline cache support

**Plan Phases**:
1. Foundation (data models, navigation, API)
2. Home Dashboard (recently played, jump back in, quick access)
3. Library View (albums grid, artists list)
4. Detail Views (album tracks, artist top tracks)
5. Interactions & Polish

### Phase 1 Complete: Foundation Data Models (2026-04-25)
**Status**: Complete
**Commits**:
- `5334eb4` - feat: mouse interaction fixes and UI redesign planning
- `9e1b886` - feat(ui): Phase 1 foundation - home state data models
- `3d8da25` - fix(home_state): correct test expectations for jump back in
- `bb5a522` - feat(state): Add LoadAction variants for home and library
- `851b142` - feat(state): Add new ContentState variants for home and library

**What**:
- HomeState struct with recently played, jump back in tracking
- 5-minute staleness detection with caching
- 9 unit tests for home state logic
- LoadAction variants: HomeData, LibraryAlbums, LibraryArtists, AlbumTracks, ArtistTopTracks
- ContentState variants: HomeDashboard, Library (with Albums/Artists tabs)
- AlbumListItem and ArtistListItem structs

**Decisions**:
- HomeState handles its own staleness checking (not via LoadCoordinator)
- Progress calculation for Jump Back In: 10-90% range, min 2 tracks
- Library uses tab-based UI (not separate views)
- All new code includes comprehensive tests

**Files**:
- Created: `src/state/home_state.rs`
- Modified: `src/state/mod.rs`, `src/state/load_coordinator.rs`, `src/state/app_state.rs`, `src/ui/main_view.rs`

**Testing**:
- All 192 tests passing
- 9 new tests for home state
- No test regressions

**Learnings**:
- Test expectations must match actual algorithm behavior
- Pattern: State structs can self-manage staleness
- LoadAction enum needs display text for all variants

---

### 2026-04-26: Drill-Down Navigation System
**Branch**: feature/drill-down-navigation
**Status**: Complete, awaiting PR
**Owner**: AI Agent

**What**:
- Created NavigationStack with push/pop/peek and history tracking
- Extended ContentState with AlbumDetail and ArtistDetail variants
- Implemented browser-like back navigation with Backspace key
- Added Enter key handlers for all sidebar items (Home, Library, Playlists, LikedSongs)
- Added h/j/k/l vim-style navigation shortcuts
- Implemented Tab key to switch Library tabs (Albums/Artists)
- Created Album Detail view with header (name, artist, year, tracks) + track list
- Created Artist Detail view with header (name, genres, followers)
- Fixed play/pause button to show correct state text ("Play" vs "Pause")
- Cleaned up unused imports in home_view.rs
- Added API methods: get_top_artists, get_top_tracks, get_album_tracks
- Added 8 unit tests for NavigationStack

**Decisions**:
- Navigation stack stores (ContentState, selected_index) tuples for full state restoration
- Album/Artist detail views reuse existing track list rendering logic
- Spotify deprecated artist_top_tracks - stubbed with documentation
- 'h' goes to sidebar, 'l' goes to main content (mnemonic: h=left, l=right)
- Backspace mirrors browser back behavior

**Files**:
- Created: `src/state/navigation_stack.rs` (NEW module with 8 tests)
- Modified: `src/state/app_state.rs`, `src/state/mod.rs`
- Modified: `src/ui/main_view.rs` (detail view rendering)
- Modified: `src/ui/player_bar.rs` (play/pause text fix)
- Modified: `src/ui/home_view.rs` (cleanup imports)
- Modified: `src/api/library.rs` (new API methods)
- Modified: `src/main.rs` (navigation handlers, vim keys)

**Testing**:
- All 203+ tests passing
- Navigation stack: 8 new tests
- Release build compiles with 3 minor warnings
- No test regressions

**Learnings**:
- Navigation stack pattern works well for TUI drill-down
- Focus transfer requires explicit state change
- API endpoint deprecation happens - stub first, verify later
- Vim-style shortcuts integrate cleanly with existing handlers

---

## Template for New Entries

```markdown
### YYYY-MM-DD: [Feature/Bug/Change Name]
**Branch**: [branch-name]
**Status**: [In Progress | Complete | Blocked]
**Owner**: [who]

**What**:
- Bullet points of what was done

**Decisions**:
- Key decisions made and why

**Files**:
- List of created/modified files

**Testing**:
- Test coverage, manual verification

**Learnings**:
- What we learned
- What to remember for next time
```
