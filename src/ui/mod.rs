//! UI components and layout
//!
//! Modular UI design:
//! - `sidebar` - Navigation sidebar with logo
//! - `main_view` - Track lists, playlists, search results
//! - `player_bar` - Now playing bar with album art
//! - `overlays` - Search input, help, queue overlays

pub mod device_selector;
pub mod image_renderer;
pub mod main_view;
pub mod overlays;
pub mod player_bar;
pub mod sidebar;

pub use device_selector::render_device_selector;
pub use main_view::render_main_view;
pub use overlays::{render_help_overlay, render_queue_overlay};
pub use player_bar::render_player_bar;
pub use sidebar::render_sidebar;

// Re-export types used by UI
pub use crate::state::app_state::NavItem;
