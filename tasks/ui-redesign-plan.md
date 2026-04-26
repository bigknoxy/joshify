# Joshify UI Redesign Plan - MVP Phase

## User Decisions Summary

1. **Scope**: MVP first, validate, then iterate
2. **Recommendations**: Use Spotify API (not local pseudo-mixes)
3. **Player Bar**: Keep current bar (sidebar "Now Playing" widget optional/future)
4. **Library**: Full album/artist browsing
5. **Podcasts**: Out of scope
6. **Offline Support**: Yes - cached "Jump Back In" data

---

## MVP Definition: "Living Room Dashboard v0.1"

### What's In MVP

**Home View (the new star of the show):**
- [x] **Recently Played** - Last 20 tracks, clickable to replay
- [x] **Jump Back In** - Unfinished playlists/albums with progress
- [x] **Quick Access** - Buttons to Liked Songs, Playlists, Library
- [x] **Empty States** - Beautiful "Start Listening" for new users

**Navigation Changes:**
- [x] **Remove Search from sidebar** - `/` works everywhere
- [x] **Implement Library** - Albums + Artists (was dead item)
- [x] **Keep existing nav items** - Home, Library, Playlists, Liked Songs

**Player Bar:**
- [x] **No changes** - Keep current design

### What's NOT In MVP (Phase 2+)

- Generated Mixes (Spotify recommendations API)
- New Releases section
- Artist detail pages
- Album detail pages
- "Now Playing" sidebar widget
- Podcasts (out of scope entirely)
- Advanced keyboard shortcuts (`g h`, etc.)

---

## Technical Architecture

### New State Types

```rust
// src/state/home_state.rs
pub struct HomeState {
    /// Recently played tracks (last 20)
    pub recently_played: Vec<RecentlyPlayedItem>,
    /// Items to "jump back in" to (unfinished contexts)
    pub jump_back_in: Vec<ContinueContext>,
    /// Whether data is loading
    pub is_loading: bool,
    /// Last successful fetch
    pub last_updated: Option<Instant>,
}

pub struct RecentlyPlayedItem {
    pub track: TrackListItem,
    pub played_at: DateTime<Utc>,
    pub context: Option<PlayContext>, // Album, Playlist, Artist
}

pub struct ContinueContext {
    pub context_type: ContextType, // Album, Playlist
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
    pub progress_percent: u32, // 0-100
    pub last_played: DateTime<Utc>,
    pub total_tracks: u32,
    pub completed_tracks: u32,
}

pub enum ContextType {
    Album,
    Playlist,
    // Artist (radio) - future
}
```

### New Load Actions

```rust
// src/state/load_coordinator.rs
pub enum LoadAction {
    // ... existing actions ...
    
    // Home dashboard data
    HomeData,
    
    // Library browsing
    LibraryAlbums,
    LibraryArtists,
    
    // Album/artist detail (future)
    AlbumTracks { album_id: String },
    ArtistTopTracks { artist_id: String },
}
```

### New Content States

```rust
// src/state/app_state.rs
pub enum ContentState {
    // ... existing states ...
    
    /// New Home dashboard
    HomeDashboard(HomeState),
    
    /// Library with tabs
    Library { 
        albums: Vec<AlbumListItem>,
        artists: Vec<ArtistListItem>,
        selected_tab: LibraryTab,
    },
    
    /// Album detail view
    AlbumDetail(AlbumDetail),
    
    /// Artist detail view
    ArtistDetail(ArtistDetail),
}

pub enum LibraryTab {
    Albums,
    Artists,
}
```

---

## Implementation Phases

### Phase 1: Foundation (Day 1-2)

#### 1.1 Data Models & State
- [ ] Create `src/state/home_state.rs` with HomeState, RecentlyPlayedItem, ContinueContext
- [ ] Create `src/state/library_state.rs` (expand existing) with AlbumListItem, ArtistListItem
- [ ] Extend `LoadAction` enum with HomeData, LibraryAlbums, LibraryArtists
- [ ] Extend `ContentState` enum with HomeDashboard, Library, AlbumDetail, ArtistDetail

#### 1.2 API Layer
- [ ] Add `get_recently_played(limit: u32)` to API client
- [ ] Add `get_user_albums()` to API client
- [ ] Add `get_user_artists()` to API client
- [ ] Add `get_album_tracks(album_id: &str)` to API client
- [ ] Add `get_artist_top_tracks(artist_id: &str)` to API client

#### 1.3 Navigation Changes
- [ ] Remove `Search` from `NavItem` enum
- [ ] Implement actual `Library` navigation (was placeholder)
- [ ] Update sidebar rendering
- [ ] Update keyboard shortcuts documentation

### Phase 2: Home Dashboard (Day 3-4)

#### 2.1 Home Data Loading
- [ ] Implement HomeData load action
- [ ] Calculate "Jump Back In" from recently played + playback progress
- [ ] Cache home data with 5-minute staleness check
- [ ] Offline support: persist to disk, load on startup

#### 2.2 Recently Played Section
- [ ] Render list of last 20 tracks
- [ ] Show timestamp (e.g., "2 minutes ago", "3 hours ago")
- [ ] Show context (from Album, Playlist, etc.)
- [ ] Click to play track
- [ ] Right-click/long-press: "Go to Album", "Go to Artist", "Add to Queue"

#### 2.3 Jump Back In Section
- [ ] Render horizontal list of unfinished contexts
- [ ] Show progress bar overlay on album art placeholder
- [ ] Show "67% complete" or "12 of 20 tracks"
- [ ] Click to resume from saved position
- [ ] Right-click: "Start from beginning"

#### 2.4 Quick Access Section
- [ ] Buttons: Liked Songs, Playlists, Library Albums, Library Artists
- [ ] Click navigates to respective view
- [ ] Show counts (e.g., "Liked Songs (247)")

#### 2.5 Empty States
- [ ] "Welcome to Joshify" for new users
- [ ] "Start Listening" call-to-action
- [ ] "Connect to Spotify" if not authenticated

### Phase 3: Library View (Day 5-6)

#### 3.1 Library Structure
- [ ] Tab bar: Albums | Artists
- [ ] Albums grid view (4-6 columns depending on width)
- [ ] Artists list view (alphabetical)

#### 3.2 Albums Tab
- [ ] Fetch user's saved albums
- [ ] Grid layout with album art placeholders
- [ ] Album name (truncated), artist name
- [ ] Click to view album tracks
- [ ] Right-click: "Play", "Add to Queue", "Remove from Library"

#### 3.3 Artists Tab
- [ ] Fetch user's followed artists
- [ ] List layout with artist name
- [ ] Click to view artist top tracks
- [ ] Right-click: "Play Top Tracks", "Radio", "Unfollow"

### Phase 4: Detail Views (Day 7-8)

#### 4.1 Album Detail
- [ ] Show album header (name, artist, year, track count)
- [ ] List tracks
- [ ] Play button plays entire album
- [ ] Click track to play

#### 4.2 Artist Detail
- [ ] Show artist header (name, genres, follower count)
- [ ] List top tracks
- [ ] "Play Top Tracks" button
- [ ] "Radio" button (shuffle play)

### Phase 5: Interactions & Polish (Day 9-10)

#### 5.1 Keyboard Navigation
- [ ] `h` - Go Home
- [ ] `l` - Go Library
- [ ] `p` - Go Playlists
- [ ] `Shift+L` - Go Liked Songs
- [ ] `Tab` - Switch tabs (in Library)
- [ ] Arrow keys navigate sections

#### 5.2 Mouse Support
- [ ] Click "Jump Back In" cards
- [ ] Click Recently Played items
- [ ] Click Quick Access buttons
- [ ] Click Library tabs

#### 5.3 Smart Refresh
- [ ] Auto-refresh Home on app focus (check foreground)
- [ ] Manual refresh with `r` key
- [ ] Stale data indicator (subtle "updated 5 min ago")

#### 5.4 Loading States
- [ ] Skeleton screens for Home sections
- [ ] Spinner for initial load
- [ ] Progress indicators for long operations

---

## File Changes

### New Files
```
src/
  state/
    home_state.rs          # Home dashboard state
    library_detail_state.rs  # Album/artist detail
  ui/
    home_view.rs           # Home dashboard rendering
    library_view.rs        # Library (albums/artists) rendering
    album_view.rs          # Album detail rendering (future)
    artist_view.rs         # Artist detail rendering (future)
```

### Modified Files
```
src/
  state/
    app_state.rs           # Add HomeDashboard, Library content states
    load_coordinator.rs    # Add HomeData, LibraryAlbums, LibraryArtists actions
    mod.rs                 # Export new state modules
  ui/
    main_view.rs           # Handle new content states
    sidebar.rs             # Remove Search, implement Library
    mod.rs                 # Export new view modules
  api/
    mod.rs or client.rs    # Add new API endpoints
```

---

## API Endpoints Required

| Feature | Endpoint | Documentation |
|---------|----------|---------------|
| Recently Played | `GET /v1/me/player/recently-played` | [Link](https://developer.spotify.com/documentation/web-api/reference/get-recently-played) |
| User's Albums | `GET /v1/me/albums` | [Link](https://developer.spotify.com/documentation/web-api/reference/get-users-saved-albums) |
| User's Artists | `GET /v1/me/following?type=artist` | [Link](https://developer.spotify.com/documentation/web-api/reference/get-followed) |
| Album Tracks | `GET /v1/albums/{id}/tracks` | [Link](https://developer.spotify.com/documentation/web-api/reference/get-an-albums-tracks) |
| Artist Top Tracks | `GET /v1/artists/{id}/top-tracks` | [Link](https://developer.spotify.com/documentation/web-api/reference/get-artists-top-tracks) |

---

## Success Criteria for MVP

### Functional
- [ ] Home loads with real data within 2 seconds
- [ ] Recently Played shows last 20 tracks with accurate timestamps
- [ ] Jump Back In shows unfinished contexts with correct progress
- [ ] Library shows user's albums and artists
- [ ] All clicks/navigations work
- [ ] Keyboard shortcuts work
- [ ] Empty states display correctly for new users

### Performance
- [ ] Initial Home load < 2 seconds
- [ ] Tab switching < 500ms
- [ ] Smooth scroll in lists
- [ ] No UI freezing during data fetch

### Quality
- [ ] All text truncates correctly with …
- [ ] No layout overflow at 80x24 terminal
- [ ] Graceful degradation when API fails
- [ ] Offline mode works (cached data)

---

## Future Enhancements (Post-MVP)

### Phase 2
- Generated Mixes using Spotify Recommendations API
- New Releases section
- Now Playing sidebar widget
- Advanced keyboard shortcuts (`g h`, `g l`, etc.)
- Search history in Home

### Phase 3
- Full artist discography view
- Related artists
- Genre-based browsing
- Time-based greetings ("Good morning", etc.)
- Listening stats/insights

### Phase 4
- Collaborative playlists
- Friend activity
- Spotify Connect device management
- Lyrics integration
- Audio features visualization

---

## Testing Strategy

### Unit Tests
- HomeState calculations (progress %, filtering)
- LoadCoordinator action matching
- Navigation state transitions

### Integration Tests
- API endpoint responses
- Data transformation (Spotify → internal)
- Cache read/write

### UI Tests
- Layout at different terminal sizes
- Keyboard navigation flow
- Mouse click areas

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Spotify API rate limits | Medium | High | Implement caching, batch requests |
| Large library slow load | Medium | Medium | Pagination, virtual scrolling |
| Album art performance | Low | Medium | Lazy loading, blurhash placeholders |
| Offline cache size | Medium | Low | LRU eviction, configurable limit |

---

## Notes

- **Album Art**: Use placeholder boxes for MVP. Full art rendering in Phase 2.
- **Recommendations**: Spotify's recs API requires additional scopes. Plan for Phase 2.
- **Pagination**: Start with simple limit/offset. Cursor-based if needed.
- **Error Handling**: Graceful fallback to cached data or empty state.

---

## Next Steps

1. **Review this plan** - Does the MVP scope feel right?
2. **Prioritize Phase 1** - I can start with data models and navigation
3. **Iterate** - Build → Test → Refine → Ship

Ready to begin Phase 1 implementation?
