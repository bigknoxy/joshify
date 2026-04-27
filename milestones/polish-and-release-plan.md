# Joshify Polish & Release Plan
## April 2026 — From Functional to Production-Ready

**Status**: Planning Complete  
**Branch**: `feature/polish-milestone-1` (will be created)  
**Total Estimated Effort**: 3-4 days CC (1 week human team)

---

## Executive Summary

This plan transforms Joshify from a functional proof-of-concept into a polished, release-ready TUI application that can compete with spotify-player (6.6k stars) and ncspot. The plan is divided into 2 milestones:

**Milestone 1**: Polish Core UX — The "Wow Factor" (makes users want to use this)  
**Milestone 2**: Release Readiness — Production features (makes this reliable enough to ship)

Based on competitive analysis of spotify-player (April 2026), premium TUI music apps require:
1. **Visual polish** — Audio visualization, smooth transitions, beautiful theming
2. **Professional features** — Desktop notifications, media keys, device switching
3. **Reliability** — Configuration, logging, error recovery, documentation
4. **Power user features** — CLI interface, fuzzy search, extensive keyboard shortcuts

---

## Competitive Analysis (April 2026)

| Feature | spotify-player (6.6k⭐) | ncspot | Joshify Current | Target M1 | Target M2 |
|---------|------------------------|--------|-----------------|-----------|-----------|
| **Core Playback** | ✅ | ✅ | ✅ | ✅ | ✅ |
| Search | ✅ | ✅ | ✅ | ✅ | ✅ |
| Queue | ✅ | ✅ | ✅ | ✅ | ✅ |
| Album Art | ✅ (multiple backends) | ✅ | ✅ (kitty/iTerm2) | ✅ | ✅ |
| **Navigation** | ✅ (pages) | ✅ | ✅ (NEW drill-down) | ✅ | ✅ |
| Audio Visualization | ✅ (64-band FFT) | ❌ | ❌ | ✅ | ✅ |
| Media Control (MPRIS) | ✅ | ✅ | ❌ | ✅ | ✅ |
| Desktop Notifications | ✅ | ❌ | ❌ | ✅ | ✅ |
| Configuration File | ✅ | ✅ | ❌ | ✅ | ✅ |
| Fuzzy Search | ✅ | ✅ | ❌ | ✅ | ✅ |
| CLI Commands | ✅ | ❌ | ❌ | ❌ | ✅ |
| Lyrics | ✅ | ❌ | ❌ | ❌ | ✅ |
| Multiple Themes | ✅ | ✅ | ❌ | ❌ | ✅ |
| Daemon Mode | ✅ | ❌ | ❌ | ❌ | ✅ |
| Logs | ✅ | ❌ | ❌ | ❌ | ✅ |

**Key Insight**: spotify-player's "wow factor" comes from audio visualization + media control + polish. Joshify needs these to compete.

---

## Milestone 1: Polish Core UX — The "Wow Factor"

**Goal**: Make users say "this is beautiful" within 30 seconds of using it.

**Success Metrics**:
- User can enable audio visualization and it looks professional
- Media keys work out of the box (Play/Pause, Next, Prev)
- Desktop notifications show on track change
- Configuration file allows customization without recompiling
- Fuzzy search makes finding music faster

### 1.1 Audio Visualization (HIGH IMPACT — THE WOW FEATURE)

**What**: Real-time frequency spectrum visualization in the player bar

**Why**: This is the #1 differentiator for premium TUI music apps. spotify-player has it. ncspot doesn't. Users notice it immediately.

**Implementation**:
```rust
// New file: src/ui/visualizer.rs
pub struct AudioVisualizer {
    // 64 log-scale frequency bands (bass → treble)
    bands: [f32; 64],
    // Smoothing factor (0.0-1.0, higher = smoother)
    smoothing: f32,
    // Color gradient (Catppuccin theme)
    colors: Vec<Color>,
}
```

**Technical Details**:
- Hook into librespot's audio pipeline (local playback only)
- Use FFT on raw PCM samples (16-bit stereo @ 44.1kHz)
- 64 frequency bands, log-scale distribution
- Render as block characters (█ ▉ ▊ ▋ ▌ ▍ ▎ ▏)
- Height: 3-4 rows in player bar
- Updates: 30fps max (performance)
- Only show when actively streaming locally (not on Connect devices)

**Files**:
- `src/ui/visualizer.rs` (NEW) — FFT visualization logic
- `src/player/` — Hook into audio pipeline
- `src/ui/player_bar.rs` — Integrate visualizer
- `tests/visualizer_tests.rs` — Unit tests for FFT math

**Test Plan**:
- Test FFT produces expected frequency bins
- Test smoothing reduces jitter
- Test falls back gracefully on Connect playback

**Effort**: 1 day CC (~3 days human)

---

### 1.2 Media Control (MPRIS) — Essential UX

**What**: System media keys (Play/Pause, Next, Prev) work globally

**Why**: Users expect this. It's jarring when it doesn't work. spotify-player and ncspot both have it.

**Implementation**:
```rust
// New file: src/media_control.rs
pub struct MediaControlService {
    // MPRIS DBus interface on Linux
    // Media key hooks on macOS
    // Windows media transport controls
}
```

**Technical Details**:
- Linux: MPRIS DBus interface (org.mpris.MediaPlayer2)
- macOS: MediaPlayer framework + global hotkeys
- Windows: IAudioSessionManager + media transport controls
- Graceful fallback if unsupported
- Toggle in config: `enable_media_control: true`

**Files**:
- `src/media_control.rs` (NEW) — Platform abstraction
- Platform-specific modules:
  - `src/media_control/linux.rs` (MPRIS)
  - `src/media_control/macos.rs` (MediaPlayer)
  - `src/media_control/windows.rs` (WASAPI)
- `src/main.rs` — Wire into event loop

**Test Plan**:
- Unit tests for DBus interface (mock)
- Integration tests (manual on each platform)

**Effort**: 0.5 day CC (~1.5 days human)

---

### 1.3 Desktop Notifications

**What**: Native OS notifications on track change

**Why**: Users want to know what's playing without switching to the terminal. Professional apps have this.

**Implementation**:
```rust
// New file: src/notifications.rs
pub struct NotificationService {
    // notify-rust on Linux
    // macOS notification center
    // Windows toast notifications
}
```

**Technical Details**:
- Notify on track change (not on every seek/pause)
- Include: Album art thumbnail, track title, artist, album
- Rate limit: max 1 notification per 5 seconds
- Configurable: `enable_notifications: true`
- Silence during active window focus (optional)

**Files**:
- `src/notifications.rs` (NEW)
- Platform-specific notification backends

**Test Plan**:
- Mock notification service for tests
- Integration tests for notification triggers

**Effort**: 0.5 day CC (~1 day human)

---

### 1.4 Configuration File

**What**: User-customizable config file (~/.config/joshify/config.toml)

**Why**: Power users expect customization. spotify-player has extensive config. This is table stakes.

**Implementation**:
```toml
# ~/.config/joshify/config.toml
[audio]
visualization = true
visualization_bands = 64
visualization_smoothing = 0.7

[notifications]
enabled = true
show_album_art = true

[media_control]
enabled = true

[keybindings]
# Override defaults
play_pause = "Space"
next = "n"
previous = "p"

[ui]
theme = "catppuccin_mocha"
show_visualizer = true
compact_mode = false
```

**Files**:
- `src/config.rs` (NEW) — Config loading/saving
- `src/config/default.rs` — Default config values
- `Cargo.toml` — Add `config` + `serde` + `toml` deps

**Test Plan**:
- Test config parsing (valid/invalid)
- Test defaults applied when file missing
- Test hot-reload (optional for M1)

**Effort**: 0.5 day CC (~1 day human)

---

### 1.5 Fuzzy Search

**What**: Typing "ts 1989" finds Taylor Swift's 1989 album instantly

**Why**: Current search is literal. Fuzzy matching is expected in modern TUIs.

**Implementation**:
```rust
// Update: src/ui/overlays.rs search functionality
pub fn fuzzy_match(query: &str, target: &str) -> f32 {
    // Use nucleo or similar fuzzy matcher
    // Score based on character matches + positions
}
```

**Technical Details**:
- Use `nucleo` crate (Rust's fastest fuzzy matcher)
- Real-time results as you type
- Highlight matching characters
- Sort by relevance score
- Handle typos gracefully ("ts" → "Taylor Swift")

**Files**:
- `Cargo.toml` — Add `nucleo` dependency
- `src/ui/overlays.rs` — Update search overlay
- `src/api/search.rs` — Client-side fuzzy ranking

**Test Plan**:
- Test fuzzy scoring accuracy
- Test ranking produces expected results
- Test performance with 1000+ items

**Effort**: 0.5 day CC (~1 day human)

---

### Milestone 1 Summary

| Feature | Files Created/Modified | Tests | Effort (CC) |
|---------|------------------------|-------|-------------|
| Audio Visualization | +3, ~2 | 5 | 1 day |
| Media Control | +4, ~1 | 3 | 0.5 day |
| Desktop Notifications | +2, ~1 | 3 | 0.5 day |
| Configuration File | +2, ~3 | 5 | 0.5 day |
| Fuzzy Search | ~2 | 4 | 0.5 day |
| **TOTAL** | **+13 files** | **20 tests** | **3 days** |

**Milestone 1 Deliverables**:
- [ ] Audio visualization renders smoothly (30fps)
- [ ] Media keys work on all platforms
- [ ] Notifications show on track change
- [ ] Config file loads and applies
- [ ] Fuzzy search replaces literal search
- [ ] All 223+ tests passing
- [ ] Release build < 10MB
- [ ] README updated with new features

---

## Milestone 2: Release Readiness — Production Quality

**Goal**: Make Joshify reliable enough that users can depend on it as their daily driver.

**Success Metrics**:
- Users can report bugs with logs
- Power users can script with CLI
- Multiple themes available
- Lyrics displayed for current track
- Can run as daemon (optional but impressive)

### 2.1 Logging & Diagnostics

**What**: Structured logging to file (~/.cache/joshify/joshify.log)

**Why**: Essential for debugging user issues. spotify-player has this. We need it for support.

**Implementation**:
```rust
// Update: Add tracing subscriber
use tracing_subscriber::{fmt, EnvFilter};

// Log to file + stderr (configurable level)
// Rotate logs at 10MB
```

**Files**:
- `src/logging.rs` (NEW) — Logging setup
- `Cargo.toml` — Add `tracing-appender`

**Test Plan**:
- Test log file rotation
- Test log levels (DEBUG, INFO, WARN, ERROR)

**Effort**: 0.5 day CC

---

### 2.2 CLI Commands

**What**: Non-interactive commands: `joshify play`, `joshify pause`, `joshify next`

**Why**: Power users want to script their music. spotify-player has extensive CLI.

**Implementation**:
```bash
joshify --help
joshify play --uri "spotify:track:..."
joshify pause
joshify next
joshify previous
joshify volume --set 50
joshify status --format json
```

**Files**:
- `src/cli.rs` (NEW) — CLI argument parsing
- `src/daemon.rs` (NEW) — IPC communication

**Test Plan**:
- Test each CLI command
- Test JSON output format

**Effort**: 1 day CC

---

### 2.3 Lyrics Display

**What**: Show synced lyrics for current track

**Why**: Nice-to-have that spotify-player has. Users love it.

**Implementation**:
- Use `lrclib` API (free, no auth required)
- Display in popup overlay
- Highlight current line
- Scroll automatically

**Files**:
- `src/api/lyrics.rs` (NEW)
- `src/ui/lyrics.rs` (NEW)

**Test Plan**:
- Test lyrics fetching
- Test sync accuracy

**Effort**: 1 day CC

---

### 2.4 Theme System

**What**: Multiple color themes beyond Catppuccin Mocha

**Why**: Personalization is important. Users want their terminal to match their setup.

**Themes to include**:
- Catppuccin Mocha (current)
- Catppuccin Latte
- Gruvbox Dark
- Gruvbox Light
- Nord
- Tokyo Night
- Dracula

**Files**:
- `src/ui/themes.rs` (NEW) — Theme definitions
- `src/ui/theme.rs` — Refactor to use theme system

**Test Plan**:
- Test each theme renders correctly
- Test theme switching

**Effort**: 0.5 day CC

---

### 2.5 Daemon Mode (Optional but Impressive)

**What**: Run `joshify -d` as background daemon, control via CLI

**Why**: Power user feature. Keeps music playing without terminal open.

**Implementation**:
- Unix socket / named pipe for IPC
- CLI sends commands to daemon
- Graceful shutdown handling

**Files**:
- `src/daemon.rs` (NEW)
- `src/ipc.rs` (NEW)

**Test Plan**:
- Test daemon start/stop
- Test IPC communication

**Effort**: 1 day CC

---

### Milestone 2 Summary

| Feature | Files Created/Modified | Tests | Effort (CC) |
|---------|------------------------|-------|-------------|
| Logging & Diagnostics | +2, ~1 | 3 | 0.5 day |
| CLI Commands | +4, ~2 | 8 | 1 day |
| Lyrics Display | +2, ~1 | 4 | 1 day |
| Theme System | +2, ~2 | 7 | 0.5 day |
| Daemon Mode | +2, ~1 | 5 | 1 day |
| **TOTAL** | **+12 files** | **27 tests** | **4 days** |

**Milestone 2 Deliverables**:
- [ ] Logs written to ~/.cache/joshify/
- [ ] CLI commands work (`joshify play`, `joshify status`)
- [ ] Lyrics displayed in popup
- [ ] 7 themes available
- [ ] Daemon mode runs (bonus)
- [ ] All 250+ tests passing
- [ ] Complete documentation
- [ ] Installation script works on macOS/Linux

---

## Implementation Order

### Phase 1: Foundation (Day 1)
1. Configuration file system (needed by all other features)
2. Logging infrastructure (needed for debugging)

### Phase 2: The Wow (Days 2-3)
3. Audio visualization (the headline feature)
4. Media control (essential UX)
5. Desktop notifications (polish)

### Phase 3: Power User (Days 4-5)
6. Fuzzy search (usability)
7. Theme system (personalization)
8. Lyrics display (delight)

### Phase 4: Professional (Days 6-7)
9. CLI commands (power users)
10. Daemon mode (bonus)
11. Final polish & documentation

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Audio visualization too CPU-heavy | Medium | High | Make optional, reduce bands to 32, lower FPS |
| Platform-specific media control bugs | High | Medium | Graceful fallback, thorough testing per platform |
| Config file format changes | Low | Medium | Use TOML (stable), version the config |
| Dependency bloat | Medium | Medium | Feature flags, optional dependencies |
| MPRIS/DBus complexity on Linux | Medium | Medium | Use `mpris` crate, test on multiple DEs |

---

## Success Criteria

**Milestone 1 Complete When**:
- [ ] User runs Joshify and sees audio visualization within 10 seconds
- [ ] Media keys work without configuration
- [ ] Notifications appear on track change
- [ ] Config file can be edited without recompiling
- [ ] Search finds "ts 1989" → Taylor Swift 1989 album
- [ ] All tests pass
- [ ] README documents all features

**Milestone 2 Complete When**:
- [ ] User can report a bug with logs attached
- [ ] CLI commands work: `joshify play`, `joshify status`
- [ ] Lyrics popup shows for tracks with lyrics
- [ ] User can switch to any of 7 themes
- [ ] All tests pass
- [ ] Installation works on fresh macOS/Linux
- [ ] Ready for v1.0.0 release

---

## Post-Release Ideas (M3+)

- [ ] Plugin system
- [ ] Playlist editing
- [ ] Radio / recommendations
- [ ] Podcast support
- [ ] Collaborative playlists
- [ ] Offline mode with caching
- [ ] Mobile companion app
- [ ] Web UI (wasm)

---

## Decision Log

**Decision 1**: Audio visualization is priority #1  
**Rationale**: This is the single biggest differentiator vs ncspot. It's the "wow" feature that makes users choose Joshify.

**Decision 2**: Configuration file before CLI  
**Rationale**: Config needed by all other features. CLI depends on daemon which is M2.

**Decision 3**: Media control separate from daemon  
**Rationale**: Media control (MPRIS) works with interactive app. Daemon is bonus.

**Decision 4**: Fuzzy search in M1, not M2  
**Rationale**: Core UX improvement. Makes search usable. Quick win.

**Decision 5**: Daemon mode optional  
**Rationale**: Complex, niche feature. Ship without it if time-constrained.

---

*Plan created: 2026-04-26*  
*Next step: Create feature branch and start Milestone 1*
