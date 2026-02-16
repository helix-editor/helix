use super::*;

pub(super) fn no_op(_cx: &mut Context) {}

pub(super) type MoveFn =
    fn(RopeSlice, Range, Direction, usize, Movement, &TextFormat, &mut TextAnnotations) -> Range;

pub(super) fn move_impl(cx: &mut Context, move_fn: MoveFn, dir: Direction, behaviour: Movement) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let text_fmt = doc.text_format(view.inner_area(doc).width, None);
    let mut annotations = view.text_annotations(doc, None);

    let selection = doc.selection(view.id).clone().transform(|range| {
        move_fn(
            text,
            range,
            dir,
            count,
            behaviour,
            &text_fmt,
            &mut annotations,
        )
    });
    drop(annotations);
    doc.set_selection(view.id, selection);
}

use silicon_core::movement::{move_horizontally, move_vertically};

pub(super) fn move_char_left(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Backward, Movement::Move)
}

pub(super) fn move_char_right(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Forward, Movement::Move)
}

pub(super) fn move_line_up(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Backward, Movement::Move)
}

pub(super) fn move_line_down(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Forward, Movement::Move)
}

pub(super) fn move_visual_line_up(cx: &mut Context) {
    move_impl(
        cx,
        move_vertically_visual,
        Direction::Backward,
        Movement::Move,
    )
}

pub(super) fn move_visual_line_down(cx: &mut Context) {
    move_impl(
        cx,
        move_vertically_visual,
        Direction::Forward,
        Movement::Move,
    )
}

pub(super) fn extend_char_left(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Backward, Movement::Extend)
}

pub(super) fn extend_char_right(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Forward, Movement::Extend)
}

pub(super) fn extend_line_up(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Backward, Movement::Extend)
}

pub(super) fn extend_line_down(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Forward, Movement::Extend)
}

pub(super) fn extend_visual_line_up(cx: &mut Context) {
    move_impl(
        cx,
        move_vertically_visual,
        Direction::Backward,
        Movement::Extend,
    )
}

pub(super) fn extend_visual_line_down(cx: &mut Context) {
    move_impl(
        cx,
        move_vertically_visual,
        Direction::Forward,
        Movement::Extend,
    )
}

pub(super) fn goto_line_end_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_start = text.line_to_char(line);

        let pos = graphemes::prev_grapheme_boundary(text, line_end_char_index(&text, line))
            .max(line_start);

        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

pub(super) fn goto_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_impl(
        view,
        doc,
        if cx.editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

pub(super) fn extend_to_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_impl(view, doc, Movement::Extend)
}

pub(super) fn goto_line_end_newline_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let pos = line_end_char_index(&text, line);

        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

pub(super) fn goto_line_end_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_newline_impl(
        view,
        doc,
        if cx.editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

pub(super) fn extend_to_line_end_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_newline_impl(view, doc, Movement::Extend)
}

pub(super) fn goto_line_start_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        // adjust to start of the line
        let pos = text.line_to_char(line);
        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

pub(super) fn goto_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_start_impl(
        view,
        doc,
        if cx.editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

pub(super) fn goto_next_buffer(cx: &mut Context) {
    goto_buffer(cx.editor, Direction::Forward, cx.count());
}

pub(super) fn goto_previous_buffer(cx: &mut Context) {
    goto_buffer(cx.editor, Direction::Backward, cx.count());
}

pub(super) fn goto_buffer(editor: &mut Editor, direction: Direction, count: usize) {
    let current = view!(editor).doc;

    let id = match direction {
        Direction::Forward => {
            let iter = editor.documents.keys();
            // skip 'count' times past current buffer
            iter.cycle().skip_while(|id| *id != &current).nth(count)
        }
        Direction::Backward => {
            let iter = editor.documents.keys();
            // skip 'count' times past current buffer
            iter.rev()
                .cycle()
                .skip_while(|id| *id != &current)
                .nth(count)
        }
    }
    .unwrap();

    let id = *id;

    editor.switch(id, Action::Replace);
}

pub(super) fn extend_to_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_start_impl(view, doc, Movement::Extend)
}

pub(super) fn goto_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    goto_first_nonwhitespace_impl(
        view,
        doc,
        if cx.editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

pub(super) fn extend_to_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_first_nonwhitespace_impl(view, doc, Movement::Extend)
}

pub(super) fn goto_first_nonwhitespace_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        if let Some(pos) = text.line(line).first_non_whitespace_char() {
            let pos = pos + text.line_to_char(line);
            range.put_cursor(text, pos, movement == Movement::Extend)
        } else {
            range
        }
    });
    doc.set_selection(view.id, selection);
}

pub(super) fn goto_window(cx: &mut Context, align: Align) {
    let count = cx.count() - 1;
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);
    let view_offset = doc.view_offset(view.id);

    let height = view.inner_height();

    // respect user given count if any
    // - 1 so we have at least one gap in the middle.
    // a height of 6 with padding of 3 on each side will keep shifting the view back and forth
    // as we type
    let scrolloff = config.scrolloff.min(height.saturating_sub(1) / 2);

    let last_visual_line = view.last_visual_line(doc);

    let visual_line = match align {
        Align::Top => view_offset.vertical_offset + scrolloff + count,
        Align::Center => view_offset.vertical_offset + (last_visual_line / 2),
        Align::Bottom => {
            view_offset.vertical_offset + last_visual_line.saturating_sub(scrolloff + count)
        }
    };
    let visual_line = visual_line
        .max(view_offset.vertical_offset + scrolloff)
        .min(view_offset.vertical_offset + last_visual_line.saturating_sub(scrolloff));

    let pos = view
        .pos_at_visual_coords(doc, visual_line as u16, 0, false)
        .expect("visual_line was constrained to the view area");

    let text = doc.text().slice(..);
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, cx.editor.mode == Mode::Select));
    doc.set_selection(view.id, selection);
}

pub(super) fn goto_window_top(cx: &mut Context) {
    goto_window(cx, Align::Top)
}

pub(super) fn goto_window_center(cx: &mut Context) {
    goto_window(cx, Align::Center)
}

pub(super) fn goto_window_bottom(cx: &mut Context) {
    goto_window(cx, Align::Bottom)
}

pub(super) fn move_word_impl<F>(cx: &mut Context, move_fn: F)
where
    F: Fn(RopeSlice, Range, usize) -> Range,
{
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| move_fn(text, range, count));
    doc.set_selection(view.id, selection);
}

pub(super) fn move_next_word_start(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_next_word_start)
}

pub(super) fn move_prev_word_start(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_prev_word_start)
}

pub(super) fn move_prev_word_end(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_prev_word_end)
}

pub(super) fn move_next_word_end(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_next_word_end)
}

pub(super) fn move_next_long_word_start(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_next_long_word_start)
}

pub(super) fn move_prev_long_word_start(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_prev_long_word_start)
}

pub(super) fn move_prev_long_word_end(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_prev_long_word_end)
}

pub(super) fn move_next_long_word_end(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_next_long_word_end)
}

pub(super) fn move_next_sub_word_start(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_next_sub_word_start)
}

pub(super) fn move_prev_sub_word_start(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_prev_sub_word_start)
}

pub(super) fn move_prev_sub_word_end(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_prev_sub_word_end)
}

pub(super) fn move_next_sub_word_end(cx: &mut Context) {
    move_word_impl(cx, core_movement::move_next_sub_word_end)
}

pub(super) fn goto_para_impl<F>(cx: &mut Context, move_fn: F)
where
    F: Fn(RopeSlice, Range, usize, Movement) -> Range + 'static,
{
    let count = cx.count();
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);
        let text = doc.text().slice(..);
        let behavior = if editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        };

        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| move_fn(text, range, count, behavior));
        doc.set_selection(view.id, selection);
    };
    cx.editor.apply_motion(motion)
}

pub(super) fn goto_prev_paragraph(cx: &mut Context) {
    goto_para_impl(cx, core_movement::move_prev_paragraph)
}

pub(super) fn goto_next_paragraph(cx: &mut Context) {
    goto_para_impl(cx, core_movement::move_next_paragraph)
}

pub(super) fn goto_file_start(cx: &mut Context) {
    goto_file_start_impl(cx, Movement::Move);
}

pub(super) fn extend_to_file_start(cx: &mut Context) {
    goto_file_start_impl(cx, Movement::Extend);
}

pub(super) fn goto_file_start_impl(cx: &mut Context, movement: Movement) {
    if cx.count.is_some() {
        goto_line_impl(cx, movement);
    } else {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, 0, movement == Movement::Extend));
        push_jump(view, doc);
        doc.set_selection(view.id, selection);
    }
}

pub(super) fn goto_file_end(cx: &mut Context) {
    goto_file_end_impl(cx, Movement::Move);
}

pub(super) fn extend_to_file_end(cx: &mut Context) {
    goto_file_end_impl(cx, Movement::Extend)
}

pub(super) fn goto_file_end_impl(cx: &mut Context, movement: Movement) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let pos = doc.text().len_chars();
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, movement == Movement::Extend));
    push_jump(view, doc);
    doc.set_selection(view.id, selection);
}

pub(super) fn goto_file(cx: &mut Context) {
    goto_file_impl(cx, Action::Replace);
}

pub(super) fn goto_file_hsplit(cx: &mut Context) {
    goto_file_impl(cx, Action::HorizontalSplit);
}

pub(super) fn goto_file_vsplit(cx: &mut Context) {
    goto_file_impl(cx, Action::VerticalSplit);
}

/// Goto files in selection.
pub(super) fn goto_file_impl(cx: &mut Context, action: Action) {
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);
    let selections = doc.selection(view.id);
    let primary = selections.primary();
    let rel_path = doc
        .relative_path()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    let paths: Vec<_> = if selections.len() == 1 && primary.len() == 1 {
        // Cap the search at roughly 1k bytes around the cursor.
        let lookaround = 1000;
        let pos = text.char_to_byte(primary.cursor(text));
        let search_start = text
            .line_to_byte(text.byte_to_line(pos))
            .max(text.floor_char_boundary(pos.saturating_sub(lookaround)));
        let search_end = text
            .line_to_byte(text.byte_to_line(pos) + 1)
            .min(text.ceil_char_boundary(pos + lookaround));
        let search_range = text.byte_slice(search_start..search_end);
        // we also allow paths that are next to the cursor (can be ambiguous but
        // rarely so in practice) so that gf on quoted/braced path works (not sure about this
        // but apparently that is how gf has worked historically in silicon)
        let path = find_paths(search_range, true)
            .take_while(|range| search_start + range.start <= pos + 1)
            .find(|range| pos <= search_start + range.end)
            .map(|range| Cow::from(search_range.byte_slice(range)));
        log::debug!("goto_file auto-detected path: {path:?}");
        let path = path.unwrap_or_else(|| primary.fragment(text));
        vec![path.into_owned()]
    } else {
        // Otherwise use each selection, trimmed.
        selections
            .fragments(text)
            .map(|sel| sel.trim().to_owned())
            .filter(|sel| !sel.is_empty())
            .collect()
    };

    for sel in paths {
        if let Ok(url) = Url::parse(&sel) {
            open_url(cx, url, action);
            continue;
        }

        let path = path::expand(&sel);
        let path = &rel_path.join(path);
        if path.is_dir() {
            let picker = ui::file_picker(cx.editor, path.into());
            cx.push_layer(Box::new(overlaid(picker)));
        } else if let Err(e) = cx.editor.open(path, action) {
            cx.editor.set_error(format!("Open file failed: {:?}", e));
        }
    }
}

/// Opens the given url. If the URL points to a valid textual file it is open in silicon.
//  Otherwise, the file is open using external program.
pub(super) fn open_url(cx: &mut Context, url: Url, action: Action) {
    let doc = doc!(cx.editor);
    let rel_path = doc
        .relative_path()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    if url.scheme() != "file" {
        return cx.jobs.callback(crate::open_external_url_callback(url));
    }

    let content_type = std::fs::File::open(url.path()).and_then(|file| {
        // Read up to 1kb to detect the content type
        let mut read_buffer = Vec::new();
        let n = file.take(1024).read_to_end(&mut read_buffer)?;
        Ok(content_inspector::inspect(&read_buffer[..n]))
    });

    // we attempt to open binary files - files that can't be open in silicon - using external
    // program as well, e.g. pdf files or images
    match content_type {
        Ok(content_inspector::ContentType::BINARY) => {
            cx.jobs.callback(crate::open_external_url_callback(url))
        }
        Ok(_) | Err(_) => {
            let path = &rel_path.join(url.path());
            if path.is_dir() {
                let picker = ui::file_picker(cx.editor, path.into());
                cx.push_layer(Box::new(overlaid(picker)));
            } else if let Err(e) = cx.editor.open(path, action) {
                cx.editor.set_error(format!("Open file failed: {:?}", e));
            }
        }
    }
}

pub(super) fn extend_word_impl<F>(cx: &mut Context, extend_fn: F)
where
    F: Fn(RopeSlice, Range, usize) -> Range,
{
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let word = extend_fn(text, range, count);
        let pos = word.cursor(text);
        range.put_cursor(text, pos, true)
    });
    doc.set_selection(view.id, selection);
}

pub(super) fn extend_next_word_start(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_next_word_start)
}

pub(super) fn extend_prev_word_start(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_prev_word_start)
}

pub(super) fn extend_next_word_end(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_next_word_end)
}

pub(super) fn extend_prev_word_end(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_prev_word_end)
}

pub(super) fn extend_next_long_word_start(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_next_long_word_start)
}

pub(super) fn extend_prev_long_word_start(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_prev_long_word_start)
}

pub(super) fn extend_prev_long_word_end(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_prev_long_word_end)
}

pub(super) fn extend_next_long_word_end(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_next_long_word_end)
}

pub(super) fn extend_next_sub_word_start(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_next_sub_word_start)
}

pub(super) fn extend_prev_sub_word_start(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_prev_sub_word_start)
}

pub(super) fn extend_prev_sub_word_end(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_prev_sub_word_end)
}

pub(super) fn extend_next_sub_word_end(cx: &mut Context) {
    extend_word_impl(cx, core_movement::move_next_sub_word_end)
}

/// Separate branch to find_char designed only for `<ret>` char.
//
// This is necessary because the one document can have different line endings inside. And we
// cannot predict what character to find when <ret> is pressed. On the current line it can be `lf`
// but on the next line it can be `crlf`. That's why [`find_char_impl`] cannot be applied here.
pub(super) fn find_char_line_ending(
    cx: &mut Context,
    count: usize,
    direction: Direction,
    inclusive: bool,
    extend: bool,
) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let cursor = range.cursor(text);
        let cursor_line = range.cursor_line(text);

        // Finding the line where we're going to find <ret>. Depends mostly on
        // `count`, but also takes into account edge cases where we're already at the end
        // of a line or the beginning of a line
        let find_on_line = match direction {
            Direction::Forward => {
                let on_edge = line_end_char_index(&text, cursor_line) == cursor;
                let line = cursor_line + count - 1 + (on_edge as usize);
                if line >= text.len_lines() - 1 {
                    return range;
                } else {
                    line
                }
            }
            Direction::Backward => {
                let on_edge = text.line_to_char(cursor_line) == cursor && !inclusive;
                let line = cursor_line as isize - (count as isize - 1 + on_edge as isize);
                if line <= 0 {
                    return range;
                } else {
                    line as usize
                }
            }
        };

        let pos = match (direction, inclusive) {
            (Direction::Forward, true) => line_end_char_index(&text, find_on_line),
            (Direction::Forward, false) => line_end_char_index(&text, find_on_line) - 1,
            (Direction::Backward, true) => line_end_char_index(&text, find_on_line - 1),
            (Direction::Backward, false) => text.line_to_char(find_on_line),
        };

        if extend {
            range.put_cursor(text, pos, true)
        } else {
            Range::point(range.cursor(text)).put_cursor(text, pos, true)
        }
    });
    doc.set_selection(view.id, selection);
}

pub(super) fn find_char(cx: &mut Context, direction: Direction, inclusive: bool, extend: bool) {
    // TODO: count is reset to 1 before next key so we move it into the closure here.
    // Would be nice to carry over.
    let count = cx.count();

    // need to wait for next key
    // TODO: should this be done by grapheme rather than char?  For example,
    // we can't properly handle the line-ending CRLF case here in terms of char.
    cx.on_next_key(move |cx, event| {
        let ch = match event {
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                find_char_line_ending(cx, count, direction, inclusive, extend);
                return;
            }

            KeyEvent {
                code: KeyCode::Tab, ..
            } => '\t',

            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => ch,
            _ => return,
        };
        let motion = move |editor: &mut Editor| {
            match direction {
                Direction::Forward => {
                    find_char_impl(editor, &find_next_char_impl, inclusive, extend, ch, count)
                }
                Direction::Backward => {
                    find_char_impl(editor, &find_prev_char_impl, inclusive, extend, ch, count)
                }
            };
        };

        cx.editor.apply_motion(motion);
    })
}

//

#[inline]
pub(super) fn find_char_impl<F, M: CharMatcher + Clone + Copy>(
    editor: &mut Editor,
    search_fn: &F,
    inclusive: bool,
    extend: bool,
    char_matcher: M,
    count: usize,
) where
    F: Fn(RopeSlice, M, usize, usize, bool) -> Option<usize> + 'static,
{
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        // TODO: use `Range::cursor()` here instead.  However, that works in terms of
        // graphemes, whereas this function doesn't yet.  So we're doing the same logic
        // here, but just in terms of chars instead.
        let search_start_pos = if range.anchor < range.head {
            range.head - 1
        } else {
            range.head
        };

        search_fn(text, char_matcher, search_start_pos, count, inclusive).map_or(range, |pos| {
            if extend {
                range.put_cursor(text, pos, true)
            } else {
                Range::point(range.cursor(text)).put_cursor(text, pos, true)
            }
        })
    });
    doc.set_selection(view.id, selection);
}

pub(super) fn find_next_char_impl(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    let pos = (pos + 1).min(text.len_chars());
    if inclusive {
        core_search::find_nth_next(text, ch, pos, n)
    } else {
        let n = match text.get_char(pos) {
            Some(next_ch) if next_ch == ch => n + 1,
            _ => n,
        };
        core_search::find_nth_next(text, ch, pos, n).map(|n| n.saturating_sub(1))
    }
}

pub(super) fn find_prev_char_impl(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    if inclusive {
        core_search::find_nth_prev(text, ch, pos, n)
    } else {
        let n = match text.get_char(pos.saturating_sub(1)) {
            Some(next_ch) if next_ch == ch => n + 1,
            _ => n,
        };
        core_search::find_nth_prev(text, ch, pos, n).map(|n| (n + 1).min(text.len_chars()))
    }
}

pub(super) fn find_till_char(cx: &mut Context) {
    find_char(cx, Direction::Forward, false, false);
}

pub(super) fn find_next_char(cx: &mut Context) {
    find_char(cx, Direction::Forward, true, false)
}

pub(super) fn extend_till_char(cx: &mut Context) {
    find_char(cx, Direction::Forward, false, true)
}

pub(super) fn extend_next_char(cx: &mut Context) {
    find_char(cx, Direction::Forward, true, true)
}

pub(super) fn till_prev_char(cx: &mut Context) {
    find_char(cx, Direction::Backward, false, false)
}

pub(super) fn find_prev_char(cx: &mut Context) {
    find_char(cx, Direction::Backward, true, false)
}

pub(super) fn extend_till_prev_char(cx: &mut Context) {
    find_char(cx, Direction::Backward, false, true)
}

pub(super) fn extend_prev_char(cx: &mut Context) {
    find_char(cx, Direction::Backward, true, true)
}

pub(super) fn repeat_last_motion(cx: &mut Context) {
    cx.editor.repeat_last_motion(cx.count())
}

pub fn scroll(cx: &mut Context, offset: usize, direction: Direction, sync_cursor: bool) {
    use Direction::*;
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);
    let mut view_offset = doc.view_offset(view.id);

    let range = doc.selection(view.id).primary();
    let text = doc.text().slice(..);

    let cursor = range.cursor(text);
    let height = view.inner_height();

    let scrolloff = config.scrolloff.min(height.saturating_sub(1) / 2);
    let offset = match direction {
        Forward => offset as isize,
        Backward => -(offset as isize),
    };

    let doc_text = doc.text().slice(..);
    let viewport = view.inner_area(doc);
    let text_fmt = doc.text_format(viewport.width, None);
    (view_offset.anchor, view_offset.vertical_offset) = char_idx_at_visual_offset(
        doc_text,
        view_offset.anchor,
        view_offset.vertical_offset as isize + offset,
        0,
        &text_fmt,
        // &annotations,
        &view.text_annotations(&*doc, None),
    );
    doc.set_view_offset(view.id, view_offset);

    let doc_text = doc.text().slice(..);
    let mut annotations = view.text_annotations(&*doc, None);

    if sync_cursor {
        let movement = match cx.editor.mode {
            Mode::Select => Movement::Extend,
            _ => Movement::Move,
        };
        // TODO: When inline diagnostics gets merged- 1. move_vertically_visual removes
        // line annotations/diagnostics so the cursor may jump further than the view.
        // 2. If the cursor lands on a complete line of virtual text, the cursor will
        // jump a different distance than the view.
        let selection = doc.selection(view.id).clone().transform(|range| {
            move_vertically_visual(
                doc_text,
                range,
                direction,
                offset.unsigned_abs(),
                movement,
                &text_fmt,
                &mut annotations,
            )
        });
        drop(annotations);
        doc.set_selection(view.id, selection);
        return;
    }

    let view_offset = doc.view_offset(view.id);

    let mut head;
    match direction {
        Forward => {
            let off;
            (head, off) = char_idx_at_visual_offset(
                doc_text,
                view_offset.anchor,
                (view_offset.vertical_offset + scrolloff) as isize,
                0,
                &text_fmt,
                &annotations,
            );
            head += (off != 0) as usize;
            if head <= cursor {
                return;
            }
        }
        Backward => {
            head = char_idx_at_visual_offset(
                doc_text,
                view_offset.anchor,
                (view_offset.vertical_offset + height - scrolloff - 1) as isize,
                0,
                &text_fmt,
                &annotations,
            )
            .0;
            if head >= cursor {
                return;
            }
        }
    }

    let anchor = if cx.editor.mode == Mode::Select {
        range.anchor
    } else {
        head
    };

    // replace primary selection with an empty selection at cursor pos
    let prim_sel = Range::new(anchor, head);
    let mut sel = doc.selection(view.id).clone();
    let idx = sel.primary_index();
    sel = sel.replace(idx, prim_sel);
    drop(annotations);
    doc.set_selection(view.id, sel);
}

pub(super) fn page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Backward, false);
}

pub(super) fn page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Forward, false);
}

pub(super) fn half_page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Backward, false);
}

pub(super) fn half_page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Forward, false);
}

pub(super) fn page_cursor_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Backward, true);
}

pub(super) fn page_cursor_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Forward, true);
}

pub(super) fn page_cursor_half_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Backward, true);
}

pub(super) fn page_cursor_half_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Forward, true);
}

pub(super) fn goto_line(cx: &mut Context) {
    goto_line_impl(cx, Movement::Move);
}

pub(super) fn goto_line_impl(cx: &mut Context, movement: Movement) {
    if cx.count.is_some() {
        let (view, doc) = current!(cx.editor);
        push_jump(view, doc);

        goto_line_without_jumplist(cx.editor, cx.count, movement);
    }
}

pub(super) fn goto_line_without_jumplist(
    editor: &mut Editor,
    count: Option<NonZeroUsize>,
    movement: Movement,
) {
    if let Some(count) = count {
        let (view, doc) = current!(editor);
        let text = doc.text().slice(..);
        let max_line = if text.line(text.len_lines() - 1).len_chars() == 0 {
            // If the last line is blank, don't jump to it.
            text.len_lines().saturating_sub(2)
        } else {
            text.len_lines() - 1
        };
        let line_idx = std::cmp::min(count.get() - 1, max_line);
        let pos = text.line_to_char(line_idx);
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, pos, movement == Movement::Extend));

        doc.set_selection(view.id, selection);
    }
}

pub(super) fn goto_last_line(cx: &mut Context) {
    goto_last_line_impl(cx, Movement::Move)
}

pub(super) fn extend_to_last_line(cx: &mut Context) {
    goto_last_line_impl(cx, Movement::Extend)
}

pub(super) fn goto_last_line_impl(cx: &mut Context, movement: Movement) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let line_idx = if text.line(text.len_lines() - 1).len_chars() == 0 {
        // If the last line is blank, don't jump to it.
        text.len_lines().saturating_sub(2)
    } else {
        text.len_lines() - 1
    };
    let pos = text.line_to_char(line_idx);
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, movement == Movement::Extend));

    push_jump(view, doc);
    doc.set_selection(view.id, selection);
}

pub(super) fn goto_column(cx: &mut Context) {
    goto_column_impl(cx, Movement::Move);
}

pub(super) fn extend_to_column(cx: &mut Context) {
    goto_column_impl(cx, Movement::Extend);
}

pub(super) fn goto_column_impl(cx: &mut Context, movement: Movement) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_start = text.line_to_char(line);
        let line_end = line_end_char_index(&text, line);
        let pos = graphemes::nth_next_grapheme_boundary(text, line_start, count - 1).min(line_end);
        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    push_jump(view, doc);
    doc.set_selection(view.id, selection);
}

pub(super) fn goto_last_accessed_file(cx: &mut Context) {
    let view = view_mut!(cx.editor);
    if let Some(alt) = view.docs_access_history.pop() {
        cx.editor.switch(alt, Action::Replace);
    } else {
        cx.editor.set_error("no last accessed buffer")
    }
}

pub(super) fn goto_last_modification(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let pos = doc.history.get_mut().last_edit_pos();
    let text = doc.text().slice(..);
    if let Some(pos) = pos {
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, pos, cx.editor.mode == Mode::Select));
        push_jump(view, doc);
        doc.set_selection(view.id, selection);
    }
}

pub(super) fn goto_last_modified_file(cx: &mut Context) {
    let view = view!(cx.editor);
    let alternate_file = view
        .last_modified_docs
        .into_iter()
        .flatten()
        .find(|&id| id != view.doc);
    if let Some(alt) = alternate_file {
        cx.editor.switch(alt, Action::Replace);
    } else {
        cx.editor.set_error("no last modified buffer")
    }
}

pub(super) fn goto_first_diag(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = match doc.diagnostics().first() {
        Some(diag) => Selection::single(diag.range.start, diag.range.end),
        None => return,
    };
    push_jump(view, doc);
    doc.set_selection(view.id, selection);
    view.diagnostics_handler
        .immediately_show_diagnostic(doc, view.id);
}

pub(super) fn goto_last_diag(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = match doc.diagnostics().last() {
        Some(diag) => Selection::single(diag.range.start, diag.range.end),
        None => return,
    };
    push_jump(view, doc);
    doc.set_selection(view.id, selection);
    view.diagnostics_handler
        .immediately_show_diagnostic(doc, view.id);
}

pub(super) fn goto_next_diag(cx: &mut Context) {
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        let cursor_pos = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));

        let diag = doc
            .diagnostics()
            .iter()
            .find(|diag| diag.range.start > cursor_pos);

        let selection = match diag {
            Some(diag) => Selection::single(diag.range.start, diag.range.end),
            None => return,
        };
        push_jump(view, doc);
        doc.set_selection(view.id, selection);
        view.diagnostics_handler
            .immediately_show_diagnostic(doc, view.id);
    };

    cx.editor.apply_motion(motion);
}

pub(super) fn goto_prev_diag(cx: &mut Context) {
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        let cursor_pos = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));

        let diag = doc
            .diagnostics()
            .iter()
            .rev()
            .find(|diag| diag.range.start < cursor_pos);

        let selection = match diag {
            // NOTE: the selection is reversed because we're jumping to the
            // previous diagnostic.
            Some(diag) => Selection::single(diag.range.end, diag.range.start),
            None => return,
        };
        push_jump(view, doc);
        doc.set_selection(view.id, selection);
        view.diagnostics_handler
            .immediately_show_diagnostic(doc, view.id);
    };
    cx.editor.apply_motion(motion)
}

pub(super) fn goto_first_change(cx: &mut Context) {
    goto_first_change_impl(cx, false);
}

pub(super) fn goto_last_change(cx: &mut Context) {
    goto_first_change_impl(cx, true);
}

pub(super) fn goto_first_change_impl(cx: &mut Context, reverse: bool) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);
    if let Some(handle) = doc.diff_handle() {
        let hunk = {
            let diff = handle.load();
            let idx = if reverse {
                diff.len().saturating_sub(1)
            } else {
                0
            };
            diff.nth_hunk(idx)
        };
        if hunk != Hunk::NONE {
            let range = hunk_range(hunk, doc.text().slice(..));
            push_jump(view, doc);
            doc.set_selection(view.id, Selection::single(range.anchor, range.head));
        }
    }
}

pub(super) fn goto_next_change(cx: &mut Context) {
    goto_next_change_impl(cx, Direction::Forward)
}

pub(super) fn goto_prev_change(cx: &mut Context) {
    goto_next_change_impl(cx, Direction::Backward)
}

pub(super) fn goto_next_change_impl(cx: &mut Context, direction: Direction) {
    let count = cx.count() as u32 - 1;
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);
        let doc_text = doc.text().slice(..);
        let diff_handle = if let Some(diff_handle) = doc.diff_handle() {
            diff_handle
        } else {
            editor.set_status("Diff is not available in current buffer");
            return;
        };

        let selection = doc.selection(view.id).clone().transform(|range| {
            let cursor_line = range.cursor_line(doc_text) as u32;

            let diff = diff_handle.load();
            let hunk_idx = match direction {
                Direction::Forward => diff
                    .next_hunk(cursor_line)
                    .map(|idx| (idx + count).min(diff.len() - 1)),
                Direction::Backward => diff
                    .prev_hunk(cursor_line)
                    .map(|idx| idx.saturating_sub(count)),
            };
            let Some(hunk_idx) = hunk_idx else {
                return range;
            };
            let hunk = diff.nth_hunk(hunk_idx);
            let new_range = hunk_range(hunk, doc_text);
            if editor.mode == Mode::Select {
                let head = if new_range.head < range.anchor {
                    new_range.anchor
                } else {
                    new_range.head
                };

                Range::new(range.anchor, head)
            } else {
                new_range.with_direction(direction)
            }
        });

        push_jump(view, doc);
        doc.set_selection(view.id, selection)
    };
    cx.editor.apply_motion(motion);
}

/// Returns the [Range] for a [Hunk] in the given text.
/// Additions and modifications cover the added and modified ranges.
/// Deletions are represented as the point at the start of the deletion hunk.
pub(super) fn hunk_range(hunk: Hunk, text: RopeSlice) -> Range {
    let anchor = text.line_to_char(hunk.after.start as usize);
    let head = if hunk.after.is_empty() {
        anchor + 1
    } else {
        text.line_to_char(hunk.after.end as usize)
    };

    Range::new(anchor, head)
}

pub(super) fn move_node_bound_impl(cx: &mut Context, dir: Direction, movement: Movement) {
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            let current_selection = doc.selection(view.id);

            let selection = core_movement::move_parent_node_end(
                syntax,
                text,
                current_selection.clone(),
                dir,
                movement,
            );

            doc.set_selection(view.id, selection);
        }
    };

    cx.editor.apply_motion(motion);
}

pub fn move_parent_node_end(cx: &mut Context) {
    move_node_bound_impl(cx, Direction::Forward, Movement::Move)
}

pub fn move_parent_node_start(cx: &mut Context) {
    move_node_bound_impl(cx, Direction::Backward, Movement::Move)
}

pub fn extend_parent_node_end(cx: &mut Context) {
    move_node_bound_impl(cx, Direction::Forward, Movement::Extend)
}

pub fn extend_parent_node_start(cx: &mut Context) {
    move_node_bound_impl(cx, Direction::Backward, Movement::Extend)
}

pub(super) fn match_brackets(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let is_select = cx.editor.mode == Mode::Select;
    let text = doc.text();
    let text_slice = text.slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let pos = range.cursor(text_slice);
        if let Some(matched_pos) = doc.syntax().map_or_else(
            || core_match_brackets::find_matching_bracket_plaintext(text.slice(..), pos),
            |syntax| core_match_brackets::find_matching_bracket_fuzzy(syntax, text.slice(..), pos),
        ) {
            range.put_cursor(text_slice, matched_pos, is_select)
        } else {
            range
        }
    });

    doc.set_selection(view.id, selection);
}

//

pub(super) fn jump_forward(cx: &mut Context) {
    cx.editor.jump_forward(cx.editor.tree.focus, cx.count());
}

pub(super) fn jump_backward(cx: &mut Context) {
    cx.editor.jump_backward(cx.editor.tree.focus, cx.count());
}

pub(super) fn save_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    push_jump(view, doc);
    cx.editor.set_status("Selection saved to jumplist");
}

pub(super) fn scroll_up(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Backward, false);
}

pub(super) fn scroll_down(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Forward, false);
}

pub(super) fn goto_ts_object_impl(cx: &mut Context, object: &'static str, direction: Direction) {
    let count = cx.count();
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);
        let loader = editor.syn_loader.load();
        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            let root = syntax.tree().root_node();

            let selection = doc.selection(view.id).clone().transform(|range| {
                let new_range = core_movement::goto_treesitter_object(
                    text, range, object, direction, &root, syntax, &loader, count,
                );

                if editor.mode == Mode::Select {
                    let head = if new_range.head < range.anchor {
                        new_range.anchor
                    } else {
                        new_range.head
                    };

                    Range::new(range.anchor, head)
                } else {
                    new_range.with_direction(direction)
                }
            });

            push_jump(view, doc);
            doc.set_selection(view.id, selection);
        } else {
            editor.set_status("Syntax-tree is not available in current buffer");
        }
    };
    cx.editor.apply_motion(motion);
}

pub(super) fn goto_next_function(cx: &mut Context) {
    goto_ts_object_impl(cx, "function", Direction::Forward)
}

pub(super) fn goto_prev_function(cx: &mut Context) {
    goto_ts_object_impl(cx, "function", Direction::Backward)
}

pub(super) fn goto_next_class(cx: &mut Context) {
    goto_ts_object_impl(cx, "class", Direction::Forward)
}

pub(super) fn goto_prev_class(cx: &mut Context) {
    goto_ts_object_impl(cx, "class", Direction::Backward)
}

pub(super) fn goto_next_parameter(cx: &mut Context) {
    goto_ts_object_impl(cx, "parameter", Direction::Forward)
}

pub(super) fn goto_prev_parameter(cx: &mut Context) {
    goto_ts_object_impl(cx, "parameter", Direction::Backward)
}

pub(super) fn goto_next_comment(cx: &mut Context) {
    goto_ts_object_impl(cx, "comment", Direction::Forward)
}

pub(super) fn goto_prev_comment(cx: &mut Context) {
    goto_ts_object_impl(cx, "comment", Direction::Backward)
}

pub(super) fn goto_next_test(cx: &mut Context) {
    goto_ts_object_impl(cx, "test", Direction::Forward)
}

pub(super) fn goto_prev_test(cx: &mut Context) {
    goto_ts_object_impl(cx, "test", Direction::Backward)
}

pub(super) fn goto_next_xml_element(cx: &mut Context) {
    goto_ts_object_impl(cx, "xml-element", Direction::Forward)
}

pub(super) fn goto_prev_xml_element(cx: &mut Context) {
    goto_ts_object_impl(cx, "xml-element", Direction::Backward)
}

pub(super) fn goto_next_entry(cx: &mut Context) {
    goto_ts_object_impl(cx, "entry", Direction::Forward)
}

pub(super) fn goto_prev_entry(cx: &mut Context) {
    goto_ts_object_impl(cx, "entry", Direction::Backward)
}

pub(super) fn goto_next_tabstop(cx: &mut Context) {
    goto_next_tabstop_impl(cx, Direction::Forward)
}

pub(super) fn goto_prev_tabstop(cx: &mut Context) {
    goto_next_tabstop_impl(cx, Direction::Backward)
}

pub(super) fn goto_next_tabstop_impl(cx: &mut Context, direction: Direction) {
    let (view, doc) = current!(cx.editor);
    let view_id = view.id;
    let Some(mut snippet) = doc.active_snippet.take() else {
        cx.editor.set_error("no snippet is currently active");
        return;
    };
    let tabstop = match direction {
        Direction::Forward => Some(snippet.next_tabstop(doc.selection(view_id))),
        Direction::Backward => snippet
            .prev_tabstop(doc.selection(view_id))
            .map(|selection| (selection, false)),
    };
    let Some((selection, last_tabstop)) = tabstop else {
        return;
    };
    doc.set_selection(view_id, selection);
    if !last_tabstop {
        doc.active_snippet = Some(snippet)
    }
    if cx.editor.mode() == Mode::Insert {
        cx.on_next_key_fallback(|cx, key| {
            if let Some(c) = key.char() {
                let (view, doc) = current!(cx.editor);
                if let Some(snippet) = &doc.active_snippet {
                    doc.apply(&snippet.delete_placeholder(doc.text()), view.id);
                }
                insert_char(cx, c);
            }
        })
    }
}

pub(super) fn goto_word(cx: &mut Context) {
    jump_to_word(cx, Movement::Move)
}

pub(super) fn extend_to_word(cx: &mut Context) {
    jump_to_word(cx, Movement::Extend)
}

pub(super) fn jump_to_label(cx: &mut Context, labels: Vec<Range>, behaviour: Movement) {
    let doc = doc!(cx.editor);
    let alphabet = &cx.editor.config().jump_label_alphabet;
    if labels.is_empty() {
        return;
    }
    let alphabet_char = |i| {
        let mut res = Tendril::new();
        res.push(alphabet[i]);
        res
    };

    // Add label for each jump candidate to the View as virtual text.
    let text = doc.text().slice(..);
    let mut overlays: Vec<_> = labels
        .iter()
        .enumerate()
        .flat_map(|(i, range)| {
            [
                Overlay::new(range.from(), alphabet_char(i / alphabet.len())),
                Overlay::new(
                    graphemes::next_grapheme_boundary(text, range.from()),
                    alphabet_char(i % alphabet.len()),
                ),
            ]
        })
        .collect();
    overlays.sort_unstable_by_key(|overlay| overlay.char_idx);
    let (view, doc) = current!(cx.editor);
    doc.set_jump_labels(view.id, overlays);

    // Accept two characters matching a visible label. Jump to the candidate
    // for that label if it exists.
    let primary_selection = doc.selection(view.id).primary();
    let view = view.id;
    let doc = doc.id();
    cx.on_next_key(move |cx, event| {
        let alphabet = &cx.editor.config().jump_label_alphabet;
        let Some(i) = event
            .char()
            .filter(|_| event.modifiers.is_empty())
            .and_then(|ch| alphabet.iter().position(|&it| it == ch))
        else {
            doc_mut!(cx.editor, &doc).remove_jump_labels(view);
            return;
        };
        let outer = i * alphabet.len();
        // Bail if the given character cannot be a jump label.
        if outer > labels.len() {
            doc_mut!(cx.editor, &doc).remove_jump_labels(view);
            return;
        }
        cx.on_next_key(move |cx, event| {
            doc_mut!(cx.editor, &doc).remove_jump_labels(view);
            let alphabet = &cx.editor.config().jump_label_alphabet;
            let Some(inner) = event
                .char()
                .filter(|_| event.modifiers.is_empty())
                .and_then(|ch| alphabet.iter().position(|&it| it == ch))
            else {
                return;
            };
            if let Some(mut range) = labels.get(outer + inner).copied() {
                range = if behaviour == Movement::Extend {
                    let anchor = if range.anchor < range.head {
                        let from = primary_selection.from();
                        if range.anchor < from {
                            range.anchor
                        } else {
                            from
                        }
                    } else {
                        let to = primary_selection.to();
                        if range.anchor > to {
                            range.anchor
                        } else {
                            to
                        }
                    };
                    Range::new(anchor, range.head)
                } else {
                    range.with_direction(Direction::Forward)
                };
                save_selection(cx);
                doc_mut!(cx.editor, &doc).set_selection(view, range.into());
            }
        });
    });
}

pub(super) fn jump_to_word(cx: &mut Context, behaviour: Movement) {
    // Calculate the jump candidates: ranges for any visible words with two or
    // more characters.
    let alphabet = &cx.editor.config().jump_label_alphabet;
    if alphabet.is_empty() {
        return;
    }

    let jump_label_limit = alphabet.len() * alphabet.len();
    let mut words = Vec::with_capacity(jump_label_limit);
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);

    // This is not necessarily exact if there is virtual text like soft wrap.
    // It's ok though because the extra jump labels will not be rendered.
    let start = text.line_to_char(text.char_to_line(doc.view_offset(view.id).anchor));
    let end = text.line_to_char(view.estimate_last_doc_line(doc) + 1);

    let primary_selection = doc.selection(view.id).primary();
    let cursor = primary_selection.cursor(text);
    let mut cursor_fwd = Range::point(cursor);
    let mut cursor_rev = Range::point(cursor);
    if text.get_char(cursor).is_some_and(|c| !c.is_whitespace()) {
        let cursor_word_end = core_movement::move_next_word_end(text, cursor_fwd, 1);
        //  single grapheme words need a special case
        if cursor_word_end.anchor == cursor {
            cursor_fwd = cursor_word_end;
        }
        let cursor_word_start = core_movement::move_prev_word_start(text, cursor_rev, 1);
        if cursor_word_start.anchor == next_grapheme_boundary(text, cursor) {
            cursor_rev = cursor_word_start;
        }
    }
    'outer: loop {
        let mut changed = false;
        while cursor_fwd.head < end {
            cursor_fwd = core_movement::move_next_word_end(text, cursor_fwd, 1);
            // The cursor is on a word that is atleast two graphemes long and
            // madeup of word characters. The latter condition is needed because
            // move_next_word_end simply treats a sequence of characters from
            // the same char class as a word so `=<` would also count as a word.
            let add_label = text
                .slice(..cursor_fwd.head)
                .graphemes_rev()
                .take(2)
                .take_while(|g| g.chars().all(char_is_word))
                .count()
                == 2;
            if !add_label {
                continue;
            }
            changed = true;
            // skip any leading whitespace
            cursor_fwd.anchor += text
                .chars_at(cursor_fwd.anchor)
                .take_while(|&c| !char_is_word(c))
                .count();
            words.push(cursor_fwd);
            if words.len() == jump_label_limit {
                break 'outer;
            }
            break;
        }
        while cursor_rev.head > start {
            cursor_rev = core_movement::move_prev_word_start(text, cursor_rev, 1);
            // The cursor is on a word that is atleast two graphemes long and
            // madeup of word characters. The latter condition is needed because
            // move_prev_word_start simply treats a sequence of characters from
            // the same char class as a word so `=<` would also count as a word.
            let add_label = text
                .slice(cursor_rev.head..)
                .graphemes()
                .take(2)
                .take_while(|g| g.chars().all(char_is_word))
                .count()
                == 2;
            if !add_label {
                continue;
            }
            changed = true;
            cursor_rev.anchor -= text
                .chars_at(cursor_rev.anchor)
                .reversed()
                .take_while(|&c| !char_is_word(c))
                .count();
            words.push(cursor_rev);
            if words.len() == jump_label_limit {
                break 'outer;
            }
            break;
        }
        if !changed {
            break;
        }
    }
    jump_to_label(cx, words, behaviour)
}

