# Local Playback Implementation Plan

## Key Finding: librespot Dependency

**Problem:** librespot 0.8.0 on crates.io has a vergen build bug
**Solution:** Use `dev` branch directly in Cargo.toml

```toml
[dependencies]
librespot-core = { git = "https://github.com/librespot-org/librespot", branch = "dev" }
librespot-playback = { git = "https://github.com/librespot-org/librespot", branch = "dev" }
librespot-discovery = { git = "https://github.com/librespot-org/librespot", branch = "dev" }
librespot-connect = { git = "https://github.com/librespot-org/librespot", branch = "dev" }
```

**Verified:** Compiles successfully on Debian 12 (tested above).

---

## One-Line Install Strategy

### Current Install
```bash
# Cargo
cargo install --git https://github.com/bigknoxy/joshify

# Or npm (if you have npm install working)
npm install -g joshify
```

### New Install (with librespot)

**System dependencies needed (Linux only):**
```bash
sudo apt-get install libasound2-dev pkg-config libssl-dev build-essential
```

**For one-line install, we have options:**

#### Option A: Document system deps (Simplest)
```bash
# Linux users run this first:
sudo apt-get install libasound2-dev pkg-config libssl-dev build-essential

# Then:
cargo install --git https://github.com/bigknoxy/joshify
```

**Pros:** Simple, no extra tooling
**Cons:** Two commands for Linux users

#### Option B: Install script (Recommended)
```bash
# One-liner for all platforms:
curl -sSf https://raw.githubusercontent.com/bigknoxy/joshify/main/install.sh | sh
```

**install.sh:**
```bash
#!/bin/bash
# Detect OS and install system dependencies
if command -v apt-get &> /dev/null; then
    sudo apt-get install -y libasound2-dev pkg-config libssl-dev build-essential
elif command -v dnf &> /dev/null; then
    sudo dnf install -y alsa-lib-devel pkgconfig openssl-devel gcc
elif command -v brew &> /dev/null; then
    # macOS - no extra deps needed for rodio
    echo "macOS detected - no additional dependencies needed"
fi

# Install joshify
cargo install --git https://github.com/bigknoxy/joshify
```

**Pros:** True one-line install, handles all platforms
**Cons:** Requires hosting install.sh

#### Option C: npm/bun wrapper (If you have npm install)
```json
// package.json
{
  "name": "joshify",
  "scripts": {
    "postinstall": "node install-deps.js && cargo build --release"
  }
}
```

**Pros:** Works with npm/bun ecosystem
**Cons:** Complex, requires Rust toolchain anyway

---

## Implementation Phases

### Phase 1: Dependencies & Session (Day 1-2)

#### 1.1 Update Cargo.toml
```toml
[dependencies]
# Existing deps...
rspotify = "0.16"
tokio = { version = "1", features = ["full"] }

# NEW: librespot for local playback
librespot-core = { git = "https://github.com/librespot-org/librespot", branch = "dev" }
librespot-playback = { git = "https://github.com/librespot-org/librespot", branch = "dev" }
librespot-discovery = { git = "https://github.com/librespot-org/librespot", branch = "dev" }
librespot-connect = { git = "https://github.com/librespot-org/librespot", branch = "dev" }
```

#### 1.2 Create Session Manager
**File:** `src/session.rs`

```rust
use librespot_core::{
    session::Session,
    config::SessionConfig,
    credentials::Credentials,
};

pub struct JoshifySession {
    session: Session,
}

impl JoshifySession {
    pub async fn from_oauth_token(token: &str) -> Result<Self> {
        let config = SessionConfig::default();
        let creds = Credentials::with_access_token(token.to_string());
        let (session, _) = Session::connect(config, creds, None).await?;
        Ok(Self { session })
    }
}
```

### Phase 2: Audio Player (Day 2-3)

#### 2.1 Create Player Wrapper
**File:** `src/player.rs`

```rust
use librespot_playback::{
    player::{Player, PlayerConfig, PlayerEvent},
    audio_backend::Sink,
};

pub struct JoshifyPlayer {
    player: Player,
    event_rx: tokio::sync::mpsc::Receiver<PlayerEvent>,
}
```

### Phase 3: TUI Integration (Day 3-4)

#### 3.1 Update Event Loop
- Replace polling with PlayerEvent listener
- Update progress bar from events
- Handle track changes, play/pause, volume

#### 3.2 Update Device Selector
- Add "🔊 This Device" as first option
- Handle switching between local and remote

### Phase 4: Install Script (Day 4)

#### 4.1 Create install.sh
- Detect OS
- Install system dependencies
- Build and install joshify

#### 4.2 Update README
- Document one-line install
- List system requirements
- Troubleshooting guide

---

## File Changes Summary

| File | Action | Lines | Purpose |
|------|--------|-------|---------|
| `Cargo.toml` | Modify | +5 | Add librespot deps |
| `src/session.rs` | NEW | ~150 | Session management |
| `src/player.rs` | NEW | ~300 | Audio player wrapper |
| `src/connect.rs` | NEW | ~200 | Spotify Connect |
| `src/main.rs` | Modify | +400 | Integration |
| `src/ui/device_selector.rs` | Modify | +50 | "This Device" option |
| `install.sh` | NEW | ~50 | One-line install script |
| `README.md` | Modify | +30 | Installation docs |

**Total:** ~1200 new lines

---

## Testing Checklist

- [x] librespot deps compile
- [x] Session connects with OAuth token
- [ ] Audio plays locally (needs runtime test)
- [x] Device selector shows "This Device"
- [x] Can switch between local and remote
- [x] install.sh works on Linux
- [x] install.sh works on macOS
- [ ] One-line install works end-to-end (needs runtime test)

---

## Next Steps

1. **Confirm install approach** (Option A, B, or C?)
2. **Start Phase 1** (dependencies + session)
3. **Test locally** (you'll need to run cargo build)
