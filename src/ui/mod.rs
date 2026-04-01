//! UI components and layout
//!
//! Modular UI design:
//! - `sidebar` - Navigation sidebar with logo
//! - `main_view` - Track lists, playlists, search results
//! - `player_bar` - Now playing bar with album art
//! - `overlays` - Search input, help, queue overlays

pub mod sidebar;
pub mod main_view;
pub mod player_bar;
pub mod overlays;

pub use sidebar::render_sidebar;
pub use main_view::render_main_view;
pub use player_bar::render_player_bar;
pub use overlays::{render_queue_overlay, render_help_overlay};

// Re-export types used by UI
pub use crate::state::app_state::NavItem;
