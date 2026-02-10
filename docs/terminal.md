# Terminal Panel Architecture

## Overview

Silicon embeds a built-in terminal panel — a VS Code / Zed-style bottom dock with full terminal emulation powered by `alacritty_terminal` 0.25. This replaces the previous approach of exiting the TUI for interactive shell commands.

## Crate: `silicon-terminal`

### Dependencies

```
silicon-terminal
├── alacritty_terminal 0.25   # Terminal emulation engine
├── silicon-view               # Editor types (KeyEvent, Color, Style, Rect)
├── silicon-tui                # Buffer/Cell rendering primitives
├── tokio                      # Async channel for event notification
├── log                        # Logging
└── libc                       # Unix PTY support
```

### Dependency in the workspace

```
silicon-term → silicon-terminal → silicon-view
                                → silicon-tui
```

`silicon-terminal` does NOT depend on `silicon-term` (no circular dependency). It implements the `Component` trait from `silicon-term` via re-export through `silicon-tui`.

## Module Structure

```
silicon-terminal/src/
├── lib.rs           # Public API re-exports
├── instance.rs      # TerminalInstance: wraps alacritty Term + PTY + EventLoop
├── keys.rs          # KeyEvent → ANSI escape sequence conversion
├── colors.rs        # alacritty Color/Flags → Silicon Color/Style/Modifier
└── panel.rs         # TerminalPanel: multi-tab Component
```

## Data Flow

### Keyboard Input → PTY

```
User keystroke
    │
    ▼
Application::handle_terminal_events()
    │ (if terminal panel is focused)
    ▼
TerminalPanel::handle_key_event()
    │
    ▼
keys::to_esc_str(KeyEvent, TermMode) → Vec<u8>
    │
    ▼
TerminalInstance::input(bytes)
    │
    ▼
Notifier → EventLoopSender → Msg::Input
    │
    ▼
alacritty EventLoop (background thread) → writes to PTY fd
    │
    ▼
Shell process
```

### PTY Output → Screen

```
Shell produces output
    │
    ▼
alacritty EventLoop reads PTY fd → vte::Processor → Term<SiliconListener>
    │
    ▼
SiliconListener::send_event(Event::Wakeup)
    │
    ▼
tokio::sync::mpsc channel → wakes Application event loop
    │
    ▼
TerminalPanel::poll_events() → drains event channel
    │
    ▼
Application::render()
    │
    ▼
TerminalPanel::render(area, surface, ctx)
    │
    ▼
TerminalInstance::render_to_surface()
    │ locks Term via FairMutex
    │ calls term.renderable_content()
    │ iterates cells, converts colors/flags
    ▼
Buffer cells updated with terminal content
    │
    ▼
Terminal backend flushes to screen
```

## Key Components

### TerminalInstance (`instance.rs`)

Wraps a single terminal session:

- `term: Arc<FairMutex<Term<SiliconListener>>>` — alacritty terminal state
- `pty_tx: Notifier` — send input bytes to PTY
- `event_rx: tokio::sync::mpsc::UnboundedReceiver<AlacEvent>` — receive PTY events
- `title: String` — terminal title (from OSC escape sequences)
- `exited: Option<i32>` — exit status when shell exits

Lifecycle:
1. `new(size)` — spawns shell from `$SHELL`, creates PTY, starts event loop thread
2. `input(bytes)` — writes keyboard input to PTY
3. `resize(cols, rows)` — resizes PTY and terminal grid
4. `poll_events()` — drains pending events, updates title/exit status
5. `render_to_surface(area, surface)` — locks term, reads grid, writes to Buffer
6. `scroll(delta)` — scrollback navigation

### TerminalPanel (`panel.rs`)

Multi-tab container implementing `Component`:

- `instances: Vec<TerminalInstance>` — terminal tabs
- `active_tab: usize` — which tab is focused
- `visible: bool` — whether panel is shown
- `height_percent: u16` — panel height as percentage of terminal (default 30%)
- `focused: bool` — whether panel has keyboard focus
- `event_tx: tokio::sync::mpsc::UnboundedSender<()>` — notification channel

### Key Conversion (`keys.rs`)

Converts Silicon's `KeyEvent` to ANSI escape sequences. Considers `TermMode` flags:
- `APP_CURSOR` — application cursor mode (changes arrow key sequences)
- `APP_KEYPAD` — application keypad mode
- `ALT_SCREEN` — alternate screen buffer
- `BRACKETED_PASTE` — bracketed paste mode

### Color Conversion (`colors.rs`)

Maps alacritty types to Silicon types:
- `alacritty Color::Named(n)` → `silicon Color::Black/Red/...` (16 ANSI colors)
- `alacritty Color::Spec(Rgb)` → `silicon Color::Rgb(r, g, b)`
- `alacritty Color::Indexed(i)` → `silicon Color::Indexed(i)`
- `alacritty Flags::BOLD` → `silicon Modifier::BOLD`
- `alacritty Flags::INVERSE` → swap fg/bg

## Integration with Application

### Focus Management

`Ctrl+Backtick` is the sole toggle, cycling through three states:

1. **Panel hidden** → show + focus (spawns terminal if none exist)
2. **Panel visible + focused** → hide + editor focused
3. **Panel visible + editor focused** → focus panel

When focused, ALL keyboard input goes to the PTY. The editor's normal mode, ex commands, and search are completely bypassed.

### Area Splitting

The TerminalPanel is NOT a Compositor layer. The Application splits the area:

```rust
// In Application::render()
if terminal_panel.visible {
    let panel_height = area.height * panel.height_percent / 100;
    let editor_area = Rect { height: area.height - panel_height, ..area };
    let terminal_area = Rect { y: editor_area.bottom(), height: panel_height, ..area };
    compositor.render(editor_area, surface, ctx);
    terminal_panel.render(terminal_area, surface, ctx);
} else {
    compositor.render(area, surface, ctx);
}
```

### Event Loop

New `tokio::select!` arm for terminal events:

```rust
Some(()) = terminal_panel.event_rx.recv() => {
    terminal_panel.poll_events();
    self.render().await;
}
```

### Commands

- `Ctrl+Backtick` — toggle terminal panel visibility/focus
- `Ctrl+Shift+Backtick` — new terminal tab
- `:terminal` — open terminal panel
- `:terminal-new` — new terminal tab
- `:terminal-close` — close active terminal tab
- `:!cmd` — runs command in terminal panel (replaces `InteractiveShellCommand`)

## Performance

- **Lazy initialization**: No PTY spawned until first use — zero startup cost
- **Event batching**: PTY output triggers a single redraw notification
- **FairMutex**: Prevents UI thread starvation when PTY produces heavy output
- **Scrollback**: 10,000 lines default

## Out of Scope

- Hyperlink detection
- Vi mode inside terminal
- Terminal search (Ctrl+F)
- Split terminals within panel
- State persistence across sessions
