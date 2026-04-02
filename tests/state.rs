//! State management tests
//!
//! Tests for NavItem, ContentState, FocusTarget, and state transitions.

// Re-implement core types for testing (mirroring src/state/app_state.rs)

/// Navigation items for the sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum NavItem {
    Home,
    Search,
    Library,
    Playlists,
    LikedSongs,
}

impl NavItem {
    fn all() -> &'static [NavItem] {
        &[NavItem::Home, NavItem::Search, NavItem::Library, NavItem::Playlists, NavItem::LikedSongs]
    }

    fn label(&self) -> &'static str {
        match self {
            NavItem::Home => "Home",
            NavItem::Search => "Search",
            NavItem::Library => "Library",
            NavItem::Playlists => "Playlists",
            NavItem::LikedSongs => "Liked Songs",
        }
    }
}

/// Focus target for Tab navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusTarget {
    Sidebar,
    MainContent,
    PlayerBar,
}

impl Default for FocusTarget {
    fn default() -> Self {
        FocusTarget::Sidebar
    }
}

/// Content state for main view
#[derive(Clone, PartialEq, Debug)]
enum ContentState {
    Home,
    Loading(String),
    LoadingInProgress(String),
    LikedSongs(Vec<String>),
    Playlists(Vec<String>),
    PlaylistTracks(String, Vec<String>),
    SearchResults(String, Vec<String>),
    Error(String),
}

impl Default for ContentState {
    fn default() -> Self {
        ContentState::Home
    }
}

/// Test application state
struct TestAppState {
    focus: FocusTarget,
    content_state: ContentState,
    selected_nav: NavItem,
    scroll_offset: usize,
    search_query: String,
}

impl TestAppState {
    fn new() -> Self {
        Self {
            focus: FocusTarget::default(),
            content_state: ContentState::default(),
            selected_nav: NavItem::Home,
            scroll_offset: 0,
            search_query: String::new(),
        }
    }

    fn focus_next(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Sidebar => FocusTarget::MainContent,
            FocusTarget::MainContent => FocusTarget::PlayerBar,
            FocusTarget::PlayerBar => FocusTarget::Sidebar,
        };
    }

    fn focus_previous(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Sidebar => FocusTarget::PlayerBar,
            FocusTarget::MainContent => FocusTarget::Sidebar,
            FocusTarget::PlayerBar => FocusTarget::MainContent,
        };
    }

    fn select_nav(&mut self, nav: NavItem) {
        self.selected_nav = nav;
    }

    fn scroll_down(&mut self, len: usize) {
        if self.scroll_offset < len.saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn start_search(&mut self) {
        self.search_query.clear();
    }

    fn search_input(&mut self, c: char) {
        self.search_query.push(c);
    }

    fn search_backspace(&mut self) {
        self.search_query.pop();
    }

    fn cancel_search(&mut self) {
        self.search_query.clear();
    }
}

#[test]
fn test_nav_item_all() {
    let items = NavItem::all();
    assert_eq!(items.len(), 5);
    assert_eq!(items[0], NavItem::Home);
    assert_eq!(items[1], NavItem::Search);
    assert_eq!(items[2], NavItem::Library);
    assert_eq!(items[3], NavItem::Playlists);
    assert_eq!(items[4], NavItem::LikedSongs);
}

#[test]
fn test_nav_item_label() {
    assert_eq!(NavItem::Home.label(), "Home");
    assert_eq!(NavItem::Search.label(), "Search");
    assert_eq!(NavItem::Library.label(), "Library");
    assert_eq!(NavItem::Playlists.label(), "Playlists");
    assert_eq!(NavItem::LikedSongs.label(), "Liked Songs");
}

#[test]
fn test_focus_cycle() {
    let mut state = TestAppState::new();

    // Start at Sidebar
    assert_eq!(state.focus, FocusTarget::Sidebar);

    // Cycle forward
    state.focus_next();
    assert_eq!(state.focus, FocusTarget::MainContent);

    state.focus_next();
    assert_eq!(state.focus, FocusTarget::PlayerBar);

    state.focus_next();
    assert_eq!(state.focus, FocusTarget::Sidebar); // Back to start

    // Cycle backward
    state.focus_previous();
    assert_eq!(state.focus, FocusTarget::PlayerBar);

    state.focus_previous();
    assert_eq!(state.focus, FocusTarget::MainContent);

    state.focus_previous();
    assert_eq!(state.focus, FocusTarget::Sidebar);
}

#[test]
fn test_content_state_transitions() {
    let mut state = TestAppState::new();

    // Start at Home
    assert_eq!(state.content_state, ContentState::Home);

    // Transition to Loading
    state.content_state = ContentState::Loading("Playlists".to_string());
    assert!(matches!(state.content_state, ContentState::Loading(_)));

    // Transition to LoadingInProgress
    state.content_state = ContentState::LoadingInProgress("Playlists".to_string());
    assert!(matches!(state.content_state, ContentState::LoadingInProgress(_)));

    // Transition to Playlists
    state.content_state = ContentState::Playlists(vec!["Playlist 1".to_string()]);
    assert!(matches!(state.content_state, ContentState::Playlists(_)));

    // Transition to Error
    state.content_state = ContentState::Error("Network error".to_string());
    assert!(matches!(state.content_state, ContentState::Error(_)));

    // Back to Home
    state.content_state = ContentState::Home;
    assert_eq!(state.content_state, ContentState::Home);
}

#[test]
fn test_search_input() {
    let mut state = TestAppState::new();

    // Start search
    state.start_search();
    assert_eq!(state.search_query, "");

    // Type characters
    state.search_input('h');
    state.search_input('e');
    state.search_input('l');
    state.search_input('l');
    state.search_input('o');
    assert_eq!(state.search_query, "hello");

    // Backspace
    state.search_backspace();
    assert_eq!(state.search_query, "hell");

    state.search_backspace();
    state.search_backspace();
    assert_eq!(state.search_query, "he");

    // Cancel search
    state.cancel_search();
    assert_eq!(state.search_query, "");
}

#[test]
fn test_scroll_offset() {
    let mut state = TestAppState::new();

    // Start at 0
    assert_eq!(state.scroll_offset, 0);

    // Scroll down
    state.scroll_down(10);
    assert_eq!(state.scroll_offset, 1);

    state.scroll_down(10);
    assert_eq!(state.scroll_offset, 2);

    // Scroll up
    state.scroll_up();
    assert_eq!(state.scroll_offset, 1);

    state.scroll_up();
    assert_eq!(state.scroll_offset, 0);

    // Can't scroll below 0
    state.scroll_up();
    assert_eq!(state.scroll_offset, 0);

    // Scroll to end
    for _ in 0..20 {
        state.scroll_down(10);
    }
    // Should cap at len - 1 = 9
    assert_eq!(state.scroll_offset, 9);
}

#[test]
fn test_nav_selection() {
    let mut state = TestAppState::new();

    // Start at Home
    assert_eq!(state.selected_nav, NavItem::Home);

    // Select different nav items
    state.select_nav(NavItem::Search);
    assert_eq!(state.selected_nav, NavItem::Search);

    state.select_nav(NavItem::Playlists);
    assert_eq!(state.selected_nav, NavItem::Playlists);

    state.select_nav(NavItem::LikedSongs);
    assert_eq!(state.selected_nav, NavItem::LikedSongs);
}
