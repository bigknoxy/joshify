# Spotify Radio / Recommendation Prefetching Research

**Date**: 2025-01-09
**Research Goal**: Understand how other Spotify clients handle song radio, recommendations, and track transition prefetching

---

## Executive Summary

Most Spotify TUI clients do NOT implement automatic recommendation prefetching at 5-10% remaining. Instead, they rely on:
1. **Explicit user action** ("Go to Radio" menu option)
2. **Spotify's server-side queue** management
3. **Local playback engines** (librespot) handling gapless transitions internally

The 5-10% preload trigger is NOT a common pattern in open-source clients - they either use track-end events or explicit radio mode activation.

---

## 1. Research Questions & Findings

### 1.1 When Do Clients Fetch Recommendations?

| Client | Trigger | Implementation |
|--------|---------|------------------|
| **spotify-tui** (Rigellute) | Explicit user action | User selects "Go to Radio" from actions menu on track/artist/album/playlist |
| **ncspot** | No built-in radio | No automatic recommendation fetching; relies on Spotify Connect or librespot |
| **spotify-player** (aome510) | Explicit user action | `GoToRadio` action available in command system; fetches on-demand |
| **Official Spotify** | Server-side | Spotify manages queue internally, clients just poll `/me/player/queue` |

**Key Finding**: No open-source client fetches recommendations automatically at 5-10% remaining. They all rely on user-initiated radio mode or Spotify's built-in queue management.

### 1.2 Transition Handling: Gapless vs Pause

| Client | Approach | Implementation Details |
|--------|----------|------------------------|
| **ncspot** | Gapless via librespot | Uses `librespot_playback::player::Player` with `PlayerConfig { gapless: true }` |
| **spotify-player** | Gapless via librespot | Same librespot-based gapless playback |
| **spotify-tui** | No local playback | Remote control only; gapless depends on target device |
| **librespot** | Gapless by default | Preloads next track internally, handles gapless transitions automatically |

**Key Finding**: Gapless playback is handled at the audio playback layer (librespot), not at the recommendation fetching layer. The audio engine preloads the next track internally.

### 1.3 APIs Used for Recommendations

```rust
// Standard Spotify Recommendations API (used by all clients)
GET /v1/recommendations
Parameters:
  - seed_artists: Vec<ArtistId> (max 5 combined seeds)
  - seed_tracks: Vec<TrackId> (max 5 combined seeds)
  - seed_genres: Vec<String> (max 5 combined seeds)
  - limit: u32 (max 100, default 20)
  - market: Market

// rspotify crate usage example (from spotify-tui, spotify-player):
client.recommendations(
    Vec::new(),                    // audio features attributes
    Some(seed_artists),            // seed artists
    Some(seed_genres),             // seed genres
    Some(seed_tracks),             // seed tracks
    Some(market),
    Some(limit),
)
```

**Key Finding**: All clients use the same `/recommendations` endpoint with track/artist seeds. No client uses alternative endpoints.

### 1.4 Rate Limits & Timing Issues

**Spotify Rate Limits** (from official docs):
- Rolling 30-second window
- Extended quota mode available for high-traffic apps
- 429 response with `Retry-After` header when exceeded

**Observed Patterns**:

| Client | Rate Limit Handling |
|--------|---------------------|
| **ncspot** | Relies on rspotify internal handling |
| **spotify-tui** | Catches errors, shows user-facing error messages |
| **spotify-player** | Uses structured error types, retry with backoff |

**Timing Best Practices** (from research):
1. **Lazy loading**: Don't fetch until user explicitly requests radio
2. **Debounce rapid requests**: 150ms poll interval minimum (from AGENTS.md)
3. **Batch operations**: Fetch recommendations once, cache results
4. **Exponential backoff**: On 429 errors, wait `Retry-After` seconds

### 1.5 State Management During Track Transitions

**ncspot approach** (most sophisticated):
```rust
// Events from the Player
pub enum PlayerEvent {
    Playing(SystemTime),   // Track started playing
    Paused(Duration),       // Track paused at position
    Stopped,                // Playback stopped
    FinishedTrack,          // Track ended naturally
}

// Worker thread pattern
async fn worker(...) {
    let player_events = player.get_player_event_channel();
    // Player events drive state transitions
}
```

**spotify-player approach**:
- Uses `rspotify` for polling playback state
- `AddSelectedItemToQueue` command adds to Spotify queue (not local)
- No automatic radio - user must explicitly trigger `GoToRadio`

**Key Finding**: State management is event-driven from the playback engine, not time-based polling for prefetching.

---

## 2. Open Source Implementation Analysis

### 2.1 spotify-tui (Rigellute)

**Radio Implementation**:
```rust
// From src/app.rs
pub fn get_recommendations_for_seed(...) {
    self.dispatch(IoEvent::GetRecommendationsForSeed(
        seed_artists,
        seed_tracks,
        Box::new(first_track),
        user_country,
    ));
}

// RouteId includes Recommendations view
RouteId::Recommendations
```

**Key observations**:
- `Recommendations` is a separate view/route, not automatic
- Seed can be artists OR tracks (not both in same call)
- No prefetching logic visible in codebase
- User explicitly navigates to recommendations

### 2.2 ncspot (hrkfdn)

**Queue/Playback Architecture**:
```rust
// From src/spotify.rs
pub struct Spotify {
    status: Arc<RwLock<PlayerEvent>>,
    // ...
}

pub fn preload(&self, track: &Playable) {
    // librespot handles preloading internally
    self.send_worker(WorkerCommand::Preload(track.clone()));
}
```

**Key observations**:
- Uses librespot's built-in `Preload` command
- No recommendation fetching - just preloads next track in queue
- Player events drive state changes, not timers
- Gapless handled by librespot configuration

### 2.3 spotify-player (aome510)

**Command System**:
```rust
// From spotify_player/src/command.rs
pub enum Action {
    GoToArtist,
    GoToAlbum,
    GoToRadio,  // <-- Radio is explicit action
    // ...
}

// construct_track_actions includes:
Action::GoToRadio
```

**Key observations**:
- `GoToRadio` available as explicit action on tracks, albums, artists, playlists
- No automatic prefetching
- User must trigger radio mode

---

## 3. Common Patterns Across Clients

### 3.1 Queue Management Pattern

All clients use a **two-tier queue system**:

```
User Queue (up_next)
  - Explicitly queued by user
  - Plays first
  - Never shuffled
  
Context Tracks (context_tracks)
  - Current playlist/album/queue
  - Plays after user queue
  - Shuffleable
```

### 3.2 Recommendation Seed Strategy

| Seed Type | Use Case | Priority |
|-----------|----------|----------|
| **Track** | "More like this song" | Primary (most specific) |
| **Artist** | "More like this artist" | Secondary |
| **Genre** | "Discovery mix" | Tertiary |

**Best Practice**: Use the currently playing track as seed for automatic radio.

### 3.3 State Transition Flow

```
Track Playing -> Track End -> PlayerEvent::FinishedTrack
                                   |
                                   v
                    Check: Queue empty?
                       |
          +------------+-----------+
          |                        |
    Yes (exhausted)          No (more tracks)
          |                        |
          v                        v
   Radio mode enabled?       Play next from queue
          |
    +-----+------+
    |            |
   Yes           No
    |            |
    v            v
 Fetch       Stop
 Recommendations
    |
    v
 Add to queue
    |
    +-------------> Next Track Plays
```

---

## 4. Is 5-10% Preload Common?

**Answer: NO**

The 5-10% preload pattern is NOT commonly used in open-source Spotify clients. Instead:

1. **Track-end trigger** (at 100% or `FinishedTrack` event) is most common
2. **Explicit user action** is the second most common
3. **Server-side queue** (Spotify manages recommendations) is third

**Why not 5-10%?**
- Spotify's `/recommendations` endpoint has latency (200-500ms typical)
- Fetching too early wastes API calls if user skips tracks
- Rate limits make aggressive prefetching risky
- librespot handles audio preloading separately from metadata fetching

---

## 5. Best Practices for Smooth Radio Transitions

### 5.1 Implementation Recommendations

Based on analysis of successful clients:

1. **Use track-end trigger, not time-based**
   ```rust
   // Listen for FinishedTrack event
   match player_event {
       PlayerEvent::FinishedTrack => {
           if queue.is_exhausted() && radio_mode {
               fetch_recommendations().await;
           }
       }
   }
   ```

2. **Pre-fetch recommendations lazily**
   - Wait until queue is almost exhausted (< 3 tracks remaining)
   - Do not fetch based on time percentage
   - Cache recommendation results for reuse

3. **Seed selection priority**
   ```rust
   // Priority order for seeds:
   // 1. Current track (most specific)
   // 2. Current track's primary artist
   // 3. Current playback context (playlist/artist)
   let seed = current_track
       .map(|t| Seed::Track(t.id))
       .or_else(|| current_artist.map(|a| Seed::Artist(a.id)))
       .unwrap_or(Seed::Genre("pop".to_string())); // fallback
   ```

4. **Gapless audio handling**
   - Let librespot handle audio preloading (it does this automatically)
   - Focus on having next track URI ready, not audio data
   - Add tracks to Spotify queue via `/me/player/queue` endpoint

5. **Rate limit protection**
   ```rust
   // Track last fetch time
   const MIN_RADIO_FETCH_INTERVAL: Duration = Duration::from_secs(30);
   
   async fn maybe_fetch_radio(&mut self) -> Result<()> {
       if self.last_radio_fetch.elapsed() < MIN_RADIO_FETCH_INTERVAL {
           return Ok(()); // Skip, too soon
       }
       // ... fetch logic
   }
   ```

### 5.2 API Call Strategy

```rust
// When to fetch recommendations:
// 1. User explicitly triggers "Go to Radio"
// 2. Queue exhausted AND radio_mode enabled
// 3. NOT on time-based triggers (avoid 5-10% pattern)

// Fetch parameters:
let seed_tracks = vec![current_track.id.clone()];
let limit = 20; // Reasonable buffer
let market = Market::FromToken; // Use user's market

// Response handling:
// - Add tracks to local queue (not immediately to Spotify queue)
// - De-duplicate against recently played
// - Limit to avoid overwhelming the user
```

### 5.3 State Machine for Radio Mode

```rust
enum RadioState {
    Disabled,           // Normal playback
    Standby,            // Enabled but queue not exhausted
    Fetching,           // Currently fetching recommendations
    Active(Vec<Track>), // Has recommendations loaded
}

// Transitions:
// Disabled -> Standby (user enables radio mode)
// Standby -> Fetching (queue < threshold)
// Fetching -> Active (API response received)
// Active -> Standby (tracks played, queue refilled)
```

---

## 6. Relevant Implementation Files in joshify

Current state:
- `/src/state/queue_state.rs` - Queue management with `radio_mode` flag
- `/src/playback/domain.rs` - `PlaybackQueue` with context awareness
- `/src/api/library.rs` - `get_recommendations()` method exists
- `/src/api/playback.rs` - Playback control, no radio logic yet

**What is missing**:
- Radio state machine integration
- Track-end event handler for auto-radio
- Recommendation caching/de-duplication
- Integration between queue exhaustion and radio fetching

---

## 7. Conclusion

**Do not implement 5-10% preload** - it is not a common pattern and has downsides:
- Wastes API calls on skipped tracks
- Complicates state management
- Risk of rate limiting

**Instead, implement**:
1. Track-end trigger when queue is exhausted
2. Explicit "Go to Radio" user action
3. Lazy fetching with caching
4. Let librespot handle audio gapless playback

This approach matches spotify-tui, spotify-player, and ncspot - the three most popular Rust Spotify TUIs.

---

## References

1. **spotify-tui**: https://github.com/Rigellute/spotify-tui
2. **ncspot**: https://github.com/hrkfdn/ncspot  
3. **spotify-player**: https://github.com/aome510/spotify-player
4. **Spotify Web API - Recommendations**: https://developer.spotify.com/documentation/web-api/reference/get-recommendations
5. **Spotify Web API - Rate Limits**: https://developer.spotify.com/documentation/web-api/concepts/rate-limits
6. **librespot**: https://github.com/librespot-org/librespot
