//! Tests for playback error handling

#[test]
fn test_error_string_matching() {
    // Test that our error string matching works for various error formats

    let test_cases = vec![
        ("No active device found", true),
        ("NO_ACTIVE_DEVICE", true),
        ("no active device", true),
        ("No_Active_Device", false), // Underscores don't match spaces
        ("Playback failed", false),
        ("Token expired", false),
        ("", false),
    ];

    for (error_msg, should_match) in test_cases {
        let matches = error_msg.contains("NO_ACTIVE_DEVICE")
            || error_msg.to_lowercase().contains("no active device");

        assert_eq!(
            matches, should_match,
            "Error matching failed for: {}",
            error_msg
        );
    }
}

#[test]
fn test_error_matching_case_insensitive() {
    // Test case-insensitive matching
    let error_msg = "No Active Device Found";
    let matches = error_msg.to_lowercase().contains("no active device");
    assert!(matches, "Should match case-insensitively");
}

#[test]
fn test_error_matching_underscore() {
    // Test matching with underscores
    let error_msg = "NO_ACTIVE_DEVICE";
    let matches = error_msg.contains("NO_ACTIVE_DEVICE");
    assert!(matches, "Should match underscore version");
}
