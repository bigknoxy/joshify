# Audit Backlog — 2026-08-08

## Goal
Full audit of the repo: code review, feature review, build/install tooling, CI health, docs drift. Findings filed as GitHub issues #10-#26 with testable AC.

## Audit Findings (prioritized)

### P0 — High severity
| # | Issue | GH |
|---|-------|----|
| P0-1 | Volume controls panic in debug / garbage volume in release (u16 overflow at main.rs:2675,2734,2818,2836) | #10 |
| P0-2 | install.sh never downloads prebuilt binaries + misses chafa dep -> install fails | #11 |
| P0-3 | CLI mode is a stub - play/pause/status do not control Spotify | #12 |
| P0-4 | Daemon mode is a complete stub - daemon-send does nothing real | #13 |

### P1 — High
| # | Issue | GH |
|---|-------|----|
| P1-1 | Remote mode silently ignores user-queued tracks on auto-advance | #14 |
| P1-2 | Left arrow in remote mode changes volume instead of seeking back | #15 |
| P1-3 | Queue removal/next_track desync between local_queue and PlaybackQueue | #16 |
| P1-4 | current_playback() masks every deserialization/API error as no playback | #17 |
| P1-5 | poll_playback holds client Mutex across network await, serializing API work | #18 |

### P2 — Medium
| # | Issue | GH |
|---|-------|----|
| P2-1 | CI is red on main - fmt check and security audit fail | #19 |
| P2-2 | Docs/version drift - README, VERSION, Cargo.toml, release tags disagree | #20 |
| P2-3 | Artist library and artist top-tracks views are unimplemented | #21 |
| P2-4 | EndOfTrack context-advance shows previous track's name in status message | #22 |
| P2-5 | media_control.rs and notifications.rs are stubs but advertised as working | #23 |

### P3 — Low
| # | Issue | GH |
|---|-------|----|
| P3-1 | Album-art 2s cooldown keyed on URI starves art on rapid track changes | #24 |
| P3-2 | OAuth callback server aborts after 100ms before browser response finishes | #25 |
| P3-3 | SearchState requests 15 results but API clamps to 10 | #26 |

Full AC for each item: `tasks/AUDIT_BACKLOG.md`

## Suggested Execution Order
1. P0-1 volume overflow (smallest, highest blast radius)
2. P0-2 installer (unblocks all users; pair with release.yml fix)
3. P0-3 + P0-4 CLI/daemon stubs (shared client wiring decision)
4. P1-1, P1-2, P1-3 playback/queue correctness
5. P1-4, P1-5 API masking, lock contention
6. P2-1 CI green (every later task needs green baseline)
7. P2-2 docs drift, P2-3 artist views, P2-4, P2-5
8. P3 as time allows

## Verification Checklist
- [x] P0-1: percent_to_volume helper + unit tests; no `as u16 * 65535` remains
- [x] P0-2: installer downloads prebuilt release binary; chafa installed on source fallback
- [x] P0-3: CLI routes real commands via mockall-tested client (CliClient trait + CliHandler wired into main)
- [ ] P0-4: daemon routes real commands via mockall-tested client
- [ ] All 451 library tests pass
- [ ] All 18 performance tests pass
- [ ] cargo fmt --check clean, clippy no new warnings
