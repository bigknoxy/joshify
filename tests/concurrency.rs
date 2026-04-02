//! Concurrency tests
//!
//! Tests for async task coordination, race conditions, and stale result rejection.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// Simulated LoadAction for testing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LoadAction {
    PlaylistTracks(String),
    Search(String),
    LikedSongs,
}

/// Simulated sequence number tracker
struct SequenceTracker {
    current: u64,
}

impl SequenceTracker {
    fn new() -> Self {
        Self { current: 0 }
    }

    fn next(&mut self) -> u64 {
        self.current += 1;
        self.current
    }
}

/// Simulated stale result checker
fn is_stale(expected_sequence: u64, actual_sequence: u64) -> bool {
    actual_sequence < expected_sequence
}

#[tokio::test]
async fn test_album_art_race() {
    // Simulate two concurrent album art fetches
    let url1 = "https://example.com/art1.jpg";
    let url2 = "https://example.com/art2.jpg";
    let current_track = Arc::new(Mutex::new(url1.to_string()));
    let current_track_clone = current_track.clone();

    // First fetch starts
    let fetch1 = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        // Check if track changed during fetch
        let current = current_track.lock().unwrap();
        if *current == url1 {
            Some(vec![0x89, 0x50]) // PNG header
        } else {
            None // Stale, discard
        }
    });

    // Track changes mid-fetch
    tokio::time::sleep(Duration::from_millis(25)).await;
    {
        let mut track = current_track_clone.lock().unwrap();
        *track = url2.to_string();
    }

    let result1 = fetch1.await.unwrap();
    // Result should be discarded because track changed
    assert!(result1.is_none(), "Stale album art should be discarded");
}

#[tokio::test]
async fn test_duplicate_load_prevention() {
    // Same load action twice - only one should spawn
    let active_tasks = Arc::new(Mutex::new(std::collections::HashSet::new()));
    let active_tasks_clone = active_tasks.clone();

    let action = LoadAction::PlaylistTracks("My Playlist".to_string());
    let action_key = format!("{:?}", action);

    // First task registration
    {
        let mut tasks = active_tasks_clone.lock().unwrap();
        let inserted = tasks.insert(action_key.clone());
        assert!(inserted, "First task should be registered");
    }

    // Second task tries to register same action
    {
        let mut tasks = active_tasks_clone.lock().unwrap();
        let inserted = tasks.insert(action_key.clone());
        assert!(!inserted, "Duplicate task should not be registered");
    }
}

#[tokio::test]
async fn test_task_cancellation() {
    // Navigate away mid-fetch - task should be cancellable
    let (tx, mut rx) = mpsc::channel::<String>(1);
    let tx_clone = tx.clone();

    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = tx_clone.send("result".to_string()).await;
    });

    // Cancel before completion
    tokio::time::sleep(Duration::from_millis(50)).await;
    handle.abort();

    // Channel should not receive result
    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert!(
        result.is_err() || result.unwrap().is_none(),
        "Cancelled task should not send result"
    );
}

#[tokio::test]
async fn test_stale_result_rejected() {
    // Old sequence number should be discarded
    let mut tracker = SequenceTracker::new();

    // Start task with sequence 1
    let seq1 = tracker.next();

    // User triggers new load, sequence advances
    let seq2 = tracker.next();
    assert!(seq2 > seq1);

    // Old task completes - should be rejected
    assert!(is_stale(seq2, seq1), "Old sequence should be stale");

    // New task completes - should be accepted
    assert!(
        !is_stale(seq2, seq2),
        "Current sequence should not be stale"
    );
}

#[tokio::test]
async fn test_channel_backpressure() {
    // Slow receiver - channel should handle backpressure
    let (tx, mut rx) = mpsc::channel::<i32>(5);

    // Send more than buffer capacity
    let send_handle = tokio::spawn(async move {
        for i in 0..10 {
            let _ = tx.send(i).await;
        }
    });

    // Receive with delay
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut received = Vec::new();
    while let Ok(item) = rx.try_recv() {
        received.push(item);
    }

    // Should have received items (backpressure handled)
    assert!(!received.is_empty(), "Should receive some items");

    // Clean up sender
    let _ = send_handle.await;
}
