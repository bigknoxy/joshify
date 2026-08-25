## [0.7.6](https://github.com/bigknoxy/joshify/compare/v0.7.5...v0.7.6) (2026-08-25)

### Bug Fixes

- **`--version` was unreachable, which silently broke the installer** ([#54](https://github.com/bigknoxy/joshify/issues/54)): `--version` was only handled in `src/cli.rs`, which is not reachable from the binary ([#48](https://github.com/bigknoxy/joshify/issues/48)), so the flag fell through and started the TUI instead. `install.sh` uses it in three places, and all three were broken: the prebuilt binary failed its smoke test on every platform and every install silently fell back to a source build, the idempotency check never detected an existing install, and after a source build the final probe returned nothing and the installer reported failure for an install that had succeeded. The binary-first installer added in 0.7.3 had therefore never once taken its fast path.
- **Installer no longer hides why a prebuilt binary was rejected**: a failed smoke test prints the binary's actual output, a binary that runs but does not report `Joshify <version>` is rejected rather than installed, and an unreadable version at the end is a warning instead of a fatal error.

## [0.7.5](https://github.com/bigknoxy/joshify/compare/v0.7.4...v0.7.5) (2026-08-25)

### Features

- **Headless setup** ([#47](https://github.com/bigknoxy/joshify/issues/47)): new `joshify --setup` runs credential setup and the OAuth flow and then exits, without ever initializing the TUI. The authorization URL is now always printed, so a machine with no browser — SSH, a container, WSL — can complete authorization by opening the URL elsewhere instead of sitting at "Waiting for authorization…" indefinitely. Under WSL the URL works from a Windows browser, since WSL2 shares localhost.
- **Documented non-interactive configuration**: `--help` now covers `--setup`, `SPOTIFY_TOKEN_EXPIRES_AT` and `SPOTIFY_REDIRECT_URI`, and states that the credential environment variables only skip the browser when set together. The README documents the `config.json` and `credentials.json` schemas, notes that `config.toml` holds no credentials, and lists the WSL requirements.

### Bug Fixes

- **Local playback no longer claims success when there is no audio device** ([#49](https://github.com/bigknoxy/joshify/issues/49)): `audio_backend::find()` only resolves a backend by name and succeeds on a machine with no working audio, so the app reported "Local playback active" and then played silence. librespot's rodio backend does not return an error in that case — it panics while *building* the sink, which on the player's audio thread kills the thread quietly. The audio device is now probed at startup behind a panic guard; when it cannot be opened the app falls back to remote playback and says why in the status bar, with a specific message when running as root (the usual cause under WSL).
- **No local player is constructed when audio is unavailable**: previously the playback mode was switched to remote but the player, session and event channel stayed installed, and Spotify Connect had already advertised joshify as a playback device that could only produce silence.
- **Root detection works on macOS**: it read `/proc/self/status`, which does not exist there.

### Technical Details

- The audio probe restores the caller's panic hook rather than reverting to the default, and only suppresses panics raised on the probing thread
- Probing happens before the TUI takes the screen, since ALSA writes diagnostics to stderr from C that no Rust hook can intercept
- Adds `libc` as a direct dependency for `geteuid()`

## [0.7.4](https://github.com/bigknoxy/joshify/compare/v0.7.3...v0.7.4) (2026-08-24)

### Bug Fixes

- **Unreadable first-run setup screen** ([#46](https://github.com/bigknoxy/joshify/issues/46)): the terminal was put into raw mode and the alternate screen, with mouse capture on and the cursor hidden, *before* the interactive credential prompts ran. Those prompts print with `println!` and read with `dialoguer`, so in raw mode `\n` stopped implying a carriage return and every line staircased across the screen, the hidden cursor meant you typed blind, mouse movement injected escape sequences into the input, and the whole thing was drawn on the alternate screen that was then cleared. Terminal initialization now happens after the auth block.
- **In-app settings key**: pressing `c` called the same interactive setup directly from the event loop while the TUI still owned the terminal, with the same result. It now runs inside a new `suspend_tui()` helper that disables mouse capture, restores the cursor, leaves the alternate screen, and re-enters the TUI afterwards. Failures to suspend are reported in the status bar instead of being discarded.

### Technical Details

- Two source-order tests guard both call sites; this ordering is easy to reintroduce while editing `run_with_args` and cannot be checked behaviourally without a real terminal

## [0.7.3](https://github.com/bigknoxy/joshify/compare/v0.7.2...v0.7.3) (2026-08-24)

### Features

- **Installer uses the prebuilt release binary**: `install.sh` previously only knew how to build from source, so a one-line install on Linux x86_64 or Apple Silicon downloaded the Rust toolchain, installed the full `-dev` package set, and compiled 648 crates — for a binary already published on the releases page. It now downloads and verifies the release asset for the platform, and builds from source only when there is no matching binary, the download cannot be verified, or the binary will not run.
- **Verified downloads**: releases now publish a `SHA256SUMS` file. The installer verifies the tarball against it and refuses to install on a mismatch. A release with no checksums falls back to a source build unless `JOSHIFY_ALLOW_UNVERIFIED=1`.
- **Idempotent installs**: re-running the installer when the target version is already present exits without doing any work (`JOSHIFY_FORCE=1` overrides). Installs are written atomically and replace an existing `joshify` in place instead of shadowing it on `PATH`.
- **New installer options**: `JOSHIFY_VERSION`, `JOSHIFY_INSTALL_DIR`, `JOSHIFY_FORCE`, `JOSHIFY_BUILD_FROM_SOURCE`, `JOSHIFY_ALLOW_UNVERIFIED`.

### Bug Fixes

- **Misleading non-interactive auth advice**: the installer implied `SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET`, and `SPOTIFY_ACCESS_TOKEN` each helped on their own. They only bypass the browser when all three are set together; with only the first two the app still opens a browser and blocks on the callback. The footer now says so and mentions `SPOTIFY_REFRESH_TOKEN` / `SPOTIFY_TOKEN_EXPIRES_AT`.
- **Uninstaller**: now clears every location the installer may have used (`~/.cargo/bin`, `~/.local/bin`, `JOSHIFY_INSTALL_DIR`) and is safe to run repeatedly.

### Technical Details

- The prebuilt binary needs only runtime libraries (`libasound`, `libssl`), not the `-dev` headers, compiler, or Rust toolchain
- Installer helper tests grew to 49 assertions, including `install_from_release` exercised end to end against a stubbed download covering verified install, idempotent re-run, checksum-mismatch refusal, and platform fallback

## [0.7.2](https://github.com/bigknoxy/joshify/compare/v0.7.1...v0.7.2) (2026-08-24)

### Bug Fixes

- **Broken demo image in README**: the Demo section embedded `assets/demo.gif`, a file that is never committed — the VHS pipeline has never successfully produced it. It now uses `screenshots/reference/demo.gif`, which is in the repo, and the duplicate embed of the same GIF below it is removed.
- **Visual Tests badge stuck on "failing"**: the `update-readme-assets` job hard-failed on every run because `download-artifact` demanded a demo-GIF artifact that `upload-artifact` never created (`if-no-files-found: warn`). The red badge reported a missing optional GIF, not a failing test. The download is now tolerant of a missing artifact and logs which case occurred.

### Continuous Integration

- Replaced the dead `update-docs` job, which sed-replaced badge patterns that no longer exist in the README, with a `verify-badges` job that checks the badges resolve and that the release badge has caught up to the new tag
- New `scripts/check-badges.sh` verifies every shields.io badge and every local file the README references; runs on every pull request and at release time
- Security audit now runs `cargo audit` directly instead of `rustsec/audit-check`, which still targets the deprecated Node 20 and whose v2 failed the check on advisories despite the job being advisory-only ([#40](https://github.com/bigknoxy/joshify/issues/40) tracks the advisories themselves)
- Bumped every action off the deprecated Node 20 runtime: `checkout` v4→v7, `upload-artifact` v4→v7, `download-artifact` v4→v8, `create-pull-request` v6→v8, `action-gh-release` v2→v3, plus the Pages actions

## [0.7.1](https://github.com/bigknoxy/joshify/compare/v0.7.0...v0.7.1) (2026-08-24)

### Bug Fixes

- **Installer on noexec /tmp**: `install.sh` now probes whether binaries can actually be executed from the temp directory and relocates to `/tmp` or `~/.cache/joshify/tmp` when they cannot. Fixes `rustup-init` failing with "Cannot execute ... (Is /tmp mounted noexec?)" on WSL2 and hardened Linux. A pre-set, working `TMPDIR` is respected. ([#37](https://github.com/bigknoxy/joshify/issues/37))
- **Installer without a TTY**: `sudo` credentials are now primed up front through `/dev/tty` (which works under `curl | bash`, where stdin is the script) instead of failing after the slow Rust install. With no TTY and no passwordless `sudo`, the dependency step is skipped with the exact packages to install by hand rather than aborting the run.
- **Installer package manager selection**: the native package manager now wins over linuxbrew on Linux, so hosts with Homebrew installed still get `libasound2-dev`, `libssl-dev`, and `build-essential`.

### Technical Details

- New `JOSHIFY_SKIP_DEPS=1` env var to skip the system dependency step entirely; `apt` runs under `DEBIAN_FRONTEND=noninteractive`
- Installer temp directory is now cleaned up via an `EXIT` trap instead of a trailing `rm` that a mid-script failure would skip; the clone is shallow
- Clear error if `cargo` is still missing after the rustup install
- New `tests/install_sh_test.sh` (22 assertions) and a CI "Installer Script" job running `shellcheck` plus those tests, wired into the `ci-success` gate

## [0.5.0](https://github.com/bigknoxy/joshify/compare/v0.4.0...v0.5.0) (2026-05-04)

### Bug Fixes

- **Playback Queue Auto-Advance**: Fixed issue where selecting a track in a playlist would cause it to play twice before continuing. Now correctly advances to the next track after playback ends.
- **Remote Mode Context Playback**: Fixed duplicate `playback_next()` call that could cause skipped tracks in Remote mode. Spotify handles auto-advance within context.
- **Mouse Click Handler**: Fixed missing `set_context_position()` call when double-clicking tracks, ensuring queue advancement works correctly for mouse interactions.

### Technical Details

- Added explicit `advance()` call after starting playback to keep queue position in sync
- Improved debug logging throughout playback flow for easier troubleshooting
- Fixed position tracking semantics: `context_position` now correctly represents "next track to be returned by advance()"

## [0.4.0](https://github.com/bigknoxy/joshify/compare/v0.3.0...v0.4.0) (2026-04-27)

### Features

- **Daemon Mode**: Background service with Unix socket IPC (`joshify daemon`, `joshify daemon-send`). JSON protocol for commands. 14 tests.
- **CLI Commands**: Full command-line interface for scripting. Commands: play, pause, next, previous, stop, status, volume, seek, search, queue-add. Output formats: text, json, minimal. 24 tests.
- **Lyrics Display**: Synced lyrics via LRCLIB API. Real-time lyric display with timestamp parsing. 10 tests.
- **Theme System**: 7 built-in themes (Catppuccin Mocha/Latte, Gruvbox Dark/Light, Nord, Tokyo Night, Dracula). Dynamic theme switching. Theme trait for extensibility. 12 tests.
- **Structured Logging**: Tracing-based logging with file rotation (10MB max, 5 files). Log level filtering. 12 tests.
- **Documentation**: Updated README with all new features, CLI examples, configuration guide.

### Dependencies

- Added `tracing-appender = "0.2"` for log rotation
- Added `toml = "0.8"` for configuration files
- Added `dirs-next = "2"` for config directory detection
- Added `realfft = "3"` for FFT audio visualization
- Added `notify-rust = "4"` for Linux notifications (optional)

---

## [0.3.0](https://github.com/bigknoxy/joshify/compare/v0.2.0...v0.3.0) (2026-04-27)

### Features

- **Configuration System**: TOML-based configuration at `~/.config/joshify/config.toml`. Settings for audio, notifications, media control, UI, keybindings. Auto-created with defaults. 5 tests.
- **Audio Visualization**: Real-time FFT spectrum visualization. Configurable bands (32, 64, 128). Smoothing factor control. Works with local playback. 7 tests.
- **Media Control**: MPRIS integration for OS media key support. Platform abstraction for Linux/macOS/Windows. Commands: play, pause, next, previous, stop. 10 tests.
- **Desktop Notifications**: Native OS notifications on track change. Rate limiting (5s cooldown). Duplicate detection. Album art thumbnails when available. 17 tests.
- **Fuzzy Search**: Typo-tolerant search with relevance scoring. Custom implementation with consecutive match bonuses and gap penalties. 17 tests.
- **Test Suite Growth**: 280+ tests covering all new functionality.

### Dependencies

- Added `toml = "0.8"` for configuration parsing
- Added `dirs-next = "2"` for cross-platform directories
- Added `realfft = "3"` for FFT processing
- Added `num-complex = "0.4"` for complex numbers (FFT)

---

## [0.2.0](https://github.com/bigknoxy/joshify/compare/v0.1.0...v0.2.0) (2026-04-08)

### Breaking Changes

- Player bar height increased from 5 to 6 rows to accommodate the new layout
- Search Enter key now attempts device discovery before playback (previously silently failed)
- Debounce timing changed from "time since last search dispatch" to "time since last keystroke"

### Features

- **Search overhaul**: Fixed search returning no results by adding `Market::FromToken` parameter and capping API limit to 10 (Spotify's new maximum). Search now works correctly.
- **Search UX**: Tab key adds selected track to queue. Enter plays the track. Results display in overlay with proper truncation.
- **Cursor alignment**: Search cursor and text now align correctly using unicode display width instead of character count. Works with emoji and wide characters.
- **Now Playing redesign**: 4 interior rows — scrolling title (bold Mauve, marquee animation for long names), artist line (dim + badges + key hints), progress bar (Green Gauge widget + time labels), volume bar (visual indicator with percentage).
- **Scrolling title**: Long track names now scroll horizontally like a car radio (8 cols/sec, 2-second pause at start/end, resets on track change).
- **All key handlers non-blocking**: Play/pause, next/prev, volume, seek, shuffle, repeat, device transfer — all use `tokio::spawn` instead of blocking `.lock().await`.
- **Optimistic volume updates**: Volume keys update local state immediately, no 2-second delay before visual feedback.
- **Album art repositions on resize**: Art is re-processed with correct coordinates when the terminal is resized. Kitty images are explicitly deleted before redrawing.
- **Gorilla-with-headphones ASCII art**: Replaced the mauve face logo with a green gorilla wearing mauve headphones.

### Bug Fixes

- **Search cursor misalignment** (#1): Used `unicode-width` for display width calculation instead of `chars().count()`. Replaced emoji `🔍` with ASCII `/` in the search prompt prefix.
- **Search infinite re-search loop** (#2): Added `last_searched_query` field to prevent re-firing a search for a query that already returned results. Debounce now measures from last keystroke.
- **Search Enter not playing** (#3): Added device discovery (`available_devices()` + `transfer_playback()`) before `start_playback()`. Previously passed `device_id=None` which silently failed with 403.
- **Stale search results on fast typing** (#4): `insert_char`/`delete_char` now clear `pending_query` so stale results are discarded.
- **Channel only processing one message per loop** (#5): Changed `if let Ok` to `while let Ok` for `rx.try_recv()` to drain all pending messages.
- **TUI freezing on key presses** (#6): All 15 key handlers that called `client.lock().await` directly now use `tokio::spawn` for async execution.
- **Text overflow in overlays** (#7): Queue and Help overlays now use `bg.inner()` for content rendering and `truncate_from_start()` for text truncation. Matches the Search overlay pattern.
- **Album art ghost on resize** (#8): Kitty images now use protocol-level delete command (`\x1b_Ga=d`) and space-filling (not `\x1b[K` which erases to end of line, wiping adjacent content).
- **Progress bar spacing** (#9): Time labels now use separate layout columns with proper gaps instead of cramming everything on one line.
- **Volume bar misalignment** (#10): Standardized volume bar patterns to consistent 6-character widths.
- **`truncate()` panic on multi-byte characters** (#11): Replaced byte-slicing with `unicode_truncate()` in `main_view.rs`.
- **Spotify API limit** (#12): Spotify reduced max search results from 50 to 10. Updated all search calls.

### Dependencies

- Added `unicode-width = "0.2"` for display width calculations
- Added `unicode-truncate = "2"` for safe width-based truncation