# Inline Completion Ghost Text Implementation

## Overview

This document describes the implementation of inline completion (ghost text) rendering in Helix, which integrates with the text annotation system to properly coexist with diagnostics and other virtual text.

## Problem Statement

The original ghost text implementation drew directly on the surface after `render_document`, which caused several issues:
1. Overwrote diagnostic text and other virtual text
2. Didn't integrate with the document rendering pipeline
3. Caused visual conflicts with EOL diagnostics

## Solution Architecture

The solution uses Helix's existing annotation and decoration systems:

### For Mid-Line Ghost Text (cursor NOT at end of line)
- **Overlay**: First ghost character replaces the character under the block cursor visually
- **InlineAnnotation**: Remaining ghost text is inserted at `cursor + 1`, shifting content (including diagnostics)

### For End-of-Line Ghost Text (cursor at newline)
- **Decoration**: Renders ghost text at EOL position via `render_virt_lines`
- Returns column offset so subsequent decorations (diagnostics) shift accordingly
- Avoids using Overlay on newline character (which would join lines)

### For Multi-Line Ghost Text
- **LineAnnotation**: Reserves virtual line space for additional lines
- **Decoration**: Renders additional lines in the reserved virtual space

## File Changes

### 1. `helix-view/src/document.rs`

Added fields to `InlineCompletion` struct:
```rust
pub struct InlineCompletion {
    pub ghost_text: String,
    pub replace_range: Range,
    pub cursor_char_idx: usize,
    pub first_char_overlay: Option<Overlay>,        // First char (mid-line only)
    pub rest_of_line_annotation: Option<InlineAnnotation>, // Rest of first line (mid-line only)
    pub eol_ghost_text: Option<String>,             // First line when at EOL
    pub additional_lines: Vec<String>,              // Multi-line support
}
```

Added annotation caches to `Document`:
```rust
pub inline_completion_overlay: Vec<Overlay>,
pub inline_completion_annotations: Vec<InlineAnnotation>,
```

Added `rebuild_annotations()` method to `InlineCompletions` for cache management.

### 2. `helix-term/src/handlers/inline_completion.rs`

Updated completion processing:
- Detects if cursor is at EOL: `text.get_char(cursor).is_none_or(|c| c == '\n')`
- **Mid-line**: Creates Overlay for first char, InlineAnnotation for rest
- **EOL**: Sets `eol_ghost_text` for Decoration rendering (no annotations)
- Expands tabs and splits into lines for multi-line support
- Calls `rebuild_annotations()` after pushing completions

Added `OnModeSwitch` hook to clear completions when leaving insert mode.

### 3. `helix-view/src/view.rs`

In `text_annotations()`:
- Adds overlay for first ghost char (mid-line case)
- Adds inline annotation for rest of first line (mid-line case)
- Adds `InlineCompletionLines` LineAnnotation for multi-line virtual line reservation

### 4. `helix-view/src/annotations/inline_completion.rs` (NEW)

`InlineCompletionLines` implementing `LineAnnotation`:
- Reserves virtual lines for additional ghost text lines
- Only activates on cursor's document line

### 5. `helix-term/src/ui/text_decorations/inline_completion.rs` (NEW)

`InlineCompletionDecoration` implementing `Decoration`:
- Renders `eol_ghost_text` at end of current line (EOL case)
- Renders `additional_lines` in virtual line space (multi-line)
- Returns column offset (ghost text width) so diagnostics shift

### 6. `helix-term/src/ui/editor.rs`

- Removed manual ghost text drawing code
- Added `InlineCompletionDecoration` to decoration manager

### 7. `helix-term/src/commands.rs`

Updated `inline_completion_next` and `inline_completion_prev` to call `rebuild_annotations()`.

## Visual Behavior

### Mid-Line Case
```
Before: hello[w]orld  error    (block cursor on 'w', diagnostic at EOL)
After:  hello[G]HOSTorld  error   ('G' overlays 'w', 'HOST' inserted, diagnostic shifted)
```

### End-of-Line Case
```
Before: hello|  error         (cursor at EOL, diagnostic after)
After:  hello|GHOST  error    (ghost text at EOL, diagnostic shifted)
```
Note: At EOL, ghost text appears AT cursor position but doesn't overlay (no char to overlay).

### Multi-Line Case
```
Before: hello|  error
        world

After:  hello|GHOST  error    (first line + shifted diagnostic)
        MORE GHOST            (virtual line 1)
        EVEN MORE             (virtual line 2)
        world                 (actual next line unchanged)
```

## Key Design Decisions

1. **Why Overlay + InlineAnnotation for mid-line?**
   - Overlay keeps cursor visually in place (replaces char, doesn't shift)
   - InlineAnnotation shifts content including diagnostics

2. **Why Decoration for EOL?**
   - Can't use Overlay on newline (would join lines)
   - InlineAnnotation at EOL shifts cursor position
   - Decoration draws without affecting cursor, returns col offset for diagnostics

3. **Why LineAnnotation + Decoration for multi-line?**
   - LineAnnotation reserves virtual line space (tells formatter how many lines)
   - Decoration renders actual content in that space
   - Same pattern used by InlineDiagnostics

## Known Limitations

1. **Soft-wrapped lines**: Behavior with soft-wrapped lines not extensively tested.

## EOL First Character Cursor Overlay Fix

### Problem
At EOL, the first ghost character was rendering ONE COLUMN TO THE RIGHT of the cursor instead of ON the cursor.

### Root Cause
Newlines are rendered as a space character with width 1 (see `document.rs:301: Grapheme::Newline => " "`). The `line_width` passed to decorations via `virt_off.col` includes this newline width.

For line "im\n":
- 'i' at col 0, 'm' at col 1, '\n' at col 2 (rendered as space)
- Cursor is at col 2 (ON the newline space cell)
- `virt_off.col = 3` (line_width after newline: col 2 + width 1)
- Ghost text was rendering at col 3 → one column to the right of cursor

### Solution
In `helix-term/src/ui/text_decorations/inline_completion.rs`, subtract 1 from `virt_off.col` when rendering EOL ghost text:

```rust
// Render EOL first-line ghost text at end of current line
if let Some(eol_text) = self.eol_ghost_text {
    // Subtract 1 because virt_off.col includes newline width, but cursor is ON the newline cell
    let col_pos = virt_off.col.saturating_sub(1);
    let mut col = renderer.viewport.x + col_pos as u16;
    // ...
}
```

Additionally, to make the first ghost character visually appear "on" the block cursor, we:
1. Pass cursor style from `editor.rs` to `InlineCompletionDecoration`
2. Render the first grapheme with cursor style (typically inverted colors)
3. Render remaining ghost text with normal ghost text style

### Visual Result
```
Before (bug):  im[ ]p ort    (cursor on col 2, 'p' on col 3 - separated)
After (fixed): im[p]ort      ('p' rendered ON cursor cell with cursor style)
```

## Testing Checklist

- [x] Ghost text appears after typing in insert mode
- [x] Ghost text clears when leaving insert mode
- [x] Ghost text clears when document changes
- [x] Diagnostics shift right when ghost text appears (mid-line)
- [x] Diagnostics shift right when ghost text appears (EOL)
- [x] Multi-line ghost text renders correctly
- [x] Additional lines appear below current line
- [x] Cursor doesn't shift at EOL
- [x] Tab characters expanded properly
- [x] Cycling between completions works
- [x] Accepting completion inserts correct text

## Related Files

- `helix-core/src/text_annotations.rs` - Overlay, InlineAnnotation, LineAnnotation traits
- `helix-term/src/ui/text_decorations.rs` - Decoration trait and DecorationManager
- `helix-term/src/ui/text_decorations/diagnostics.rs` - InlineDiagnostics (reference implementation)
- `helix-view/src/annotations/diagnostics.rs` - Diagnostics LineAnnotation (reference)
