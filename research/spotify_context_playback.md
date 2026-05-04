# Spotify Context Playback Research

**Date**: 2026-04-29
**Scope**: Spotify Web API + rspotify 0.16.0
**Purpose**: Understand correct context playback flow for joshify

---

## 1. API Endpoint: Start/Resume Playback

### Endpoint
```
PUT https://api.spotify.com/v1/me/player/play?device_id={device_id}
```

### Required Scope
`user-modify-playback-state`

### Request Body (JSON) — Mutual Exclusivity Rules

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `context_uri` | string | **Either this OR `uris`** | Spotify URI of context (album, artist, playlist, show). Example: `"spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"` |
| `uris` | string[] | **Either this OR `context_uri`** | Array of track/episode URIs to play directly. Example: `["spotify:track:4iV5W9uYEdYUVa79Axb7Rh"]` |
| `offset` | object | Optional | Where to start in the context. **Only valid with `context_uri`** (not with `uris`). Two forms: `{"position": 5}` (zero-based index) or `{"uri": "spotify:track:..."}` |
| `position_ms` | integer | Optional | Start position in milliseconds within the starting track |

### Critical Constraint: `context_uri` vs `uris`
- **You must provide exactly one**: `context_uri` OR `uris`, never both
- If `context_uri` is provided, `offset` can reference position or URI within that context
- If `uris` is provided, `offset` is ignored (Spotify plays the array as-is)
- `position_ms` works with both modes

### Response
- `204 No Content` — Playback started successfully
- `401 Unauthorized` — Invalid/expired token
- `403 Forbidden` — Not Premium, or no active device
- `429 Too Many Requests` — Rate limited

---

## 2. Queue Endpoint

### Add to Queue
```
POST https://api.spotify.com/v1/me/player/queue?uri={track_uri}&device_id={device_id}
```

- Adds a **single track or episode** to the end of the user's playback queue
- Queue is **device-specific** (per-user, per-device)
- Returns `204 No Content` on success
- Requires `user-modify-playback-state` scope

### Get Queue
```
GET https://api.spotify.com/v1/me/player/queue
```

- Returns `{"currently_playing": TrackObject|null, "queue": [TrackObject]}`
- Requires `user-read-playback-state` or `user-read-currently-playing` scope

### Queue vs Context Playback — Key Differences

| Aspect | Context Playback | Queue |
|--------|-----------------|-------|
| **What it plays** | All tracks in a playlist/album/artist | Explicitly added tracks only |
| **Auto-advance** | Yes — plays entire context sequentially | Yes — plays queued tracks in order |
| **After context ends** | Stops (unless repeat is on) | Stops when queue is empty |
| **Adding during playback** | N/A (context is fixed) | Queued track plays **before** next context track |
| **Use case** | "Play this playlist starting at track X" | "Play this song next, then resume what I was listening to" |

### Queue Priority Behavior
When you add a track to the queue during context playback:
1. The queued track plays **immediately after the current track ends**
2. After the queued track finishes, playback **resumes from the context** (next track in the playlist/album)
3. The queue is FIFO — multiple additions queue in order

---

## 3. rspotify Crate Usage (v0.16.0)

### Key Types

#### `PlayContextId<'a>` — Context URIs
```rust
pub enum PlayContextId<'a> {
    Artist(ArtistId<'a>),
    Album(AlbumId<'a>),
    Playlist(PlaylistId<'a>),
    Show(ShowId<'a>),
}
```
- Implements `From<PlaylistId>`, `From<AlbumId>`, `From<ArtistId>`, `From<ShowId>`
- `.uri()` returns `"spotify:type:id"` format

#### `PlayableId<'a>` — Individual Tracks/Episodes
```rust
pub enum PlayableId<'a> {
    Track(TrackId<'a>),
    Episode(EpisodeId<'a>),
}
```
- Implements `From<TrackId>`, `From<EpisodeId>`
- Used for `start_uris_playback` and `add_item_to_queue`

#### `Offset` — Starting Position
```rust
pub enum Offset {
    Position(chrono::Duration),  // Zero-based index (Duration used as integer wrapper)
    Uri(String),                  // Track URI to start at
}
```
- **Use `Offset::Uri` for playlists** — more reliable than position index
- **Use `Offset::Position` for albums** — position is stable within an album
- Position is **zero-based** (0 = first track)

### Key Methods on `OAuthClient` Trait

#### `start_context_playback`
```rust
async fn start_context_playback(
    &self,
    context_uri: PlayContextId<'_>,
    device_id: Option<&str>,
    offset: Option<Offset>,
    position: Option<chrono::Duration>,
) -> ClientResult<()>
```
- Sends `PUT /me/player/play` with `context_uri` in body
- `offset` maps to `{"position": N}` or `{"uri": "..."}` in JSON
- `position` maps to `position_ms` in JSON

#### `start_uris_playback`
```rust
async fn start_uris_playback<'a>(
    &self,
    uris: impl IntoIterator<Item = PlayableId<'a>> + Send + 'a,
    device_id: Option<&str>,
    offset: Option<Offset>,
    position: Option<chrono::Duration>,
) -> ClientResult<()>
```
- Sends `PUT /me/player/play` with `uris` array in body
- `offset` is technically accepted but **ignored by Spotify** when using `uris`
- `position` maps to `position_ms`

#### `add_item_to_queue`
```rust
async fn add_item_to_queue(
    &self,
    item: PlayableId<'_>,
    device_id: Option<&str>,
) -> ClientResult<()>
```
- Sends `POST /me/player/queue?uri={uri}`

### Internal JSON Serialization (from rspotify source)

**`start_context_playback` builds:**
```json
{
    "context_uri": "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M",
    "offset": {"uri": "spotify:track:4iV5W9uYEdYUVa79Axb7Rh"},
    "position_ms": 0
}
```

**`start_uris_playback` builds:**
```json
{
    "uris": ["spotify:track:4iV5W9uYEdYUVa79Axb7Rh"],
    "offset": {"position": 0},
    "position_ms": 0
}
```

---

## 4. Code Examples

### Example 1: Play a playlist starting at a specific track
```rust
use rspotify::clients::OAuthClient;
use rspotify::model::{PlayContextId, PlaylistId, Offset};

async fn play_playlist_from_track(
    oauth: &impl OAuthClient,
    playlist_id: &str,
    track_uri: &str,
) -> anyhow::Result<()> {
    let pid = PlaylistId::from_id(playlist_id)?;
    
    oauth.start_context_playback(
        PlayContextId::from(pid),     // context_uri
        None,                          // device_id (use active)
        Some(Offset::Uri(track_uri.to_string())),  // start at this track
        None,                          // position_ms
    ).await?;
    
    Ok(())
}
```

### Example 2: Play an album from track 5 (by position)
```rust
use chrono::Duration;

oauth.start_context_playback(
    PlayContextId::from(album_id),
    None,
    Some(Offset::Position(Duration::milliseconds(5))),  // zero-based index
    None,
).await?;
```

### Example 3: Play a single track directly (no context)
```rust
use rspotify::model::{PlayableId, TrackId};

let track_id = TrackId::from_id("4iV5W9uYEdYUVa79Axb7Rh")?;
oauth.start_uris_playback(
    vec![PlayableId::Track(track_id)],
    None,
    None,  // offset ignored with uris
    None,  // position_ms
).await?;
```

### Example 4: Add track to queue
```rust
let track_id = TrackId::from_id("4iV5W9uYEdYUVa79Axb7Rh")?;
oauth.add_item_to_queue(
    PlayableId::Track(track_id),
    None,
).await?;
```

### Example 5: Play album with position offset (correct rspotify usage)
```rust
// NOTE: Offset::Position takes a chrono::Duration, but Spotify interprets
// the milliseconds value as a zero-based track index, NOT milliseconds.
// This is a rspotify quirk — the Duration is just a wrapper for an integer.
oauth.start_context_playback(
    PlayContextId::from(album_id),
    None,
    Some(Offset::Position(chrono::Duration::milliseconds(3))),  // 4th track (0-indexed)
    Some(chrono::Duration::milliseconds(30000)),  // 30 seconds into the track
).await?;
```

---

## 5. Correct Playback Flow

### Flow A: Play from Context (Playlist/Album)
```
User selects track #5 in playlist
    ↓
Call: start_context_playback(
    context_uri = "spotify:playlist:...",
    offset = Offset::Uri("spotify:track:track5_uri"),
    position_ms = None
)
    ↓
Spotify plays track #5
    ↓
Track #5 ends → Spotify auto-advances to track #6 (in context)
    ↓
Track #6 ends → track #7, etc.
    ↓
Context exhausted → playback stops (or repeats if repeat mode on)
```

### Flow B: Add to Queue During Context Playback
```
Context playback active (playing playlist, currently on track #3)
    ↓
User adds track X to queue
    ↓
Call: add_item_to_queue(PlayableId::Track(track_X))
    ↓
Track #3 ends → Spotify plays queued track X
    ↓
Track X ends → Spotify resumes context at track #4
    ↓
Continues through rest of playlist
```

### Flow C: Direct Track Playback (No Context)
```
User selects a single track (not from a playlist view)
    ↓
Call: start_uris_playback(
    uris = [track_uri],
    offset = None,
    position_ms = None
)
    ↓
Spotify plays only that track
    ↓
Track ends → playback stops (no context to advance through)
```

---

## 6. Limitations and Gotchas

### Gotcha 1: `Offset::Position` semantics
- In rspotify, `Offset::Position(Duration)` uses `num_milliseconds()` from the Duration
- Spotify interprets this value as a **zero-based track index**, NOT milliseconds
- So `Offset::Position(Duration::milliseconds(5))` means "start at track index 5" (the 6th track)
- This is confusing but matches the Spotify API spec

### Gotcha 2: `offset` is ignored with `uris`
- When using `start_uris_playback`, the `offset` parameter is sent to Spotify but **ignored**
- Spotify plays the `uris` array in order from the beginning
- If you need offset behavior, use `start_context_playback` instead

### Gotcha 3: `context_uri` and `uris` are mutually exclusive
- Sending both in the same request will cause an error
- rspotify's `start_context_playback` only sends `context_uri`
- rspotify's `start_uris_playback` only sends `uris`

### Gotcha 4: Queue is per-device
- The queue is associated with a specific device, not the user globally
- If you switch devices, the queue doesn't follow
- `add_item_to_queue` without a `device_id` targets the active device

### Gotcha 5: Context playback requires Premium
- All playback endpoints return `403 Forbidden` for free-tier users
- This is a Spotify policy, not an API limitation

### Gotcha 6: No "clear queue" endpoint
- Spotify's Web API has no endpoint to clear the queue
- Workaround: start a new context playback, which replaces the queue context

### Gotcha 7: `position_ms` vs `offset`
- `position_ms` = milliseconds into the **current track** (seek position)
- `offset` = which track to start at within the **context**
- They serve different purposes and can be used together

### Gotcha 8: Artist context playback
- `PlayContextId::Artist` is valid for `start_context_playback`
- Spotify generates a radio-like context from the artist's popular tracks
- No guaranteed order; `offset` may not work predictably with artist contexts

### Gotcha 9: Rate limiting
- Playback commands are subject to Spotify's rate limits
- Rapid successive calls may return `429 Too Many Requests`
- Implement exponential backoff for retries

---

## 7. Current Joshify Implementation Analysis

### What joshify does correctly:
- Uses `start_context_playback` with `Offset::Uri` for playlist playback (main.rs lines 2115-2120, 2917-2922, 2943-2948)
- Falls back to `start_playback` (direct URIs) when no playlist context is available
- Uses `add_item_to_queue` for queue management (library.rs line 190)
- Parses track URIs correctly for queue operations

### Potential issues in current code:

1. **`start_playback` parameter confusion** (playback.rs line 213-248):
   - The method signature takes `offset: Option<u32>` but passes it as `position` (4th param to `start_uris_playback`)
   - The parameter name `offset` is misleading — it's actually used as `position_ms`
   - In practice, this is always called with `None`, so it doesn't affect behavior

2. **No wrapper for `start_context_playback`**:
   - joshify calls `guard.oauth.start_context_playback(...)` directly
   - No error handling wrapper in the `SpotifyClient` impl
   - Consider adding a `start_context_playback` method to `playback.rs` for consistency

3. **Queue state is local-only**:
   - joshify maintains its own `QueueState` struct for display purposes
   - This is separate from Spotify's server-side queue
   - The local queue is used for the "local playback" mode (librespot), not the Spotify queue

---

## 8. References

### Official Spotify Documentation
- [Start/Resume Playback](https://developer.spotify.com/documentation/web-api/reference/start-a-users-playback)
- [Add Item to Playback Queue](https://developer.spotify.com/documentation/web-api/reference/add-to-queue)
- [Get the User's Queue](https://developer.spotify.com/documentation/web-api/reference/get-queue)
- [Player API Overview](https://developer.spotify.com/documentation/web-api/concepts/player)

### rspotify Documentation
- [rspotify 0.16.0 docs.rs](https://docs.rs/rspotify/0.16.0/rspotify/)
- [OAuthClient trait](https://docs.rs/rspotify/0.16.0/rspotify/clients/trait.OAuthClient.html)
- [PlayContextId enum](https://docs.rs/rspotify-model/0.16.0/rspotify_model/idtypes/enum.PlayContextId.html)
- [PlayableId enum](https://docs.rs/rspotify-model/0.16.0/rspotify_model/idtypes/enum.PlayableId.html)
- [Offset enum](https://docs.rs/rspotify-model/0.16.0/rspotify_model/offset/enum.Offset.html) (source: `rspotify-model-0.16.0/src/offset.rs`)

### rspotify Source Code (v0.16.0)
- `rspotify-0.16.0/src/clients/oauth.rs` — `start_context_playback` (line 1238), `start_uris_playback` (line 1274), `add_item_to_queue` (line 1447)
- `rspotify-model-0.16.0/src/idtypes.rs` — `PlayContextId` (line 483), `PlayableId` (line 526)
- `rspotify-model-0.16.0/src/offset.rs` — `Offset` enum (line 7)
