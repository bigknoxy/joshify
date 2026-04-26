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
