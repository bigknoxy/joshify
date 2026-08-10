# Joshify Audit Backlog — 2026-08-08

Full audit of the repo: code review, feature review, build/install tooling, CI health, and docs drift. Each item is prioritized and has **testable acceptance criteria** so an agent with no prior context can implement it with TDD.

Priority guide: **P0** = crash/data-loss/blocking-install, **P1** = wrong behavior for advertised feature, **P2** = degradation/cleanup, **P3** = nice-to-have / documentation.

---

## P0-1: Volume controls panic in debug / set garbage volume in release (u16 overflow)

- **Severity**: P0 (panic + wrong volume)
- **Files**: `src/main.rs:2675`, `src/main.rs:2734`, `src/main.rs:2818`, `src/main.rs:2836`
- **Bug**: All four keyboard volume paths compute `app.player_state.volume as u16 * 65535 / 100`. `volume` is `u32` (0-100). The `as u16` cast makes the multiply happen in `u16`, and `volume * 65535` overflows `u16` for any volume ≥ 2. In debug builds this **panics** (`attempt to multiply with overflow`); in release it wraps to a garbage value (e.g. volume 5 → `set_volume(655)`). The mouse path at `src/main.rs:3400` already does it correctly with `u32` arithmetic.
- **Why it matters**: Pressing `+`/`-`/`j`/`k` on the player bar crashes a debug build and sends wrong volume to librespot in release.
- **How to verify today**: Set `JOSHIFY_MOCK=1`, focus player bar, press `+` twice with volume ≥ 5 in a debug build → panic.
- **AC**:
  - Extract a single helper `fn percent_to_volume(percent: u32) -> u16 { ((percent.min(100) as u32 * 65535) / 100) as u16 }` in `src/player/mod.rs` (alongside `LocalPlayer`).
  - All four keyboard volume sites call the helper; no inline `as u16 * 65535` remains (grep must return 0).
  - Unit test `percent_to_volume` for 0, 1, 50, 100, and 150 (clamped) — assert `percent_to_volume(100) == 65535`, `percent_to_volume(0) == 0`, no panic.
  - `cargo test --lib` passes; `cargo clippy` introduces no new warnings.
- **TDD steps**: write the unit test first (it fails today), add helper, swap call sites, verify.

---

## P0-2: `install.sh` never downloads prebuilt binaries and misses `chafa` dependency → install fails

- **Severity**: P0 (blocking install — reproduced live)
- **Files**: `install.sh`, `.github/workflows/release.yml`, `Cargo.toml`
- **Bug**: `install.sh` always runs `cargo install --path .` (source build). The `release.yml` workflow *does* build and upload prebuilt tarballs (`dist/joshify-${{ matrix.name }}.tar.gz`, latest = `joshify-linux-x86_64.tar.gz`), but the installer never fetches them. Worse, the source build requires `chafa >= 1.8.0` (hard `build.rs` requirement of `ratatui-image`), which `install.sh` never installs on any distro → the installer fails with `Failed to find chafa via pkg-config` (confirmed in the user's live run). The one-line install is the primary install path advertised in the README.
- **AC**:
  - `install.sh` (or a new `download-release.sh`) detects OS + arch and downloads the matching `joshify-*.tar.gz` asset from the latest GitHub release, verifies it with a SHA256SUMS file if published, and installs to `~/.cargo/bin/joshify`.
  - If no prebuilt asset matches, fall back to source build **and** install `libchafa-dev` (Debian/Ubuntu), `chafa` (Fedora/Arch) before `cargo install`.
  - Script exits non-zero with a clear message if both paths fail; no `set -e`-killed silent tail.
  - Test (shell): run `install.sh` in a Docker container for at least `ubuntu:24.04` and `archlinux` — both must install successfully (binary path resolves, `joshify --help` returns 0). Add a CI job to `.github/workflows/ci.yml` running the installer in a container.
- **Note**: `release.yml` only publishes `linux-x86_64` today (5 targets declared but cross-builds are incomplete/failing); the AC above must match what is actually published — do not advertise assets that don't exist.

---

## P0-3: CLI mode is a stub — `joshify play/pause/status/...` do not control Spotify

- **Severity**: P0 (advertised core feature does nothing)
- **Files**: `src/cli.rs` (esp. `cmd_play`, `cmd_pause`, `cmd_resume`, `cmd_next`, `cmd_status`, `cmd_volume`), README.md
- **Bug**: The README documents `joshify play`, `joshify pause`, `joshify status`, `joshify search`, `joshify queue-add` as functional CLI controls. In `cli.rs`, nearly every `cmd_*` just does `writeln!(self.output, "...")` — it prints a message and returns `Ok(())` without ever calling the Spotify API. `cmd_status` literally says "Mock status for now". Only `cmd_search`/`cmd_queue_add` appear to do real work.
- **AC**:
  - Wire `CliHandler` to an `Arc<Mutex<SpotifyClient>>` (same pattern as the TUI) OR to the daemon socket; pass the client in `CliHandler::new`.
  - `joshify pause` calls `playback_pause()`; `joshify resume` calls `playback_resume()`; `joshify next` calls `playback_next()`; `joshify status --format json` returns real playback state from `current_playback()`; `joshify volume 50` calls `set_volume(50)`.
  - Unit tests using a mock client (`mockall`, already a dev-dependency) assert the correct API method is invoked for each command and the output format is correct.
  - CLI commands that hit the network are covered by integration-style tests (mock) and verify error paths produce non-zero exit / error output.
  - README's CLI section updated to note commands require auth (or daemon).

---

## P0-4: Daemon mode is a complete stub — `joshify daemon-send play` does nothing real

- **Severity**: P0 (advertised core feature does nothing)
- **Files**: `src/daemon.rs` (esp. `execute_command`, `DaemonState`), `src/cli.rs`
- **Bug**: The daemon's `execute_command` only mutates an in-memory `DaemonState` (`current_track`, `progress_ms`, etc.) with **fabricated** data (`name: "Track from {uri}"`, `artists: vec!["Unknown Artist"]`, hardcoded `duration_ms: 180000`). It never connects to Spotify or librespot, never plays audio, and `Status`/`Current` return fake data. CHANGELOG advertises "Daemon Mode: Background service with Unix socket IPC". The IPC itself works, but the playback is fake.
- **AC**:
  - `DaemonService` holds a `SpotifyClient` (and/or `LocalPlayer`) reference and `execute_command` routes real commands to it (`Play { uri }` → `start_playback`/`load_uri`; `Pause` → `playback_pause`/`player.pause()`; `Status` → real `current_playback()` data, not fabricated).
  - Mock-based unit tests (`mockall`) for each `DaemonCommand` variant assert the correct underlying API call and that the returned `DaemonResponse` contains real data (track name/artists/duration from the mocked response), not placeholder strings.
  - End-to-end test (mock or a fake socket server) verifies the full `send_command → handle_connection → execute_command → response` round-trip.
  - Keep the existing 14 daemon tests green; add new ones for the real command routing.

---

## P1-1: Remote mode silently ignores user-queued tracks on auto-advance

- **Severity**: P1 (advertised queue feature broken in remote mode)
- **Files**: `src/main.rs:296-330` (`trigger_remote_advance`)
- **Bug**: When the current track ends in remote mode and there are items in the user queue, `trigger_remote_advance` computes `next_uri = queue.advance()` (which correctly pops the local queue), but then calls `guard.playback_next()` — Spotify's *server-side* next track — instead of playing `next_uri`. The popped local entry is silently discarded and Spotify's own queue advances instead. The queued track never plays.
- **AC**:
  - In remote mode, when a local queue entry is advanced, play that exact URI via `start_playback(vec![next_uri], None)` (or `start_context_playback` if a context applies), not `playback_next()`.
  - Only call `playback_next()` when there are no local queue items and we want Spotify's context to advance.
  - Unit test the decision function: given (has_up_next=true) → returns "play specific URI"; given (has_up_next=false, has_context) → returns "playback_next/context"; given (nothing) → returns "stop". Extract this into a testable pure function (e.g. in `playback/service.rs`) if needed.
  - Manual: with mock/remote device, queue 2 tracks, end the current track → the first queued track plays by its URI.

---

## P1-2: `Left` arrow in remote mode changes *volume* instead of seeking back

- **Severity**: P1 (wrong behavior for a documented keybinding)
- **Files**: `src/main.rs:2773-2790`
- **Bug**: README and help document `←`/`→` as "Seek ±10 seconds". `Right` correctly seeks forward in both modes, but `Left` in **remote** mode computes `new_vol = app.player_state.volume` and calls `set_volume` — it changes volume instead of seeking back. `Left` in local mode correctly seeks back. So `Left`/`Right` are asymmetric.
- **AC**:
  - `Left` in remote mode seeks back: `progress_ms.saturating_sub(10000)` → `guard.seek(new_pos, None)`.
  - Extract the seek-back computation into a testable helper `fn seek_back(progress_ms: u32) -> u32 { progress_ms.saturating_sub(10_000) }` with unit tests (0 → 0, 5000 → 0, 15000 → 5000, large → correct).
  - Volume adjustments remain on `-`/`+`/`j`/`k` only.
  - Verify the README keybinding table matches actual behavior for both modes.

---

## P1-3: Queue removal/`next_track` desync between `local_queue` and `PlaybackQueue`

- **Severity**: P1 (queue operations corrupt ordering)
- **Files**: `src/main.rs:1858-1875` (`'D'` remove-from-queue), `src/state/queue_state.rs:55-64` (`next_track`), `src/state/queue_state.rs:98-106` (`sync_from_playback_queue`)
- **Bug**: The `'D'` handler removes an entry from `local_queue` only, never from `playback_queue.up_next`, so the two mirrors diverge. `QueueState::next_track()` then pops `local_queue[0]` but calls `playback_queue.advance()`, which pops a *different* entry (or advances context) — queue ordering becomes wrong after any removal. Also `next_track()` uses `local_queue.remove(0)` (O(n)) with no reconciliation.
- **AC**:
  - `'D'` removes from both `local_queue` and `playback_queue.up_next` (via `remove_from_up_next`) at the same index/URI.
  - `QueueState` gets a `remove(uri: &str) -> Option<QueueEntry>` that removes from both stores atomically; `main.rs` `'D'` calls it.
  - Unit tests in `queue_state.rs`: (a) add 3, remove middle by URI, assert both `local_queue` and `playback_queue.up_next_entries()` reflect the same 2 remaining in the same order; (b) `next_track()` after removal returns the correct next entry and both stores stay consistent; (c) `next_track()` on empty returns None without advancing context.
  - `cargo test --lib` green.

---

## P1-4: `current_playback()` masks every deserialization/API error as "no playback"

- **Severity**: P1 (diagnostics & correctness)
- **Files**: `src/api/playback.rs:20-92`
- **Bug**: In `current_playback()`, any `serde_json` parse failure or any error whose string contains "player"/"device"/"404"/"400" is silently converted to `Ok(None)`. This means real API schema drift, rate-limit errors, and auth failures are indistinguishable from "nothing playing" — the TUI shows "Nothing playing" and never surfaces the error. The "any deserialization error = no playback" fallback (line 60-61) is the worst: a broken schema change would hide playback entirely.
- **AC**:
  - Distinguish genuine "no active playback" (empty body, `null`, `204`, explicit `NO_ACTIVE_DEVICE`) from parse/network/auth errors.
  - Return `Ok(None)` only for the genuine cases; propagate other errors so the TUI shows the message.
  - Unit tests with a mocked HTTP layer (or by refactoring the JSON→result decision into a pure `fn interpret_playback_response(body: Option<&str>, err: Option<&str>) -> Result<Option<()>, String>`): empty → Ok(None); `null` → Ok(None); malformed JSON → Err; `404` with device error → Ok(None) only when it's the documented no-device case; a real API error (401, 429) → Err.
  - Keep the `PlayableItem`/ad variant tolerance (that one legitimately means "can't parse item") but bound it.

---

## P1-5: `poll_playback` holds the shared client Mutex across a network await, serializing all API work

- **Severity**: P1 (perf/stall)
- **Files**: `src/main.rs:153-275` (`poll_playback`), call site `src/main.rs:628-635`
- **Bug**: `poll_playback` does `client.lock().await` and holds the lock for the entire `current_playback()` HTTP request. Meanwhile every other async task (search, liked-songs pagination, playlist load, queue-add) also needs that same lock. A slow Spotify response blocks ALL background loads for up to the request duration, every 2 seconds.
- **AC**:
  - Do not hold the `Arc<Mutex<SpotifyClient>>` across the network await in the poll path: e.g. clone the data needed (or snapshot the token), drop the guard, then call the API. Since `current_playback()` is a single method on the shared client, restructure so the lock is only held for setup (token read) and released during the HTTP call (or run the poll entirely inside a spawned task with its own scoped lock acquisition window).
  - No `client.lock().await` may span a network call anywhere in `main.rs` (audit all `.lock().await` usages — grep + review).
  - Test: a unit/integration test asserting that two concurrent operations (poll + a mocked slow search) do not deadlock and both complete (use `tokio::time::timeout`); plus a code-level check that poll doesn't wrap its network call in the lock.
- **Note**: If refactoring `current_playback()` to not need the lock is infeasible with rspotify's API, document the trade-off and instead ensure the poll interval + timeout prevent indefinite stalls (add a request timeout on the client).

---

## P2-1: CI is red on `main` — fmt check and security audit fail

- **Severity**: P2
- **Files**: `.github/workflows/ci.yml`
- **Bug**: Latest CI runs on `main` show failure. `cargo fmt --check` fails because the tree isn't rustfmt-clean (confirmed locally), and the `rustsec/audit-check` reports critical vulnerabilities and "Resource not accessible by integration" errors. The Release workflow also fails. The repo's own README shows a green CI badge but the pipeline is actually red.
- **AC**:
  - `cargo fmt --all` is clean and committed; `cargo fmt --check` passes in CI.
  - `cargo audit` output is reviewed; fix or document each advisory (dependency bumps where available — several deps have newer versions per Cargo.lock); if a fix requires a major bump, open a separate tracked issue and add an explicit `ignore`/rationale comment rather than leaving a hard failure.
  - Fix the audit-check token/permission issue (`contents: read` on the job or use `security-events: write`) so the check can publish.
  - CI on `main` is green for `fmt`, `clippy`, `test`, `build` (the `ci-success` gate passes).

---

## P2-2: Docs/version drift — README, VERSION, Cargo.toml, release tags disagree

- **Severity**: P2
- **Files**: `VERSION` (=`0.2.0`), `Cargo.toml` (=`0.5.0`), `package.json` (=`0.1.0`), README (test count, badges, broken links), `CHANGELOG.md`
- **Issues**:
  - `VERSION` file says `0.2.0`; Cargo.toml says `0.5.0`; latest GH release is `v0.6.0`. `release.yml` reads the version from Cargo.toml only.
  - README claims "339 tests" and "81 pass" is in AGENTS.md — actual count is ~573 test attributes; README badge says `tests-339 passing`.
  - README references `LICENSE`, `CONTRIBUTING.md`, `assets/logo.png`, `assets/demo.gif`, `screenshots/reference/*.png` — none of these files exist (confirmed via `ls`).
  - README links a `latest` prebuilt release; only `linux-x86_64` asset exists.
- **AC**:
  - Add `LICENSE` (MIT — package.json already declares it) and `CONTRIBUTING.md`, or remove the broken links from the README.
  - Make version single-source: either delete `VERSION` and derive everything from Cargo.toml, or make a CI job that keeps them in sync. All three of Cargo.toml/VERSION/latest-tag must match after a release.
  - Update README test-count badge to the real count, and make the count verifiable (a CI step that greps `#[test]` and updates the badge, or just correct the number and note the command).
  - Fix or remove references to non-existent assets (logo, demo.gif, screenshot paths) — either commit real files under `assets/`/`screenshots/` or remove the markup.
- **Verification**: run `cargo run -- --help` and a fresh `JOSHIFY_MOCK=1` session; README links resolve; `cargo test --bin joshify --test performance_tests` count matches the README badge.

---

## P2-3: Artist library and artist top-tracks views are unimplemented (empty/placeholder)

- **Severity**: P2 (incomplete feature reachable in UI)
- **Files**: `src/main.rs:1660-1664` (`LoadAction::LibraryArtists` → "not yet implemented"), `src/main.rs:1722-1738` (`LoadAction::ArtistTopTracks` → immediately sends `ArtistDetail` with no tracks)
- **Bug**: Selecting the Artists tab in Library shows a "not yet implemented" error; clicking an artist shows an `ArtistDetail` with no track list (the `ArtistTopTracks` load action never fetches anything — it just sends an empty `ArtistDetail` and sets `LoadingInProgress`, which can also leave the UI stuck).
- **AC**:
  - Implement `LoadAction::LibraryArtists` → fetch followed artists via `oauth.followed_artists()` and populate `Library { artists, .. }`.
  - Implement `LoadAction::ArtistTopTracks` → fetch top tracks via `oauth.artist_top_tracks()` and render them as a `TrackListItem` list; set a proper `Artist` context so playback/advance works.
  - `ArtistDetail` state carries the tracks; navigation back (`Backspace`) returns to the Library Artists list.
  - Mock-based tests for both load actions assert the correct API calls and the resulting `ContentState`; error paths send `ContentState::Error`.
  - Manual: `JOSHIFY_MOCK=1` or real account — Artists tab loads; clicking an artist shows their top tracks; pressing Enter plays the first.

---

## P2-4: EndOfTrack context-advance shows the *previous* track's name in the status message

- **Severity**: P2 (wrong UI message)
- **Files**: `src/main.rs:881-903`
- **Bug**: When advancing through context tracks in local mode, the success path reads `track_name`/`artist_name` from `app.player_state.current_track_name` — which still holds the *just-ended* track — and prints "Playing next from playlist: {old} - {old_artist}". The `next_uri` is correct; only the message uses stale metadata.
- **AC**:
  - The status message uses the new track's metadata (fetch it from the queued `QueueEntry`/context, or set name to the URI fallback until `TrackChanged` fills it in).
  - Unit-testable: extract the message-building into a pure function `fn next_play_message(next_name: Option<&str>, next_artist: Option<&str>) -> String` and test both known and unknown-metadata cases.
  - Manual: play playlist track 1, let it end, status bar must not show track 1's name for the "next" message.

---

## P2-5: `media_control.rs` and `notifications.rs` are stubs but README/CHANGELOG advertise them as working

- **Severity**: P2 (misleading feature claims)
- **Files**: `src/media_control.rs` (all platform init functions are `info!("... would initialize ...")` stubs), `src/notifications.rs` (`StubNotifier`), README, CHANGELOG
- **Bug**: README lists "MPRIS/media keys" and "Desktop Notifications" as features and CHANGELOG claims them; the implementations are no-op stubs that never register with the OS.
- **AC** (choose one direction):
  - **Implement**: Wire Linux notifications via `notify-rust` (add dependency) so `notify_track_change` posts a real notification; implement MPRIS registration via `dbus`/`mpris-server`, or
  - **De-scope**: Remove the feature claims from README/CHANGELOG and gate the modules behind a `media-control`/`notifications` cargo feature defaulting to off, with clear "unimplemented" messages.
  - Either way: no code path claims a notification was shown when none was (the stub must not print "Notification sent"). Unit tests assert stub behavior is honest (e.g. returns `Err` or logs "not supported").

---

## P3-1: Album-art fetch has a 2s cooldown keyed on URI, so rapid track changes starve art

- **Severity**: P3
- **Files**: `src/main.rs:944-985`
- **Bug**: `can_fetch_art` requires `now - last_art_fetch_ms >= 2000` AND `last_fetched_art_uri != current`. After two rapid track changes within 2s, the second track's art is never fetched (and the guard is per-URI, so skipping a track that was already fetched earlier in the session won't refetch). Combined with `PlayerEvent::TrackChanged` art logic in `main.rs:938-987` this can leave stale art on the second track.
- **AC**:
  - Make the cooldown keyed on (URI + track-change timestamp): a track change always permits fetching art for the *new* URI regardless of the global cooldown; the cooldown only suppresses refetching the *same* URI.
  - Unit test a pure `fn should_fetch_art(current_uri, last_uri, last_fetch_ms, now_ms) -> bool` covering: same URI within cooldown → false; new URI regardless of cooldown → true; same URI after cooldown → true.
  - Manual: rapidly skip 3 tracks in local mode; each track shows its own art (or art area cleared if fetch fails).

---

## P3-2: OAuth callback server aborts after 100 ms, before the browser response may finish

- **Severity**: P3
- **Files**: `src/auth.rs:396-400`
- **Bug**: After receiving the code, `run_oauth_callback_server` sleeps 100 ms then `handle.abort()`. The hyper connection may not have flushed the success HTML response to the browser in that window, so users can see a connection error page ("Not found" / reset) even though auth succeeded.
- **AC**:
  - Wait for the response to be fully sent before aborting (e.g. track completion via a oneshot that the service_fn signals after `hyper` returns, or a graceful `serve_connection` shutdown).
  - Unit/integration test: start the callback server with a mock Spotify flow, complete an HTTP GET to `/callback?code=abc`, assert the client receives HTTP 200 with the success HTML body and the server shuts down cleanly afterward (no abort-before-response).

---

## P3-3: `SearchState` results limit vs Spotify hard limit (15 requested, 10 returned)

- **Severity**: P3
- **Files**: `src/api/library.rs:117` (`limit.min(10)`), `src/main.rs:1032` (`search(&query, 15)`)
- **Bug**: The UI requests 15 results but the API clamps to 10 (Spotify max for a single type). Minor mismatch; UI may reserve space for 15.
- **AC**:
  - Align: either request and display 10, or batch search to fetch 20 and display 15.
  - Unit test `search()` with limit 15 → 10 items returned (documented), or raise the cap and assert 15 come back with pagination.
  - No UI visible empty row or gap caused by expecting more than returned.

---

## Suggested execution order

1. P0-1 (volume overflow) — smallest, highest blast radius.
2. P0-2 (installer) — unblocks all users; pair with fixing release.yml asset publishing.
3. P0-3 + P0-4 (CLI/daemon stubs) — decide together since they share the client wiring.
4. P1-1, P1-2, P1-3 (playback/queue correctness).
5. P1-4, P1-5 (API error masking, lock contention).
6. P2-1 (CI green) — do early so every later task has a green baseline.
7. P2-2 (docs drift), P2-3 (artist views), P2-4, P2-5.
8. P3 items as time allows.

## Definition of Done (applies to every item)

- Acceptance criteria from the item's AC section are implemented.
- New tests are written first (TDD) and pass: `cargo test --lib`.
- Performance regression suite still passes: `cargo test --bin joshify --test performance_tests`.
- `cargo fmt --check` clean; `cargo clippy --message-format=short` adds no new warnings.
- `tasks/todo.md`, `.learnings/learnings.md`, `.learnings/history.md` updated per AGENTS.md.
