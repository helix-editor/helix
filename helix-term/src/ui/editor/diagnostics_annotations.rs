use std::borrow::Cow;
use std::rc::Rc;

use helix_core::diagnostic::Severity;
use helix_core::text_annotations::TextAnnotations;
use helix_core::visual_offset_from_anchor;
use helix_core::SmallVec;
use helix_view::graphics::Rect;
use helix_view::theme::Style;
use helix_view::{Document, Theme, View};

use crate::ui::document::{LineDecoration, LinePos, TextRenderer};

pub fn inline_diagnostics_decorator(
    doc: &Document,
    view: &View,
    viewport: Rect,
    theme: &Theme,
    text_annotations: &TextAnnotations,
) -> Box<dyn LineDecoration> {
    let whole_view_aera = view.area;
    let background = theme.get("ui.virtual.diagnostics");

    let hint = theme.get("hint");
    let info = theme.get("info");
    let warning = theme.get("warning");
    let error = theme.get("error");

    let messages = doc.diagnostic_annotations_messages();

    let text = doc.text().slice(..);
    let text_fmt = doc.text_format(viewport.width, None);

    let mut visual_offsets = Vec::with_capacity(messages.len());
    for message in messages.iter() {
        visual_offsets.push(
            visual_offset_from_anchor(
                text,
                view.offset.anchor,
                message.anchor_char_idx,
                &text_fmt,
                text_annotations,
                viewport.height as usize,
            )
            .map(|x| x.0),
        );
    }

    // Compute the Style for a given severity
    let sev_style = move |sev| match sev {
        Some(Severity::Error) => error,
        // The same is done when highlighting gutters so we do it here too to be consistent.
        Some(Severity::Warning) | None => warning,
        Some(Severity::Info) => info,
        Some(Severity::Hint) => hint,
    };

    // Vectors used when computing the items to display. We declare them here so that they're not deallocated when the
    // closure is done, only when it is dropped, that way calls are don't have to allocate as much.
    let mut stack = Vec::new();
    let mut left = Vec::new();
    let mut center = SmallVec::<[_; 2]>::new();

    let line_decoration = move |renderer: &mut TextRenderer, pos: LinePos| {
        let mut first_message_idx = usize::MAX;
        let mut found_first = false;
        let mut last_message_idx = usize::MAX;

        for (idx, message) in messages.iter().enumerate() {
            if message.line == pos.doc_line {
                if !found_first {
                    first_message_idx = idx;
                    found_first = true;
                }
                last_message_idx = idx;
            }
        }

        // If we found no diagnostic for this position, do nothing.
        if !found_first {
            return;
        }

        // Extract the relevant diagnostics and visual offsets.
        let messages = match messages.get(first_message_idx..=last_message_idx) {
            Some(m) => m,
            None => return,
        };
        let visual_offsets = match visual_offsets.get(first_message_idx..=last_message_idx) {
            Some(v) => v,
            None => return,
        };

        // Used to build a stack of diagnostics and items to use when computing `DisplayItem`
        #[derive(Debug)]
        enum StackItem {
            // Insert `n` spaces
            Space(u16),
            // Two diagnostics are overlapping in their rendering, we'll need to insert a vertical bar
            Overlap,
            // Leave a blank space that needs a style (used when a diagnostic message is empty)
            Blank(Style),
            // A diagnostic and its style (computed from its severity)
            Diagnostic(Rc<String>, Style),
        }

        // Additional items to display to point the messages to the diagnostic's position in the text
        #[derive(Debug)]
        enum DisplayItem {
            Space(u16),
            Static(&'static str, Style, u16),
            String(String, Style, u16),
        }

        stack.clear();
        stack.reserve(
            stack
                .capacity()
                .saturating_sub(messages.len().saturating_mul(2)),
        );
        let mut prev_col = None;
        let mut line_count = 0_u16;

        // Attribution: the algorithm to compute the layout of the symbols and columns here has been
        // originally written by Hugo Osvaldo Barrera, for https://git.sr.ht/~whynothugo/lsp_lines.nvim.
        // At the time of this comment's writing, the commit used is ec98b45c8280e5ef8c84028d4f38aa447276c002.
        //
        // We diverge from the original code in that we don't iterate in reverse since we display at the end of the
        // loop instead of later, which means we don't have the stack problem that `lsp_lines.nvim` has.

        // First we build the stack, inserting `StackItem`s as needed
        for (message, visual_offset) in messages.iter().zip(visual_offsets.iter()) {
            let visual_offset = match visual_offset {
                Some(o) => *o,
                None => continue,
            };

            let style = sev_style(message.severity);

            // First the item to offset the diagnostic's text
            stack.push(match prev_col {
                Some(prev_col) if prev_col != visual_offset.col => StackItem::Space(
                    visual_offset
                        .col
                        .abs_diff(prev_col)
                        // Account for the vertical bars that are inserted to point diagnostics to
                        // their position in the text
                        .saturating_sub(1)
                        .min(u16::MAX as _) as _,
                ),
                Some(_) => StackItem::Overlap,
                None => StackItem::Space(visual_offset.col.min(u16::MAX as _) as _),
            });

            let trimmed = message.message.trim();

            // Then the diagnostic's text
            if trimmed.is_empty() {
                stack.push(StackItem::Blank(style));
            } else {
                stack.push(StackItem::Diagnostic(Rc::clone(&message.message), style));
            }

            prev_col = Some(visual_offset.col);
            line_count = line_count.saturating_add(trimmed.lines().count().min(u16::MAX as _) as _);
        }

        // When several diagnostics are present in the same virtual block, we will start by
        // displaying the last one and go up one at a time
        let mut pos_y = viewport
            .y
            .saturating_add(pos.visual_line)
            .saturating_add(line_count);

        // Then we iterate the stack we just built to find diagnostics
        for (idx, item) in stack.iter().enumerate() {
            let (text, style) = match item {
                StackItem::Diagnostic(text, style) => (text.trim(), *style),
                _ => continue,
            };

            left.clear();
            let mut overlap = false;
            let mut multi = 0;

            // Iterate the stack for this line to find elements on the left.
            let mut peekable = stack[..idx].iter().peekable();
            while let Some(item2) = peekable.next() {
                match item2 {
                    &StackItem::Space(n) if multi == 0 => left.push(DisplayItem::Space(n)),
                    &StackItem::Space(n) => {
                        left.push(DisplayItem::String("─".repeat(n as usize), style, n))
                    }
                    StackItem::Blank(_) => {
                        left.push(DisplayItem::Static(
                            if multi == 0 { "└" } else { "┴" },
                            style,
                            1,
                        ));
                        multi += 1;
                    }
                    StackItem::Diagnostic(_, style) => {
                        // If an overlap follows this, don't add an extra column.
                        if !(matches!(peekable.peek(), Some(StackItem::Overlap))) {
                            left.push(DisplayItem::Static("│", *style, 1));
                        }
                        overlap = false;
                    }
                    StackItem::Overlap => overlap = true,
                }
            }

            let center_symbol = if overlap && multi > 0 {
                "┼─── "
            } else if overlap {
                "├─── "
            } else if multi > 0 {
                "┴─── "
            } else {
                "└─── "
            };

            center.clear();
            center.push(DisplayItem::Static(center_symbol, style, 5));

            // TODO: We can draw on the left side if and only if:
            // a. Is the last one stacked this line.
            // b. Has enough space on the left.
            // c. Is just one line.
            // d. Is not an overlap.

            let lines_offset = text.lines().count() as u16;
            pos_y -= lines_offset;

            // Use `view` since it's the whole outer view instead of just the inner area so that the background
            // is also applied to the gutters and other elements that are not in the editable part of the document
            let diag_area = Rect::new(
                whole_view_aera.x,
                pos_y + 1,
                whole_view_aera.width,
                lines_offset,
            );
            renderer.surface.set_style(diag_area, background);

            for (offset, line) in text.lines().enumerate() {
                let mut pos_x = viewport.x;
                let pos_y = pos_y + 1 + offset as u16;

                for item in left.iter().chain(center.iter()) {
                    let (text, style, width): (Cow<str>, _, _) = match *item {
                        // No need to allocate a string here when we simply want the default
                        // background filled with empty space
                        DisplayItem::Space(n) => {
                            pos_x = pos_x.saturating_add(n);
                            continue;
                        }
                        DisplayItem::Static(s, style, n) => (s.into(), style, n),
                        DisplayItem::String(ref s, style, n) => (s.into(), style, n),
                    };

                    renderer.surface.set_string(pos_x, pos_y, text, style);
                    pos_x = pos_x.saturating_add(width);
                }

                renderer
                    .surface
                    .set_string(pos_x, pos_y, line.trim(), style);

                center.clear();
                // Special-case for continuation lines
                if overlap {
                    center.push(DisplayItem::Static("│", style, 1));
                    center.push(DisplayItem::Space(4));
                } else {
                    center.push(DisplayItem::Space(5));
                }
            }
        }
    };

    Box::new(line_decoration)
}
