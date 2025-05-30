This is a just a fork of Helix. All claims rest with Helix maintainers and its copyright holders. I am just maintaining a fork. 
For more information please see Helix's readme file. 

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

## Hover Documentation Commands

**Hover Documentation:**
- `Space + k` - Show hover documentation in popup
- `Space + K` - Open hover documentation in navigable buffer (goto_hover)

The `goto_hover` command opens documentation in a new scratch buffer where you can navigate, search, and copy text from long documentation.
