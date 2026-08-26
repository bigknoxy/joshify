//! Transport key mapping for playback controls.
//!
//! Centralises the (mode, key) → command decision so the remote/local
//! behaviour cannot drift apart again — a copy-paste slip previously made
//! Left re-apply the volume instead of seeking back in remote mode.

use crossterm::event::KeyCode;

/// A resolved transport action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportCommand {
    TogglePlayPause,
    NextTrack,
    PreviousTrack,
    /// Seek back 10 seconds from the current position.
    SeekBack10s,
    /// Seek forward 10 seconds, clamped to the track duration by the caller.
    SeekForward10s,
    VolumeUp5,
    VolumeDown5,
}

/// Map a key press to a transport command. Identical in both playback modes:
/// mode only changes how the command is executed, never which key does what.
pub fn map_transport_key(key: KeyCode) -> Option<TransportCommand> {
    match key {
        KeyCode::Char(' ') => Some(TransportCommand::TogglePlayPause),
        KeyCode::Char('n') => Some(TransportCommand::NextTrack),
        KeyCode::Char('p') => Some(TransportCommand::PreviousTrack),
        KeyCode::Left => Some(TransportCommand::SeekBack10s),
        KeyCode::Right => Some(TransportCommand::SeekForward10s),
        KeyCode::Char('+') | KeyCode::Char('=') => Some(TransportCommand::VolumeUp5),
        KeyCode::Char('-') | KeyCode::Char('_') => Some(TransportCommand::VolumeDown5),
        _ => None,
    }
}

/// Position after seeking back 10 seconds, floored at zero.
pub fn seek_back_position(current_ms: u32) -> u32 {
    current_ms.saturating_sub(10_000)
}

/// Position after seeking forward 10 seconds, clamped to the duration.
pub fn seek_forward_position(current_ms: u32, duration_ms: u32) -> u32 {
    current_ms.saturating_add(10_000).min(duration_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Falsifier for the copy-paste bug where remote-mode Left re-applied
    /// the current volume instead of seeking back.
    #[test]
    fn test_left_arrow_is_seek_back_not_volume() {
        let cmd = map_transport_key(KeyCode::Left).expect("arrow keys must map");
        assert_eq!(
            cmd,
            TransportCommand::SeekBack10s,
            "Left must seek back in every mode"
        );
        assert_ne!(cmd, TransportCommand::VolumeDown5);
    }

    #[test]
    fn test_transport_key_map_covers_all_documented_bindings() {
        assert_eq!(
            map_transport_key(KeyCode::Char(' ')),
            Some(TransportCommand::TogglePlayPause)
        );
        assert_eq!(
            map_transport_key(KeyCode::Char('n')),
            Some(TransportCommand::NextTrack)
        );
        assert_eq!(
            map_transport_key(KeyCode::Char('p')),
            Some(TransportCommand::PreviousTrack)
        );
        assert_eq!(
            map_transport_key(KeyCode::Right),
            Some(TransportCommand::SeekForward10s)
        );
        assert_eq!(
            map_transport_key(KeyCode::Char('+')),
            Some(TransportCommand::VolumeUp5)
        );
        assert_eq!(
            map_transport_key(KeyCode::Char('-')),
            Some(TransportCommand::VolumeDown5)
        );
        // Unrelated keys must not be swallowed.
        assert_eq!(map_transport_key(KeyCode::Char('j')), None);
        assert_eq!(map_transport_key(KeyCode::Esc), None);
    }

    #[test]
    fn test_seek_math_saturates_and_clamps() {
        assert_eq!(seek_back_position(4_000), 0);
        assert_eq!(seek_back_position(65_000), 55_000);
        // Never past the end of the track.
        assert_eq!(seek_forward_position(95_000, 100_000), 100_000);
        assert_eq!(seek_forward_position(0, 0), 0);
    }
}
