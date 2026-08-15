# README screenshot capture (mock data, no Spotify auth needed)
# Usage: cargo build --release && vhs tapes/readme_screenshots.tape

Output screenshots/reference/demo.gif
Set Shell "bash"
Set FontFamily "JetBrainsMono Nerd Font"
Set FontSize 14
Set Width 1400
Set Height 900
Set Theme "Catppuccin Mocha"
Set Padding 10

Hide
Type "JOSHIFY_MOCK=1 ./target/release/joshify"
Enter
Sleep 4s
Show

# Home view
Sleep 1s
Screenshot screenshots/reference/home_view.png
Sleep 500ms

# Library view (sidebar: j to Library, Enter to open)
Type "j"
Sleep 300ms
Enter
Sleep 1s
Screenshot screenshots/reference/library_view.png
Sleep 500ms

# Liked Songs view
Escape
Sleep 300ms
Type "jj"
Sleep 300ms
Enter
Sleep 1s
Screenshot screenshots/reference/liked_songs.png
Sleep 500ms

# Search overlay
Type "/"
Sleep 300ms
Type@100ms "never gonna"
Sleep 1s
Screenshot screenshots/reference/search_overlay.png
Sleep 500ms
Escape
Sleep 300ms

# Help overlay
Type "?"
Sleep 800ms
Screenshot screenshots/reference/help_overlay.png
Sleep 500ms
Escape
Sleep 300ms

# Back home and quit
Escape
Sleep 300ms
Type "kkk"
Sleep 300ms
Enter
Sleep 800ms
Hide
Type "q"
