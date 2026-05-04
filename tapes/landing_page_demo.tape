# Joshify Landing Page Demo
# Records a GIF showcasing the main interface for the landing page
# Usage: vhs tapes/landing_page_demo.tape

Output docs/landing-page/assets/demo.gif
Set Shell "bash"
Set FontSize 16
Set Width 140
Set Height 45
Set Theme "Catppuccin Mocha"
Set Padding 20

# Start Joshify with mock data in background
Hide
Type "cd /home/josh/projects/joshify && JOSHIFY_MOCK=1 cargo run --release 2>&1 &"
Enter
Sleep 4s
Show

# Capture initial home view with the beautiful UI
Sleep 800ms
Screenshot docs/landing-page/assets/home_view.png

# Show library navigation
Type "l"
Sleep 500ms
Screenshot docs/landing-page/assets/library_view.png

# Show playlists
Type "p"
Sleep 500ms
Screenshot docs/landing-page/assets/playlists_view.png

# Back to home
Type "h"
Sleep 500ms

# Open search overlay
Type "/"
Sleep 300ms
Type "never gonna give you up"
Sleep 1s
Screenshot docs/landing-page/assets/search_view.png

# Close search
Type "Escape"
Sleep 300ms

# Show queue overlay
Type "Q"
Sleep 500ms
Screenshot docs/landing-page/assets/queue_view.png

# Close queue
Type "Q"
Sleep 300ms

# Show help overlay
Type "?"
Sleep 500ms
Screenshot docs/landing-page/assets/help_view.png

# Close help
Type "Escape"
Sleep 300ms

# Cycle through themes to show variety
Type "T"
Sleep 400ms
Type "T"
Sleep 400ms
Type "T"
Sleep 400ms
Type "T"
Sleep 400ms

# Navigate to a playlist and show track list
Type "h"
Sleep 300ms
Type "p"
Sleep 500ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Enter
Sleep 1s
Screenshot docs/landing-page/assets/playlist_tracks.png

# Back and show album view
Type "Backspace"
Sleep 300ms

# Show help one more time with keyboard shortcuts
Type "?"
Sleep 800ms

# Close and quit
Type "Escape"
Sleep 300ms
Type "q"
Sleep 500ms

# Cleanup
Hide
Type "pkill -f 'cargo run' || pkill -f joshify"
Enter
Sleep 500ms
Show

# Final screenshot of empty terminal
Screenshot docs/landing-page/assets/terminal_end.png
