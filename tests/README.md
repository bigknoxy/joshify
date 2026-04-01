# Joshify Test Suite

Target: **80% line coverage** on critical paths.

## Test Categories

### 1. Authentication Flow (`tests/auth.rs`)

| Test | Description | Critical |
|------|-------------|----------|
| `test_credentials_from_env` | Load credentials from environment variables | Yes |
| `test_credentials_expired` | Credentials correctly report expired state | Yes |
| `test_credentials_valid` | Credentials correctly report valid state | Yes |
| `test_oauth_config_from_args` | CLI args override env vars | Yes |
| `test_oauth_config_default` | Default config when no args/env | No |
| `test_save_load_credentials` | Round-trip credential persistence | Yes |
| `test_keyring_storage` | Tokens stored in OS keyring | Yes |
| `test_keyring_fallback` | Plaintext fallback when keyring unavailable | Yes |

### 2. Player State (`tests/player.rs`)

| Test | Description | Critical |
|------|-------------|----------|
| `test_player_state_from_context` | Convert Spotify context to PlayerState | Yes |
| `test_player_state_track` | Track playback context | Yes |
| `test_player_state_episode` | Episode playback context | No |
| `test_player_state_no_context` | Handle None context gracefully | Yes |
| `test_format_duration` | Format ms as MM:SS | Yes |
| `test_format_duration_edge_cases` | Zero, overflow, large values | No |

### 3. Album Art Cache (`tests/album_art.rs`)

| Test | Description | Critical |
|------|-------------|----------|
| `test_cache_new` | Create empty cache | No |
| `test_cache_hit` | Return cached image data | Yes |
| `test_cache_miss` | Return None for uncached URL | Yes |
| `test_cache_eviction` | LRU eviction at 50 entries | Yes |
| `test_cache_disk_persistence` | Survive restart via disk cache | Yes |
| `test_cache_url_to_filename` | Safe filename conversion | No |

### 4. API Client (`tests/api.rs`)

| Test | Description | Critical |
|------|-------------|----------|
| `test_spotify_client_new` | Create client with config | Yes |
| `test_spotify_client_cached_auth` | Apply cached credentials | Yes |
| `test_current_playback` | Fetch playback state | Yes |
| `test_current_playback_no_device` | Handle NO_ACTIVE_DEVICE | Yes |
| `test_playback_controls` | Play, pause, next, previous | Yes |
| `test_set_volume` | Volume 0-100, clamping | Yes |
| `test_search` | Search tracks, limit results | Yes |
| `test_start_playback` | Play specific track URI | Yes |
| `test_add_to_queue` | Add track to queue | Yes |
| `test_seek` | Seek to position | Yes |
| `test_rate_limit_backoff` | Exponential backoff on 429 | Yes |
| `test_invalid_track_uri` | Handle malformed URI | Yes |

### 5. UI Components (`tests/ui.rs`)

| Test | Description | Critical |
|------|-------------|----------|
| `test_nav_item_all` | All navigation items | No |
| `test_nav_item_label` | Label for each nav item | No |
| `test_render_sidebar` | Sidebar renders without panic | No |
| `test_render_player_bar` | Player bar renders | No |
| `test_render_track_list` | Track list renders | No |
| `test_render_playlist_list` | Playlist list renders | No |
| `test_search_input_overlay` | Search overlay renders | No |
| `test_help_overlay` | Help overlay renders | No |

### 6. State Management (`tests/state.rs`)

| Test | Description | Critical |
|------|-------------|----------|
| `test_app_state_new` | Create initial AppState | Yes |
| `test_focus_cycle` | Tab/Shift+Tab cycles focus | Yes |
| `test_content_state_transitions` | Navigate between content states | Yes |
| `test_search_input` | Search query buffer | Yes |
| `test_scroll_offset` | List auto-scroll | No |

### 7. Integration Tests (`tests/integration.rs`)

| Test | Description | Critical |
|------|-------------|----------|
| `test_full_auth_flow` | Browser OAuth → token cache | Yes |
| `test_playback_session` | Auth → search → play → pause | Yes |
| `test_queue_workflow` | Add to queue → view queue | No |
| `test_playlist_workflow` | List playlists → select → play | Yes |

### 8. Concurrency Tests (`tests/concurrency.rs`) — NEW

| Test | Description | Critical |
|------|-------------|----------|
| `test_album_art_race` | Track change during fetch — old fetch doesn't overwrite new | Yes |
| `test_duplicate_load_prevention` | Same load action twice — only one spawns | Yes |
| `test_task_cancellation` | Navigate away mid-fetch — task cancels or result ignored | Yes |
| `test_stale_result_rejected` | Old sequence number — result discarded | Yes |
| `test_channel_backpressure` | Main loop slow — sends don't block forever | Yes |

### 9. Error Injection Tests (`tests/error_injection.rs`) — NEW

| Test | Description | Critical |
|------|-------------|----------|
| `test_album_art_timeout` | Fetch times out — graceful fallback, no panic | Yes |
| `test_malformed_json` | Spotify returns invalid JSON — error logged, app continues | Yes |
| `test_keyring_failure` | Keyring returns Err — plaintext fallback works | Yes |
| `test_rate_limit_exhaust` | 5 rate limits — error shown, app recoverable | Yes |
| `test_token_expiry_mid_session` | Token expires during use — graceful re-auth prompt | Yes |

## Running Tests

```bash
# Unit tests only
cargo test --lib

# All tests (including integration)
cargo test

# With coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir ./coverage

# Coverage report (target: 80%)
cargo tarpaulin --out Lcov
genhtml lcov.info --output-directory coverage-html
```

## Mocking Strategy

### Spotify API Mocks

Use `mockall` crate for mocking `SpotifyClient`:

```rust
#[cfg(test)]
mod tests {
    use mockall::predicate::*;
    use crate::api::MockSpotifyClient;

    #[tokio::test]
    async fn test_current_playback() {
        let mut mock = MockSpotifyClient::new();
        mock.expect_current_playback()
            .returning(|| Ok(Some(mock_playback_context())));

        let result = mock.current_playback().await;
        assert!(result.is_ok());
    }
}
```

### Token Mocks

```rust
fn mock_credentials() -> Credentials {
    Credentials {
        access_token: "test_access_token".to_string(),
        refresh_token: Some("test_refresh_token".to_string()),
        expires_at: (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600), // 1 hour from now
    }
}
```

## Dependencies for Testing

Add to `Cargo.toml`:

```toml
[dev-dependencies]
mockall = "0.13"
tokio-test = "0.4"
tempfile = "3" # For temp directories
```
