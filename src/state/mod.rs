//! Application state management
//!
//! Modular state design with clear boundaries:
//! - `app_state` - Navigation, focus, UI overlays, content state
//! - `player_state` - Playback state (moved from player.rs)
//! - `queue_state` - Queue management
//! - `library_state` - Liked songs, playlists

pub mod app_state;
pub mod player_state;
pub mod queue_state;
pub mod library_state;
pub mod load_coordinator;

// Re-export specific items for public use
pub use app_state::{ContentState, NavItem, FocusTarget};
pub use load_coordinator::LoadAction;
