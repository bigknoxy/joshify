//! Album-art fetch cooldown policy (#24).
//!
//! The cooldown existed to stop an art-fetch storm during rapid seeking, but it
//! was applied globally: a track change within the 2s window after the previous
//! fetch was silently dropped, so the new track kept the *old* track's art.
//!
//! These tests exercise the real `should_fetch_art` from the library rather than
//! a re-implementation, so they fail if the shipped policy regresses. (The
//! sibling `album_art.rs` test file mirrors logic locally instead, which cannot
//! catch that.)

use joshify::album_art::{should_fetch_art, ART_FETCH_COOLDOWN_MS};

const NOW: u64 = 10_000;
const URI_A: &str = "spotify:track:aaa";
const URI_B: &str = "spotify:track:bbb";

#[test]
fn a_new_uri_is_fetched_even_inside_the_cooldown_window() {
    // The regression: two rapid track changes, second one starved of art.
    assert!(
        should_fetch_art(URI_B, Some(URI_A), NOW - 100, NOW),
        "a track change must always be allowed to fetch art for the new URI, \
         regardless of how recently the previous URI was fetched"
    );
}

#[test]
fn the_same_uri_is_not_refetched_inside_the_cooldown_window() {
    // The cooldown's original purpose: suppress refetch storms while seeking.
    assert!(!should_fetch_art(URI_A, Some(URI_A), NOW - 100, NOW));
}

#[test]
fn the_same_uri_is_refetched_once_the_cooldown_expires() {
    assert!(should_fetch_art(
        URI_A,
        Some(URI_A),
        NOW - ART_FETCH_COOLDOWN_MS,
        NOW
    ));
}

#[test]
fn the_first_fetch_ever_is_allowed() {
    assert!(should_fetch_art(URI_A, None, 0, NOW));
}

#[test]
fn cooldown_is_not_starved_by_a_stale_zero_timestamp() {
    // last_fetch_ms == 0 means "never fetched", not "fetched at the epoch".
    // Subtracting from it saturates to 0, so the elapsed check would pass and
    // the very first fetch would be wrongly refused as same-URI-within-cooldown.
    assert!(should_fetch_art(URI_A, Some(URI_A), 0, 0));
}

#[test]
fn a_uri_fetched_long_ago_but_now_different_is_still_fetched() {
    // Per-URI cooldown, not a global timer: hours-old fetches must not gate a
    // different track.
    assert!(should_fetch_art(URI_B, Some(URI_A), 0, NOW));
}
