# Joshify Player View Showcase
# Demonstrates the now playing bar with album art and controls
# Usage: vhs tapes/player_view.tape

Output screenshots/player_view.gif
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

# Navigate to a playlist to show player in context
Type "p"
Sleep 500ms

# Capture playlist view
Screenshot screenshots/playlist_browse.png

# Select and play first track (simulated)
Type "j"
Sleep 200ms
Enter
Sleep 1s

# Capture player active state
Screenshot screenshots/player_playing.png

# Show queue overlay
Type "Q"
Sleep 500ms
Screenshot screenshots/queue_with_player.png

# Close queue
Type "Q"
Sleep 200ms

# Toggle play/pause
Type "Space"
Sleep 500ms
Screenshot screenshots/player_paused.png

# Toggle back
Type "Space"
Sleep 200ms

# Cycle through themes
Type "T"
Sleep 500ms
Screenshot screenshots/theme_gruvbox.png

Type "T"
Sleep 500ms
Screenshot screenshots/theme_nord.png

Type "T"
Sleep 500ms
Screenshot screenshots/theme_tokyo_night.png

# Toggle visualizer (if available)
Type "v"
Sleep 1s
Screenshot screenshots/visualizer.png

# Turn off visualizer
Type "v"
Sleep 200ms

# Back to default theme
Type "T"
Sleep 200ms

# Show lyrics (if available)
Type "L"
Sleep 500ms
Screenshot screenshots/lyrics_view.png

# Close lyrics
Type "Escape"
Sleep 200ms

# Navigate back
Type "Backspace"
Sleep 200ms

# Quit
Type "q"
Sleep 500ms

# Cleanup
Hide
Type "pkill -f 'cargo run' || pkill -f joshify"
Enter
Show
