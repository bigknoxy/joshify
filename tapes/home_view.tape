# Joshify Home View Showcase
# Records a GIF demonstration of the home screen
# Usage: vhs tapes/home_view.tape

Output screenshots/home_view.gif
Set Shell "bash"
Set FontSize 14
Set Width 120
Set Height 40
Set Theme "Catppuccin Mocha"

# Start Joshify with mock data in background
Hide
Type "cd /home/josh/projects/joshify && JOSHIFY_MOCK=1 cargo run --release 2>&1 &"
Enter
Sleep 3s
Show

# Capture the initial home view
Sleep 500ms
Screenshot screenshots/home_view.png

# Show we're on the home screen
Sleep 500ms

# Navigate to Library view
Type "l"
Sleep 500ms
Screenshot screenshots/library_view.png

# Show playlists
Type "p"
Sleep 500ms
Screenshot screenshots/playlists_view.png

# Show Liked Songs
Type "k"
Sleep 500ms
Screenshot screenshots/liked_songs_view.png

# Back to home
Type "h"
Sleep 500ms

# Open search overlay
Type "/"
Sleep 200ms
Type "never gonna give you up"
Sleep 1s
Screenshot screenshots/search_overlay.png

# Close search
Type "Escape"
Sleep 200ms

# Show help overlay
Type "?"
Sleep 500ms
Screenshot screenshots/help_overlay.png

# Close help
Type "Escape"
Sleep 200ms

# Show queue overlay
Type "Q"
Sleep 500ms
Screenshot screenshots/queue_overlay.png

# Close queue
Type "Q"
Sleep 200ms

# Quit Joshify
Type "q"
Sleep 500ms

# Cleanup
Hide
Type "pkill -f 'cargo run' || pkill -f joshify"
Enter
Show

# End with final screenshot
Screenshot screenshots/home_view_final.png
