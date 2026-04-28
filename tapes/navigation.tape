# Joshify Navigation Demo
# Shows sidebar navigation, breadcrumb navigation, and view transitions
# Usage: vhs tapes/navigation.tape

Output screenshots/navigation.gif
Set Shell "bash"
Set FontSize 14
Set Width 120
Set Height 40
Set Theme "Catppuccin Mocha"

# Start Joshify with mock data
Hide
Type "cd /home/josh/projects/joshify && JOSHIFY_MOCK=1 cargo run --release 2>&1 &"
Enter
Sleep 3s
Show

# Start at home
Screenshot screenshots/nav_home.png
Sleep 500ms

# Navigate through sidebar items
Type "Tab"
Sleep 300ms
Screenshot screenshots/nav_sidebar_focus.png

# Move down sidebar
Type "j"
Sleep 200ms
Screenshot screenshots/nav_library_selected.png

Type "j"
Sleep 200ms
Screenshot screenshots/nav_playlists_selected.png

Type "j"
Sleep 200ms
Screenshot screenshots/nav_liked_songs_selected.png

# Select Liked Songs
Enter
Sleep 500ms
Screenshot screenshots/view_liked_songs.png

# Navigate with Tab back to sidebar
Type "Tab"
Sleep 200ms

# Go to playlists
Type "k"
Sleep 200ms
Enter
Sleep 500ms
Screenshot screenshots/view_playlists.png

# Drill into a playlist
Type "Tab"
Sleep 200ms
Type "j"
Sleep 200ms
Enter
Sleep 500ms
Screenshot screenshots/view_playlist_tracks.png

# Show breadcrumb navigation - go back
Type "Backspace"
Sleep 500ms
Screenshot screenshots/view_back_to_playlists.png

# Back to home
Type "Tab"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Type "k"
Sleep 200ms
Enter
Sleep 500ms
Screenshot screenshots/nav_back_home.png

# Demonstrate help overlay from home
Type "?"
Sleep 500ms
Screenshot screenshots/help_from_home.png

Type "Escape"
Sleep 200ms

# Show search from any screen
Type "/"
Sleep 200ms
Type "queen"
Sleep 1s
Screenshot screenshots/search_from_home.png

Type "Escape"
Sleep 200ms

# Quit
Type "q"
Sleep 500ms

# Cleanup
Hide
Type "pkill -f 'cargo run' || pkill -f joshify"
Enter
Show
