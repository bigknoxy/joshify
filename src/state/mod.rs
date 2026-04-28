//! Application state management
//!
//! Modular state design with clear boundaries:
//! - `app_state` - Navigation, focus, UI overlays, content state
//! - `player_state` - Playback state (moved from player.rs)
//! - `queue_state` - Queue management
//! - `library_state` - Liked songs, playlists
//! - `search_state` - Search with debounce and caching
//! - `mock_data` - Mock data for visual testing (VHS)

pub mod app_state;
pub mod home_state;
pub mod library_state;
pub mod load_coordinator;
pub mod mock_data;
pub mod navigation_stack;
pub mod player_state;
pub mod queue_state;
pub mod search_state;

// Re-export specific items for public use
pub use app_state::{ContentState, FocusTarget, NavItem};
pub use load_coordinator::LoadAction;
