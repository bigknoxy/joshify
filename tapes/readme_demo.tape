# Joshify README Demo
# Creates an animated GIF for the README showcasing core features
# Usage: vhs tapes/readme_demo.tape

Output assets/demo.gif
Set Shell "bash"
Set FontSize 14
Set Width 120
Set Height 40
Set Theme "Catppuccin Mocha"

# Start Joshify with mock data in background
Hide
Type "cd /home/josh/projects/joshify && JOSHIFY_MOCK=1 cargo run --release 2>&1 &"
Enter
Sleep 4s
Show

# Capture the beautiful home view
Sleep 500ms

# Navigate to library and playlists
Type "l"
Sleep 400ms
Type "p"
Sleep 400ms

# Show search functionality
Type "/"
Sleep 200ms
Type "blinding lights"
Sleep 800ms
Type "Escape"
Sleep 300ms

# Cycle through themes
Type "T"
Sleep 400ms
Type "T"
Sleep 400ms

# Show queue
Type "Q"
Sleep 500ms
Type "Q"
Sleep 300ms

# Show help overlay
Type "?"
Sleep 800ms
Type "Escape"
Sleep 300ms

# Navigate to a playlist
Type "p"
Sleep 300ms
Enter
Sleep 800ms

# Go back home
Type "Backspace"
Sleep 300ms
Type "h"
Sleep 300ms

# Quit cleanly
Type "q"
Sleep 500ms

# Cleanup
Hide
Type "pkill -f 'cargo run' || pkill -f joshify"
Enter
Sleep 500ms
Show
