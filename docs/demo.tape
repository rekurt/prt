Output docs/prt.gif

Set Shell "zsh"
Set FontSize 16
Set Width 1280
Set Height 720
Set Padding 16
Set Framerate 30
Set TypingSpeed 35ms
Set Theme "Catppuccin Mocha"

Hide
Type "PRT_LANG=en ./target/release/prt"
Enter
Sleep 4s
Show

# Connections and the unified details panel.
Sleep 2s

# Open the selected process, then show its topology.
Enter
Sleep 2s
Type "]"
Sleep 2s

# Use the searchable command palette to return to Connections.
Type ":"
Sleep 750ms
Type "connections"
Sleep 1s
Enter
Sleep 2s

# Finish on the contextual action menu.
Type " "
Sleep 3s
