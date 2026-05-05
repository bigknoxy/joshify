//! UI components and layout
//!
//! Modular UI design:
//! - `sidebar` - Navigation sidebar with logo
//! - `main_view` - Track lists, playlists, search results
//! - `player_bar` - Now playing bar with album art
//! - `overlays` - Search input, help, queue overlays
//! - `help` - Comprehensive help overlay with keyboard and mouse controls
//! - `theme` - Catppuccin Mocha color system
//! - `mouse` - Mouse event handling utilities

pub mod device_selector;
pub mod help;
pub mod home_view;
pub mod image_renderer;
pub mod layout_cache;
pub mod lite_mode;
pub mod main_view;
pub mod mouse_handler;
pub mod overlays;
pub mod player_bar;
pub mod sidebar;
pub mod theme;

pub use device_selector::render_device_selector;
pub use help::{render_help_overlay, HelpContent, HelpOverlayState};
pub use layout_cache::{ClickableArea, LayoutCache};
pub use lite_mode::{render_lite_help, render_lite_mode};
pub use main_view::render_main_view;
pub use mouse_handler::{
    handle_left_click, handle_mouse_event, handle_scroll_down, handle_scroll_up, MouseAction,
    MouseState,
};
pub use overlays::{render_queue_overlay, render_search_overlay};
pub use player_bar::render_player_bar;
pub use sidebar::render_sidebar;

// Re-export types used by UI
pub use crate::state::app_state::NavItem;
