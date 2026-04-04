# Option 3: Local Playback with librespot

## Executive Summary

**Goal:** Transform joshify from a **remote control** into a **full Spotify Connect receiver** that plays audio locally on your machine.

**Scope:** Major refactor (~3000-5000 lines of new code)
**Timeline:** 2-4 weeks of focused development
**Complexity:** High - requires audio pipeline, Spotify Connect protocol, credentials management

---

## What Changes

### Current Architecture (Remote Control)
```
┌─────────────┐     Spotify API      ┌──────────────┐
│   Joshify   │ ───────────────────► │ Spotify Servers │
│   (TUI)     │ ◄─────────────────── │ (Web API)      │
└─────────────┘   JSON responses     └──────────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │ Your Phone/PC   │  ← Actual audio plays here
                 │ (Spotify App)   │
                 └─────────────────┘
```

### New Architecture (Local Playback)
```
┌─────────────────────────────────────────────────────────┐
│                      Joshify                            │
│  ┌─────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   TUI   │  │  librespot  │  │   Audio Backend     │ │
│  │ (ratatui)│──│  (Spotify   │──│ (rodio/ALSA/Pulse)  │ │
│  │         │  │  Connect)   │  │                     │ │
│  └─────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │   Your Speakers │  ← Audio plays HERE
                 │   (local)       │
                 └─────────────────┘
```

---

## Technical Requirements

### 1. Dependencies to Add

```toml
[dependencies]
# Core librespot crates
librespot = "0.8.0"                    # Main library
librespot-core = "0.8.0"               # Authentication, session
librespot-playback = "0.8.0"           # Audio playback
librespot-metadata = "0.8.0"           # Track metadata
librespot-discovery = "0.8.0"          # Spotify Connect discovery (mDNS)

# Audio backend (choose one)
rodio = "0.20"                         # Cross-platform (default, recommended)
# OR
cpal = "0.15"                          # Lower-level audio I/O
# OR system-specific:
# alsa = "0.9"                        # Linux ALSA
# pulse-simple = "0.3"                # Linux PulseAudio

# Audio decoding
libmad = "0.11"                        # MP3 decoding
# OR
ogg = "0.9"                            # Ogg Vorbis
vorbis = "0.0.23"                      # Vorbis decoding

# Additional utilities
tokio-stream = "0.1"                   # Async stream utilities
futures = "0.3"                        # Async utilities
byteorder = "1.5"                      # Binary data handling
```

### 2. System Dependencies (Linux)

```bash
# Ubuntu/Debian
sudo apt-get install build-essential libasound2-dev libportaudio2-dev

# Fedora
sudo dnf install alsa-lib-devel portaudio-devel

# macOS (rodio backend - no extra deps needed)
# Just ensure Xcode command line tools installed

# Windows (rodio backend - no extra deps needed)
# Visual Studio Build Tools
```

---

## Implementation Phases

### Phase 1: Core Integration (Week 1)

#### 1.1 Session Management
**File:** `src/session.rs` (NEW)

```rust
use librespot_core::{
    session::Session,
    config::SessionConfig,
    credentials::Credentials,
    spotify_id::SpotifyId,
};

pub struct JoshifySession {
    session: Session,
    credentials: Credentials,
}

impl JoshifySession {
    pub async fn connect(username: &str, password: &str) -> Result<Self>;
    pub async fn connect_spotify_oauth(token: &str) -> Result<Self>;
    pub fn get_session(&self) -> &Session;
}
```

**Tasks:**
- [ ] Create session module
- [ ] Implement OAuth token → librespot credentials conversion
- [ ] Handle reconnection on session expiry
- [ ] Store session in Arc<Mutex> for TUI access

#### 1.2 Audio Player
**File:** `src/player.rs` (NEW)

```rust
use librespot_playback::{
    player::{Player, PlayerConfig, PlayerEvent},
    audio_backend::Sink,
    config::AudioFormat,
};

pub struct JoshifyPlayer {
    player: Player,
    event_channel: mpsc::Receiver<PlayerEvent>,
}

impl JoshifyPlayer {
    pub fn new(session: &Session, backend: &str) -> Self;
    pub fn load_track(&self, spotify_id: SpotifyId, start_position_ms: u32);
    pub fn play(&self);
    pub fn pause(&self);
    pub fn stop(&self);
    pub fn seek(&self, position_ms: u32);
    pub fn set_volume(&self, volume: u16);  // 0-65535
    pub fn get_event_channel(&self) -> &mpsc::Receiver<PlayerEvent>;
}
```

**Tasks:**
- [ ] Create player module
- [ ] Configure audio backend (rodio default)
- [ ] Implement play/pause/stop/seek
- [ ] Handle PlayerEvent updates (for UI progress bar)
- [ ] Volume control (0-65535 scale)

#### 1.3 Spotify Connect Receiver
**File:** `src/connect.rs` (NEW)

```rust
use librespot_connect::{
    discovery::Discovery,
    spirc::Spirc,
};

pub struct JoshifyConnect {
    spirc: Spirc,
    discovery: Option<Discovery>,
}

impl JoshifyConnect {
    pub fn new(session: &Session, player: &Player, device_name: &str) -> Self;
    pub fn start_discovery(&mut self) -> Result<()>;
    pub fn stop_discovery(&mut self);
}
```

**Tasks:**
- [ ] Create Spotify Connect module
- [ ] Register device on network (mDNS)
- [ ] Handle incoming playback requests from other devices
- [ ] Show "Joshify on [hostname]" in Spotify app device list

---

### Phase 2: TUI Integration (Week 2)

#### 2.1 Update Main Event Loop
**File:** `src/main.rs`

**Current:**
```rust
// Polls Spotify Web API every second
match client.current_playback().await {
    Ok(Some(ctx)) => { /* update UI */ }
    Ok(None) => { /* show "Nothing playing" */ }
    Err(e) => { /* show error */ }
}
```

**New:**
```rust
// Listen to librespot PlayerEvents
tokio::spawn(async move {
    while let Some(event) = player_event_rx.recv().await {
        match event {
            PlayerEvent::TrackChanged { .. } => { /* update UI */ }
            PlayerEvent::Playing { .. } => { /* update play icon */ }
            PlayerEvent::Paused { .. } => { /* update pause icon */ }
            PlayerEvent::VolumeChanged { volume } => { /* update volume */ }
            // ... handle all events
        }
    }
});
```

**Tasks:**
- [ ] Remove Web API polling loop
- [ ] Add PlayerEvent listener
- [ ] Update player bar from events (not polling)
- [ ] Handle track changes, play/pause, volume

#### 2.2 Playback Controls
**File:** `src/main.rs` (key handlers)

**Update existing handlers:**
```rust
// Space bar - toggle play/pause
KeyCode::Char(' ') => {
    if let Some(ref player) = app.player {
        if app.player_state.is_playing {
            player.pause();
        } else {
            player.play();
        }
    }
}

// Enter - play selected track (NOW plays locally!)
KeyCode::Enter => {
    if let (Some(ref player), Some(ref session)) = (app.player, app.session) {
        let track = &tracks[app.selected_index];
        let spotify_id = SpotifyId::from_uri(&track.uri)?;
        player.load_track(spotify_id, 0);
        player.play();
    }
}

// n/p - next/previous
KeyCode::Char('n') => { /* player.next() */ }
KeyCode::Char('p') => { /* player.prev() */ }
```

**Tasks:**
- [ ] Update Enter handler to load tracks locally
- [ ] Update Space to control local player
- [ ] Implement next/previous track
- [ ] Add seek with left/right arrows

#### 2.3 Device Selector → Now Playing
**File:** `src/ui/player_bar.rs`

**Add to player bar:**
```rust
// Show "Joshify on hostname" badge
if app.is_local_playback {
    Paragraph::new("🔊 This Device")
        .style(Style::default().fg(Color::Green))
}
```

**Tasks:**
- [ ] Add visual indicator for local playback
- [ ] Show audio quality (160/320 kbps)
- [ ] Show backend type (rodio/ALSA/etc.)

---

### Phase 3: Advanced Features (Week 3)

#### 3.1 Audio Quality Settings
**File:** `src/config.rs`

```toml
[audio]
bitrate = 320  # 96, 160, 320 kbps
normalization = true
normalization_threshold = -3.0  # dB
gain_type = "album"  # or "track"
backend = "rodio"  # rodio, alsa, pulse, portaudio
```

**Tasks:**
- [ ] Add audio config options
- [ ] Implement bitrate selection
- [ ] Add volume normalization (ReplayGain)
- [ ] Backend selection CLI flag

#### 3.2 Cache Management
**File:** `src/cache.rs` (NEW)

```rust
use librespot_cache::{Cache, CacheConfig};

pub struct JoshifyCache {
    cache: Cache,
}

impl JoshifyCache {
    pub fn new(cache_dir: &Path) -> Result<Self>;
    pub fn configure(&self, size_limit_mb: u32);
}
```

**Tasks:**
- [ ] Create cache module
- [ ] Store audio data locally (faster replay)
- [ ] Store credentials securely
- [ ] Add cache size limit config
- [ ] Add cache clear command

#### 3.3 Queue Management
**File:** `src/queue.rs` (enhance existing)

```rust
// librespot provides built-in queue
use librespot_playback::queue::Queue;

// Integrate with existing queue UI
app.queue = player.get_queue();
```

**Tasks:**
- [ ] Replace custom queue with librespot queue
- [ ] Keep existing queue UI (Q key)
- [ ] Add "Add to Queue" from search/liked songs

---

### Phase 4: Testing & Polish (Week 4)

#### 4.1 Platform Testing

| Platform | Audio Backend | Status |
|----------|--------------|--------|
| Linux (ALSA) | `alsa` | ⬜ Test |
| Linux (Pulse) | `pulseaudio` | ⬜ Test |
| macOS | `rodio` (CoreAudio) | ⬜ Test |
| Windows | `rodio` (WASAPI) | ⬜ Test |
| Raspberry Pi | `alsa` | ⬜ Test |

#### 4.2 Integration Tests

```rust
#[tokio::test]
async fn test_local_playback() {
    let session = JoshifySession::connect_oauth("test_token").await?;
    let player = JoshifyPlayer::new(&session, "rodio");
    
    let track_id = SpotifyId::from_uri("spotify:track:4uLU6hMCjMI75M1A2tKUQC")?;
    player.load_track(track_id, 0);
    player.play();
    
    // Wait for playback to start
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Verify audio is playing (check event channel)
    // assert!(player.is_playing());
}
```

#### 4.3 Error Handling

**Common issues to handle:**
- [ ] Audio backend initialization failed → fallback to next backend
- [ ] No audio device found → show helpful error
- [ ] Session expired → auto-reconnect with OAuth refresh
- [ ] DRM-protected content → show "Track unavailable" message
- [ ] Network issues → retry with exponential backoff

---

## CLI Changes

### New Flags

```bash
# Start with local playback
joshify --playback

# Choose audio backend
joshify --playback --backend alsa

# Set audio quality
joshify --playback --bitrate 320

# Enable volume normalization
joshify --playback --normalize

# Set device name (shows in Spotify app)
joshify --playback --device-name "My Joshify Speaker"

# Cache directory
joshify --playback --cache-dir ~/.cache/joshify

# Disable Spotify Connect (local only)
joshify --playback --no-connect
```

### Config File (~/.config/joshify/config.toml)

```toml
[playback]
enabled = true
backend = "rodio"
bitrate = 320
normalize = true
device_name = "Joshify on {{hostname}}"

[cache]
enabled = true
dir = "~/.cache/joshify"
size_limit_mb = 1024

[audio]
volume_steps = 100
initial_volume = 75
gapless = true
```

---

## UI Changes

### Player Bar Enhancements

```
╭────────────────────────────────────────────────────────────╮
│ 🔊 Joshify on josh-mx    ▶ Song Name - Artist    02:15/04:30 │
│                        ██████████░░░░░░░░  160kbps 🔊 75%   │
╰────────────────────────────────────────────────────────────╘
```

**New indicators:**
- 🔊 = Local playback active
- 160/320 kbps = Audio quality
- 🔊 75% = Volume level
- Gapless indicator (crossfade icon)

### Help Text Updates

```
Playback (Local):
  Enter - Play track locally
  Space - Play/pause
  n/p - Next/previous track
  ←/→ - Seek ±10s
  +/- - Volume up/down
  0-9 - Set volume (0=0%, 9=90%)
  M - Mute/unmute
  
Spotify Connect:
  d - Select playback device
  (Joshify appears as device in other Spotify apps)
```

---

## Migration Path

### For Existing Users

**Current joshify users (remote control only):**
```bash
# Old behavior still works by default
joshify

# Opt-in to local playback
joshify --playback
```

**Config migration:**
- Existing OAuth tokens work with librespot
- Credentials stored in same location
- No re-authentication needed

---

## Code Size Estimate

| Module | Lines | Description |
|--------|-------|-------------|
| `src/session.rs` | ~300 | Session management |
| `src/player.rs` | ~500 | Audio player wrapper |
| `src/connect.rs` | ~400 | Spotify Connect |
| `src/cache.rs` | ~200 | Caching layer |
| `src/main.rs` | +800 | Integration (modified) |
| `src/ui/*.rs` | +300 | UI updates |
| `tests/` | +500 | Integration tests |
| **Total** | **~3000** | New/modified code |

---

## Risks & Mitigations

### Risk 1: Audio Backend Issues
**Problem:** Different systems need different backends

**Mitigation:**
- Default to rodio (cross-platform)
- Provide fallback chain: rodio → alsa → pulse → pipe
- Clear error messages with setup instructions

### Risk 2: Spotify API Changes
**Problem:** librespot breaks with Spotify protocol changes

**Mitigation:**
- Pin librespot version in Cargo.toml
- Monitor librespot releases
- Have fallback to Web API remote control mode

### Risk 3: Performance on Low-End Devices
**Problem:** Audio decoding CPU-intensive

**Mitigation:**
- Offer lower bitrate options (96 kbps)
- Use hardware decoding where available
- Profile and optimize hot paths

### Risk 4: Legal/ToS Concerns
**Problem:** Spotify may not approve of librespot

**Mitigation:**
- Clear disclaimer in README
- Premium-only (no free tier support)
- No circumvention of DRM (librespot respects it)

---

## Success Criteria

### Functional Requirements
- [ ] Plays audio locally through system speakers
- [ ] Appears as Spotify Connect device in other apps
- [ ] Can be controlled from phone/desktop app
- [ ] All existing TUI features work (search, playlists, liked songs)
- [ ] Volume control works
- [ ] Seek works
- [ ] Queue works

### Quality Requirements
- [ ] Audio quality matches official app (320 kbps)
- [ ] Gapless playback between tracks
- [ ] CPU usage <5% during playback (modern CPU)
- [ ] Memory usage <200MB
- [ ] Starts playing <2 seconds after pressing Enter
- [ ] No crashes during 1-hour playback session

### Platform Support
- [ ] Linux (ALSA/PulseAudio)
- [ ] macOS (CoreAudio via rodio)
- [ ] Windows (WASAPI via rodio)
- [ ] Raspberry Pi 3+ (ALSA)

---

## Comparison: Remote vs Local

| Feature | Remote Control (Current) | Local Playback (New) |
|---------|-------------------------|----------------------|
| **Audio Output** | Phone/PC speakers | Your machine's speakers |
| **Network** | Requires internet | Works offline (cached) |
| **Spotify Connect** | Controller only | Full receiver + controller |
| **Audio Quality** | Depends on other device | User-configurable (96-320 kbps) |
| **Latency** | ~500ms (API polling) | ~50ms (direct playback) |
| **Volume Control** | Via API (device-dependent) | Direct system control |
| **Gapless** | No | Yes |
| **Code Complexity** | Low | High |
| **Dependencies** | Minimal | Audio libs, librespot |
| **Lines of Code** | ~1200 | ~4000+ |

---

## Decision: Should You Build This?

### ✅ Build Local Playback If:
- You want joshify to be a **full Spotify client** (not just remote)
- You're comfortable maintaining audio dependencies
- You want to run joshify on a Raspberry Pi as a Spotify Connect speaker
- You want gapless playback and audio quality control
- You have 2-4 weeks for focused development

### ❌ Keep Remote Control If:
- You're happy controlling Spotify from your TUI
- You don't want to maintain audio backend dependencies
- You prefer minimal codebase (~1200 lines)
- You always have your phone/PC nearby for audio output
- You want to focus on TUI features instead

---

## Hybrid Approach (USER'S CHOICE) ✅

**Local playback by default, remote control as fallback:**

```rust
// Default: Local playback (plays on THIS machine)
joshify

// Fallback: Remote control (if local fails or user prefers)
joshify --remote-only

// In TUI: Press 'd' to select device (including "This Device")
// Can switch between local and remote on the fly
```

**Benefits:**
- ✅ Plays locally by default (what user wants)
- ✅ Can still control other devices (phone, desktop, speakers)
- ✅ Remote control as fallback if local fails
- ✅ Best of both worlds

---

## Next Steps

1. **Read librespot examples:** https://github.com/librespot-org/librespot/tree/dev/examples
2. **Test librespot CLI:** `cargo install librespot && librespot --name "Test"`
3. **Decide:** Full local, hybrid, or keep remote-only
4. **If proceeding:** Start with Phase 1.1 (Session Management)

---

## Questions to Consider

1. **Audio backend preference?**
   - Rodio (cross-platform, recommended)
   - ALSA (Linux only, lower latency)
   - PulseAudio (Linux desktop)
   - PortAudio (cross-platform C library)

2. **Default audio quality?**
   - 160 kbps (balanced)
   - 320 kbps (best quality)
   - User configurable

3. **Spotify Connect discovery?**
   - Enable by default (appears in Spotify app)
   - Or disable with `--no-connect` flag

4. **Cache strategy?**
   - Cache audio (faster replay, uses disk space)
   - Or stream only (no cache, slower replay)

5. **Hybrid mode?**
   - Default to remote, `--playback` for local
   - Or always use local when available

---

**Let me know which approach you want, and I'll start implementation!**
