This is a just a fork of Helix. All claims rest with Helix maintainers and its copyright holders. I am just maintaining a fork. 
For more information please see Helix's readme file. 

New Features
1. Noice Cmdline
   
   <img width="580" height="424" alt="image" src="https://github.com/user-attachments/assets/a91ef59a-0923-47ab-b35a-2d9cb22a6462" />
   <img width="1395" height="624" alt="image" src="https://github.com/user-attachments/assets/42c01975-b4a0-4d80-ab0e-8171dbf4df72" />


3. Noice Notifications
   
   <img width="713" height="760" alt="image" src="https://github.com/user-attachments/assets/cf78a977-1fd1-421f-acb2-fe359791b3d9" />

3. Cutomizable Picker border changes (now supports the gradients and the border thickness)

   <img width="1916" height="980" alt="image" src="https://github.com/user-attachments/assets/abfeef4a-e633-489f-9236-3e9adbad05bb" />

4. Show function name in the status bar
   Add this following to your editor.statusline config 
   <img width="428" height="220" alt="image" src="https://github.com/user-attachments/assets/dea63278-a649-4561-a29a-b1c0eddacf9f" />


   

This fork of Helix implements the following pull requests. Future pull requests that are merged will be merged and listed here.
1. https://github.com/helix-editor/helix/pull/13354 (index command)
2. https://github.com/helix-editor/helix/pull/13053 (local search in buffer)
3. https://github.com/helix-editor/helix/pull/12369 (basic support for icons)
4. https://github.com/helix-editor/helix/pull/13306 (customizable color swatches)
5. https://github.com/helix-editor/helix/pull/13430 (showing vertical preview)
6. https://github.com/helix-editor/helix/pull/11497 (support for rounded corners)
7. https://github.com/helix-editor/helix/pull/13197 (welcome screen)
8. https://github.com/helix-editor/helix/pull/12520 (picker titles)
9. https://github.com/helix-editor/helix/pull/12173 (buffer click)
10. https://github.com/helix-editor/helix/pull/7988 (inactive panes background color)
11. https://github.com/helix-editor/helix/pull/8546 (flex resize, focus mode) - updated with some of my code.
12. https://github.com/helix-editor/helix/pull/12208 (goto hover command)
13. https://github.com/helix-editor/helix/pull/13113 (add file path to the file names for similar file names)
14. https://github.com/helix-editor/helix/pull/12574 (remove code column from diagnotics buffer)
15. https://github.com/helix-editor/helix/pull/9875 (add code file picker)
16. https://github.com/helix-editor/helix/pull/14121 (move lines - no more macros to move lines)
17. https://github.com/helix-editor/helix/pull/14072 (auto-scrolling bufferline)
18. https://github.com/helix-editor/helix/pull/13821 (fix block cursor in terminal)
19. https://github.com/helix-editor/helix/pull/13760 (support workspace commands)
20.  https://github.com/helix-editor/helix/pull/13988 (add support to swap splits)
21. https://github.com/helix-editor/helix/pull/13133 (Inline Git Blame - show commit info for current line)
22. https://github.com/helix-editor/helix/pull/14453 (ruler chars)
23. https://github.com/helix-editor/helix/pull/12902 (File Explorer: create/delete/copy/move/rename)
24. https://github.com/helix-editor/helix/pull/13963 (auto reload buffers on focus / changes)
25. https://github.com/helix-editor/helix/pull/14825 (soft-wrap indicators in gutter)
26. https://github.com/helix-editor/helix/pull/11695 (Add ability to match in next and previous char pairs)
27. https://github.com/helix-editor/helix/pull/14593 (code folding)
28. https://github.com/helix-editor/helix/pull/12043 (6 new case conversion commands: PascalCase, camelCase, snake_case, Title Case, kebab-case, Sentence case)

## Setup

[Docs](https://docs.helix-editor.com/building-from-source.html) on build it from source.
Summary for commands:

```bash
git clone https://github.com/gj1118/helix.git
cd helix
cargo install --path helix-term --locked

```

Or if you have `just` installed, you can simply run `just` recipes in the folder where you checked the source code. For more information please see the `justfile` 

THEN THE IMPORTANT PART that is missing from the docs page(this way you get all the default themes and gramars), [source](https://github.com/helix-editor/helix/discussions/7549#discussioncomment-6948696)
```bash
cp -R runtime ~/.config/helix/
hx -g fetch
hx -g build
```

**Full Height Mode:**

When `use-full-height = true` is set along with `style = "popup"`, the command line popup uses the full terminal height by removing the traditionally reserved bottom line.

```toml
# Maximum screen space (recommended for popup style)
[editor.cmdline]
style = "popup"
use-full-height = true

# Traditional with reserved space (default)
[editor.cmdline]
style = "popup"
use-full-height = false
```

## Window Resizing and Focus Mode Commands

**Window Resizing:**
- `Alt+w h` or `Alt+w left` - Shrink window width
- `Alt+w l` or `Alt+w right` - Grow window width  
- `Alt+w j` or `Alt+w down` - Shrink window height
- `Alt+w k` or `Alt+w up` - Grow window height

**Focus Mode:**
- `Alt+w f` - Toggle focus mode (expands current window)

**Sticky Mode:**
Activate sticky mode with `Alt+W` (Alt + Shift + w), then use single keys for repeated resizing:
- `h` or `left` - Shrink width
- `l` or `right` - Grow width  
- `j` or `down` - Shrink height
- `k` or `up` - Grow height
- `f` - Toggle focus mode

Sticky mode stays active until you press a key that's not part of the window resize commands.

**Window Resizing Configuration:**

Configure panel resizing limits in your `config.toml`:

```toml
[editor]
# Absolute maximum limits (in terminal character units)
max-panel-width = 50      # Set to 0 for dynamic limit based on terminal size
max-panel-height = 50     # Set to 0 for dynamic limit based on terminal size

# Percentage-based limits (used when absolute limits are set to 0)
max-panel-width-percent = 0.8   # 80% of terminal width (0.0-1.0)
max-panel-height-percent = 0.8  # 80% of terminal height (0.0-1.0)
```

**Configuration Examples:**

```toml
# Conservative: limit panels to 60% of terminal size
[editor]
max-panel-width = 0
max-panel-height = 0
max-panel-width-percent = 0.6
max-panel-height-percent = 0.6

# Aggressive: allow panels up to 95% of terminal size
[editor]
max-panel-width-percent = 0.95
max-panel-height-percent = 0.95

# Hybrid: absolute width limit, percentage height limit
[editor]
max-panel-width = 100
max-panel-height = 0
max-panel-height-percent = 0.8
```

**Benefits:**
- Prevents performance issues with very large panels
- Automatically adapts to your terminal size
- Smooth resizing throughout the entire range
- Configurable limits for different workflows

### Ruler Character

Choose the character used to render rulers in the foreground (defaults to `┊`).
Set it to an empty string to fall back to background-style rulers.

```toml
[editor]
rulers = [80, 100, 120]
ruler-char = "┊"   # examples: "┊", "│", ".", "|"; set to "" for background style
```

## Hover Documentation Commands

**Hover Documentation:**
- `Space + k` - Show hover documentation in popup
- `Space + K` - Open hover documentation in navigable buffer (goto_hover)

The `goto_hover` command opens documentation in a new scratch buffer where you can navigate, search, and copy text from long documentation.

## Customizable Color Swatches

**Color Swatches Configuration:**

Configure color swatches appearance in your `config.toml`:

```toml
[editor.lsp]
# Enable/disable color swatches display (default: true)
display-color-swatches = true

# Customize the color swatch symbol (default: "■")
color-swatches-string = "●"
```

**Configuration Examples:**

```toml
# Circle symbols
[editor.lsp]
color-swatches-string = "●"

# Diamond symbols
[editor.lsp]
color-swatches-string = "◆"

# Hexagon symbols
[editor.lsp]
color-swatches-string = "⬢"

# Alternative hexagon
[editor.lsp]
color-swatches-string = "⬣"

# Default square (explicit)
[editor.lsp]
color-swatches-string = "■"
```

Color swatches appear next to color values in your code (CSS, configuration files, etc.) when LSP support is available, making it easier to visualize colors at a glance.

## Line Movement Commands

**Move Lines Up/Down:**
- `Ctrl+k` - Move current line or selected lines up
- `Ctrl+j` - Move current line or selected lines down

The line movement feature allows you to easily move the current line or multiple selected lines up and down in your document. This works with:
- Single line: When cursor is on a line, moves that entire line
- Multiple selections: Moves all selected lines while preserving their relative positions
- Discontinuous selections: Handles multiple separate line selections correctly
- Unicode content: Properly handles files with Unicode characters

## Noice.nvim-like Command Line (Cmdline)

**Command Line Popup Configuration:**

This fork includes a modern, noice.nvim-inspired command line with customizable icons and popup-style interface.

```toml
[editor.cmdline]
# Command line style: "bottom" (default) or "popup" (noice.nvim style)
style = "popup"

# Show command type icons (default: true)
show-icons = true

# Popup dimensions
min-popup-width = 40    # Minimum width for popup cmdline
max-popup-width = 80    # Maximum width for popup cmdline

# Use full height when style is popup (removes bottom space, default: false)
# Only applies when style = "popup"
use-full-height = true

# Customize command icons
[editor.cmdline.icons]
search = "🔍"    # For search commands (/,?)
command = "⚙"    # For command mode (:)
shell = "⚡"      # For shell commands (!)
general = "💬"   # For other prompts
```

**Icon Theme Examples:**

```toml
# Minimalist ASCII Style
[editor.cmdline.icons]
search = "/"
command = ":"
shell = "$"
general = ">"

# Nerd Font Icons
[editor.cmdline.icons]
search = ""    # nf-fa-search
command = ""    # nf-fa-cog
shell = ""     # nf-fa-terminal
general = ""   # nf-fa-comment

# Fun Emoji Theme
[editor.cmdline.icons]
search = "🔎"
command = "🛠️"
shell = "🖥️"
general = "📝"

# Disable all icons
[editor.cmdline]
show-icons = false
```

**Features:**
- **Popup-style command line** - Centered floating window instead of bottom line
- **Command type icons** - Visual indicators for different command types
- **Enhanced completion** - Better visual feedback and layout
- **Customizable appearance** - Full control over icons and dimensions
- **Backward compatibility** - Traditional bottom style still available

## Noice-style Notifications

**Notifications Configuration:**

This fork includes a modern notification system inspired by noice.nvim, displaying messages as popup windows instead of in the statusline.

```toml
[editor.notifications]
enable = true              # Enable notification system
style = "popup"            # "popup" (noice-style) or "statusline" (traditional)
position = "top-right"     # Position: "top-right", "top-left", "bottom-right", "bottom-left"
max-width = 60             # Maximum width for notification popups
max-height = 10            # Maximum height for notification popups
default-timeout = 5000     # Default timeout in milliseconds (0 = no timeout)
show-icons = true          # Show notification type icons
show-emojis = true         # Show notification type emojis
max-history = 100          # Maximum notifications to keep in history

# Border configuration
[editor.notifications.border]
enable = true              # Show borders around notifications
width = 1                  # Border width
radius = 0                 # Corner radius (0 for square corners)

# Customize notification icons
[editor.notifications.icons]
info = "i"
warning = "!"
error = "x"
success = "ok"

# Customize notification emojis
[editor.notifications.emojis]
info = "info"
warning = "warn"
error = "err"
success = "ok"
```

**Styling Notifications:**

Notification appearance is controlled via theme settings. If notifications look poor with your theme, add these to your theme file:

```toml
# In ~/.config/helix/themes/my-theme.toml
inherits = "your-base-theme"

# Popup background and border
"ui.popup" = { bg = "#1a1a1a", fg = "#ffffff" }
"ui.popup.border" = { fg = "#888888" }

# Notification text colors by severity
"error" = { fg = "#ff5555" }
"warning" = { fg = "#ffaa00" }
"info" = { fg = "#55aaff" }
"hint" = { fg = "#888888" }
```

**Commands:**
- `:notifications-history` - Show notification history
- `:notifications-clear` - Clear notification history
- `:notifications-dismiss` - Dismiss all active notifications

**Features:**
- **Popup-style notifications** - Non-intrusive floating windows
- **Severity-based styling** - Different colors for errors, warnings, info, hints
- **Configurable position** - Place notifications where you prefer
- **Auto-dismiss** - Notifications fade out after timeout
- **History** - Review past notifications with `:notifications-history`

## Aesthetic Gradient Borders

**Gradient Borders Configuration:**

Transform your Helix interface with beautiful, configurable gradient borders for all pickers and UI components.

```toml
[editor.gradient-borders]
enable = true                    # Enable/disable gradient borders
thickness = 2                   # Border thickness (1-5)
direction = "horizontal"        # "horizontal", "vertical", "diagonal", "radial"
start-color = "#8A2BE2"        # Start color (hex format)
end-color = "#00BFFF"          # End color (hex format)
middle-color = "#FF69B4"       # Optional middle color for 3-color gradients
animation-speed = 3            # Animation speed (0-10, 0 = disabled)
```

**Aesthetic Theme Examples:**

```toml
# Cyberpunk Theme
[editor.gradient-borders]
enable = true
thickness = 2
direction = "horizontal"
start-color = "#FF0080"        # Hot Pink
end-color = "#00FFFF"          # Cyan
animation-speed = 2

# Sunset Theme
[editor.gradient-borders]
enable = true
thickness = 3
direction = "diagonal"
start-color = "#FF4500"        # Orange Red
middle-color = "#FFD700"       # Gold
end-color = "#FF69B4"          # Hot Pink
animation-speed = 1

# Ocean Wave
[editor.gradient-borders]
enable = true
thickness = 2
direction = "vertical"
start-color = "#00CED1"        # Dark Turquoise
end-color = "#4169E1"          # Royal Blue
animation-speed = 4

# Matrix Style
[editor.gradient-borders]
enable = true
thickness = 1
direction = "radial"
start-color = "#00FF00"        # Lime Green
end-color = "#008000"          # Dark Green
animation-speed = 5

# Minimalist (No Animation)
[editor.gradient-borders]
enable = true
thickness = 1
direction = "horizontal"
start-color = "#6A5ACD"        # Slate Blue
end-color = "#9370DB"          # Medium Purple
animation-speed = 0
```

**Border Thickness Styles:**
- **1**: Thin Unicode lines (─│┌┐└┘ square, ─│╭╮╰╯ rounded)
- **2**: Thick Unicode lines (━┃┏┓┗┛)
- **3**: Double Unicode lines (═║╔╗╚╝)
- **4**: Block characters (█ style)
- **5**: Full block characters

**Rounded Corners Support:**
Gradient borders automatically respect your existing `rounded_corners` setting:

```toml
[editor]
# Enable rounded corners for all borders (traditional and gradient)
rounded-corners = true

[editor.gradient-borders]
enable = true
thickness = 1    # Thin borders work best with rounded corners
direction = "horizontal"
start-color = "#6A5ACD"
end-color = "#9370DB"
```

- **Thickness 1**: Full rounded corner support (╭╮╰╯)
- **Thickness 2+**: Uses square corners (no Unicode rounded equivalents)
- **Block styles**: Rounded corners don't apply to block characters

**Features:**
- **Applied to all components**: Pickers, command line popups, completion menus, preview panels
- **Dynamic gradients**: Smooth color transitions across any direction
- **Animation support**: Animated gradients with configurable speed
- **Configurable thickness**: From thin lines to chunky block borders
- **Multiple directions**: Horizontal, vertical, diagonal, and radial patterns
- **3-color gradients**: Optional middle color for more complex gradients
- **Performance optimized**: Efficient rendering with minimal overhead

**Note**: Gradient borders are applied to file pickers, command palettes, completion menus, preview panels, and the noice.nvim-style command line popup. Traditional borders are used when gradient borders are disabled.

**Local Development on MacOS**:

To run the app locally on MacOS systems, run the following command on the terminal:
`xattr -d com.apple.quarantine /path/to/your/app`
(this removes the quarantine attribute)

## Inline Git Blame

**Inline Blame Configuration:**

Show git blame information as virtual text next to the current line you're editing. This feature displays the latest commit information for the line your cursor is on.

```toml
[editor]
# Inline blame configuration (inline table form)
inline-blame = { show = "cursor-line", format = "{author} • {time-ago} • {title}", auto-fetch = false }
```

Or in expanded format:

```toml
[editor.inline-blame]
# Show inline blame on specific lines (default: "never")
# Options: "cursor-line", "all-lines", "never"
show = "cursor-line"

# Format string for blame display
# Available placeholders: {author}, {commit}, {time-ago}, {title}
format = "{author} • {time-ago} • {title}"

# Auto-fetch blame information (default: false)
auto-fetch = false
```

**Keybindings:**

- `<space>B` - Show git blame for current line in status line (manual blame)

**Configuration Examples:**

```toml
# Minimal blame display
[editor.inline-blame]
show = "cursor-line"
format = "{author} • {time-ago}"

# Detailed blame information
[editor.inline-blame]
show = "cursor-line"
format = "{commit} - {author} ({time-ago}): {title}"

# Show blame for all lines (can be noisy)
[editor.inline-blame]
show = "all-lines"
format = "{author}"
auto-fetch = true

# Manual blame only (no inline display, use <space>B)
[editor.inline-blame]
show = "never"
```

**Features:**
- **Virtual text display** - Non-intrusive blame info that doesn't affect text editing
- **Cursor-based** - Shows blame only for the line you're currently on
- **Customizable format** - Control what information is displayed
- **Manual fallback** - Use `<space>B` to check blame without enabling inline display
- **Smart caching** - Efficiently caches blame data to avoid repeated git operations

## Signature Help Position

Control where signature help popups appear:

```toml
[editor.lsp]
# Position signature help above cursor (default)
signature-help-position = "above"

# Or below cursor
signature-help-position = "below"

```

## Auto reload buffers on focus / changes
Implements automatic reloading of buffers when their corresponding files are changed on disk. This behavior can be configured to trigger when the editor window gains focus, at a periodic interval, or both.


Here are some example snippets of configuration for this feature:
```toml

# Disable auto-reloading (default)
[editor]
auto-reload = false

#or
[editor.auto-reload]
focus-gained = false

# Auto-reload on focus
[editor]
auto-reload = true
# or
[editor.auto-reload]
focus-gained = true

# Auto-reload at some periodically at time interval (5 seconds in this example)
[editor.auto-reload]
periodic.enable = true
periodic.interval = 5000

# Of course, you could have it reload on focus and at an interval too:
[editor.auto-reload]
focus-gained = true
periodic.enable = true
periodic.interval = 5000

## File Picker Configuration

**Hide Preview:**

You can choose to hide the file preview in the file picker to save screen space or improve performance.

```toml
[editor.file-picker]
hide-preview = true
```

## Code Folding

**Code Folding Commands:**
- `fold` - Fold text objects
- `unfold` - Unfold text objects
- `toggle_fold` - Toggle fold for the text object at the primary cursor

**Configuration:**
To automatically fold specific text objects when a document is opened:

```toml
[editor]
fold-textobjects = ["comment"] # Example: fold all comments on open
```
