use crate::ui::{menu, Markdown, Menu, Popup, PromptEvent};
use crate::{
    compositor::{Component, Context, Event, EventResult},
    handlers::completion::{
        trigger_auto_completion, CompletionItem, CompletionResponse, LspCompletionItem,
        ResolveHandler,
    },
};
use helix_core::snippets::{ActiveSnippet, RenderedSnippet, Snippet};
use helix_core::{self as core, chars, fuzzy::MATCHER, Change, Transaction};
use helix_lsp::{lsp, util, OffsetEncoding};
use helix_view::icons::ICONS;
use helix_view::{
    editor::CompleteAction,
    handlers::lsp::SignatureHelpInvoked,
    theme::{Color, Modifier, Style},
    ViewId,
};
use helix_view::{graphics::Rect, Document, Editor};
use nucleo::{
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
    Config, Utf32Str,
};
use tui::text::Spans;
use tui::{buffer::Buffer as Surface, text::Span};

use std::cmp::Reverse;

impl menu::Item for CompletionItem {
    type Data = Style;

    fn format(&self, dir_style: &Self::Data) -> menu::Row {
        let deprecated = match self {
            CompletionItem::Lsp(LspCompletionItem { item, .. }) => {
                item.deprecated.unwrap_or_default()
                    || item
                        .tags
                        .as_ref()
                        .is_some_and(|tags| tags.contains(&lsp::CompletionItemTag::DEPRECATED))
            }
            CompletionItem::Other(_) => false,
        };

        let label = match self {
            CompletionItem::Lsp(LspCompletionItem { item, .. }) => item.label.as_str(),
            CompletionItem::Other(core::CompletionItem { label, .. }) => label,
        };

        let mut kind = match self {
            CompletionItem::Lsp(LspCompletionItem { item, .. }) => match item.kind {
                Some(lsp::CompletionItemKind::TEXT) => "text".into(),
                Some(lsp::CompletionItemKind::METHOD) => "method".into(),
                Some(lsp::CompletionItemKind::FUNCTION) => "function".into(),
                Some(lsp::CompletionItemKind::CONSTRUCTOR) => "constructor".into(),
                Some(lsp::CompletionItemKind::FIELD) => "field".into(),
                Some(lsp::CompletionItemKind::VARIABLE) => "variable".into(),
                Some(lsp::CompletionItemKind::CLASS) => "class".into(),
                Some(lsp::CompletionItemKind::INTERFACE) => "interface".into(),
                Some(lsp::CompletionItemKind::MODULE) => "module".into(),
                Some(lsp::CompletionItemKind::PROPERTY) => "property".into(),
                Some(lsp::CompletionItemKind::UNIT) => "unit".into(),
                Some(lsp::CompletionItemKind::VALUE) => "value".into(),
                Some(lsp::CompletionItemKind::ENUM) => "enum".into(),
                Some(lsp::CompletionItemKind::KEYWORD) => "keyword".into(),
                Some(lsp::CompletionItemKind::SNIPPET) => "snippet".into(),
                Some(lsp::CompletionItemKind::COLOR) => item
                    .documentation
                    .as_ref()
                    .and_then(|docs| {
                        let text = match docs {
                            lsp::Documentation::String(text) => text,
                            lsp::Documentation::MarkupContent(lsp::MarkupContent {
                                value, ..
                            }) => value,
                        };
                        // Language servers which send Color completion items tend to include a 6
                        // digit hex code at the end for the color. The extra 1 digit is for the '#'
                        text.get(text.len().checked_sub(7)?..)
                    })
                    .and_then(Color::from_hex)
                    .map_or("color".into(), |color| {
                        let icons = ICONS.load();
                        Spans::from(vec![
                            Span::raw("color "),
                            Span::styled(
                                icons.kind().color().glyph().to_string(),
                                Style::default().fg(color),
                            ),
                        ])
                    }),
                Some(lsp::CompletionItemKind::FILE) => "file".into(),
                Some(lsp::CompletionItemKind::REFERENCE) => "reference".into(),
                Some(lsp::CompletionItemKind::FOLDER) => "folder".into(),
                Some(lsp::CompletionItemKind::ENUM_MEMBER) => "enum_member".into(),
                Some(lsp::CompletionItemKind::CONSTANT) => "constant".into(),
                Some(lsp::CompletionItemKind::STRUCT) => "struct".into(),
                Some(lsp::CompletionItemKind::EVENT) => "event".into(),
                Some(lsp::CompletionItemKind::OPERATOR) => "operator".into(),
                Some(lsp::CompletionItemKind::TYPE_PARAMETER) => "type_param".into(),
                Some(kind) => {
                    log::error!("Received unknown completion item kind: {:?}", kind);
                    "".into()
                }
                None => "".into(),
            },
            CompletionItem::Other(core::CompletionItem { kind, .. }) => kind.as_ref().into(),
        };

        let icons = ICONS.load();
        let name = &kind.0[0].content;

        let is_folder = kind.0[0].content == "folder";

        if let Some(icon) = icons.kind().get(name) {
            kind.0[0].content = format!("{}  {name}", icon.glyph()).into();

            if let Some(color) = icon.color() {
                kind.0[0].style = Style::default().fg(color);
            } else if is_folder {
                kind.0[0].style = *dir_style;
            }
        } else {
            kind.0[0].content = format!("{name}").into();
        }

        let label = Span::styled(
            label,
            if deprecated {
                Style::default().add_modifier(Modifier::CROSSED_OUT)
            } else if is_folder {
                *dir_style
            } else {
                Style::default()
            },
        );

        menu::Row::new([menu::Cell::from(label), menu::Cell::from(kind)])
    }
}

/// Wraps a Menu.
pub struct Completion {
    popup: Popup<Menu<CompletionItem>>,
    #[allow(dead_code)]
    trigger_offset: usize,
    filter: String,
    // TODO: move to helix-view/central handler struct in the future
    resolve_handler: ResolveHandler,
}

impl Completion {
    pub const ID: &'static str = "completion";

    pub fn new(editor: &Editor, items: Vec<CompletionItem>, trigger_offset: usize) -> Self {
        let preview_completion_insert = editor.config().preview_completion_insert;
        let replace_mode = editor.config().completion_replace;

        let dir_style = editor.theme.get("ui.text.directory");

        // Then create the menu
        let menu = Menu::new(items, dir_style, move |editor: &mut Editor, item, event| {
            let (view, doc) = current!(editor);

            macro_rules! language_server {
                ($item:expr) => {
                    match editor
                        .language_servers
                        .get_by_id($item.provider)
                    {
                        Some(ls) => ls,
                        None => {
                            editor.set_error("completions are outdated");
                            // TODO close the completion menu somehow,
                            // currently there is no trivial way to access the EditorView to close the completion menu
                            return;
                        }
                    }
                };
            }

            match event {
                PromptEvent::Abort => {}
                PromptEvent::Update if preview_completion_insert => {
                    // Update creates "ghost" transactions which are not sent to the
                    // lsp server to avoid messing up re-requesting completions. Once a
                    // completion has been selected (with tab, c-n or c-p) it's always accepted whenever anything
                    // is typed. The only way to avoid that is to explicitly abort the completion
                    // with c-c. This will remove the "ghost" transaction.
                    //
                    // The ghost transaction is modeled with a transaction that is not sent to the LS.
                    // (apply_temporary) and a savepoint. It's extremely important this savepoint is restored
                    // (also without sending the transaction to the LS) *before any further transaction is applied*.
                    // Otherwise incremental sync breaks (since the state of the LS doesn't match the state the transaction
                    // is applied to).
                    if matches!(editor.last_completion, Some(CompleteAction::Triggered)) {
                        editor.last_completion = Some(CompleteAction::Selected {
                            savepoint: doc.savepoint(view),
                        })
                    }
                    let item = item.unwrap();
                    let context = &editor.handlers.completions.active_completions[&item.provider()];
                    // if more text was entered, remove it
                    doc.restore(view, &context.savepoint, false);
                    // always present here

                    match item {
                        CompletionItem::Lsp(item) => {
                            let (transaction, _) = lsp_item_to_transaction(
                                doc,
                                view.id,
                                &item.item,
                                language_server!(item).offset_encoding(),
                                trigger_offset,
                                replace_mode,
                            );
                            doc.apply_temporary(&transaction, view.id)
                        }
                        CompletionItem::Other(core::CompletionItem { transaction, .. }) => {
                            doc.apply_temporary(transaction, view.id)
                        }
                    };
                }
                PromptEvent::Update => {}
                PromptEvent::Validate => {
                    if let Some(CompleteAction::Selected { savepoint }) =
                        editor.last_completion.take()
                    {
                        doc.restore(view, &savepoint, false);
                    }

                    let item = item.unwrap();
                    let context = &editor.handlers.completions.active_completions[&item.provider()];
                    // if more text was entered, remove it
                    doc.restore(view, &context.savepoint, true);
                    // save an undo checkpoint before the completion
                    doc.append_changes_to_history(view);

                    // item always present here
                    let (transaction, additional_edits, snippet) = match item.clone() {
                        CompletionItem::Lsp(mut item) => {
                            let language_server = language_server!(item);

                            // resolve item if not yet resolved
                            if !item.resolved {
                                if let Some(resolved_item) = Self::resolve_completion_item(
                                    language_server,
                                    item.item.clone(),
                                ) {
                                    item.item = resolved_item;
                                }
                            };

                            let encoding = language_server.offset_encoding();
                            let (transaction, snippet) = lsp_item_to_transaction(
                                doc,
                                view.id,
                                &item.item,
                                encoding,
                                trigger_offset,
                                replace_mode,
                            );
                            let add_edits = item.item.additional_text_edits;

                            (
                                transaction,
                                add_edits.map(|edits| (edits, encoding)),
                                snippet,
                            )
                        }
                        CompletionItem::Other(core::CompletionItem { transaction, .. }) => {
                            (transaction, None, None)
                        }
                    };

                    doc.apply(&transaction, view.id);
                    let placeholder = snippet.is_some();
                    if let Some(snippet) = snippet {
                        doc.active_snippet = match doc.active_snippet.take() {
                            Some(active) => active.insert_subsnippet(snippet),
                            None => ActiveSnippet::new(snippet),
                        };
                    }

                    editor.last_completion = Some(CompleteAction::Applied {
                        trigger_offset,
                        changes: completion_changes(&transaction, trigger_offset),
                        placeholder,
                    });

                    // TODO: add additional _edits to completion_changes?
                    if let Some((additional_edits, offset_encoding)) = additional_edits {
                        if !additional_edits.is_empty() {
                            let transaction = util::generate_transaction_from_edits(
                                doc.text(),
                                additional_edits,
                                offset_encoding, // TODO: should probably transcode in Client
                            );
                            doc.apply(&transaction, view.id);
                        }
                    }
                    // we could have just inserted a trigger char (like a `crate::` completion for rust
                    // so we want to retrigger immediately when accepting a completion.
                    trigger_auto_completion(editor, true);
                }
            };

            // In case the popup was deleted because of an intersection w/ the auto-complete menu.
            if event != PromptEvent::Update {
                editor
                    .handlers
                    .trigger_signature_help(SignatureHelpInvoked::Automatic, editor);
            }
        });

        let popup = Popup::new(Self::ID, menu)
            .with_scrollbar(false)
            .ignore_escape_key(true);

        let (view, doc) = current_ref!(editor);
        let text = doc.text().slice(..);
        let cursor = doc.selection(view.id).primary().cursor(text);
        let offset = text
            .chars_at(cursor)
            .reversed()
            .take_while(|ch| chars::char_is_word(*ch))
            .count();
        let start_offset = cursor.saturating_sub(offset);

        let fragment = doc.text().slice(start_offset..cursor);
        let mut completion = Self {
            popup,
            trigger_offset,
            // TODO: expand nucleo api to allow moving straight to a Utf32String here
            // and avoid allocation during matching
            filter: String::from(fragment),
            resolve_handler: ResolveHandler::new(),
        };

        // need to recompute immediately in case start_offset != trigger_offset
        completion.score(false);

        completion
    }

    fn score(&mut self, incremental: bool) {
        let pattern = &self.filter;
        let mut matcher = MATCHER.lock();
        matcher.config = Config::DEFAULT;
        // slight preference towards prefix matches
        matcher.config.prefer_prefix = true;
        let pattern = Atom::new(
            pattern,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
            false,
        );
        let mut buf = Vec::new();
        let (matches, options) = self.popup.contents_mut().update_options();
        if incremental {
            matches.retain_mut(|(index, score)| {
                let option = &options[*index as usize];
                let text = option.filter_text();
                let new_score = pattern.score(Utf32Str::new(text, &mut buf), &mut matcher);
                match new_score {
                    Some(new_score) => {
                        *score = new_score as u32 / 2;
                        true
                    }
                    None => false,
                }
            })
        } else {
            matches.clear();
            matches.extend(options.iter().enumerate().filter_map(|(i, option)| {
                let text = option.filter_text();
                pattern
                    .score(Utf32Str::new(text, &mut buf), &mut matcher)
                    .map(|score| (i as u32, score as u32 / 3))
            }));
        }
        // Nucleo is meant as an FZF-like fuzzy matcher and only hides matches that are truly
        // impossible - as in the sequence of characters just doesn't appear. That doesn't work
        // well for completions with multiple language servers where all completions of the next
        // server are below the current one (so you would get good suggestions from the second
        // server below those of the first). Setting a reasonable cutoff below which to move bad
        // completions out of the way helps with that.
        //
        // The score computation is a heuristic derived from Nucleo internal constants that may
        // move upstream in the future. I want to test this out here to settle on a good number.
        let min_score = (7 + pattern.needle_text().len() as u32 * 14) / 3;
        matches.sort_unstable_by_key(|&(i, score)| {
            let option = &options[i as usize];
            (
                score <= min_score,
                Reverse(option.preselect()),
                option.provider_priority(),
                Reverse(score),
                i,
            )
        });
    }

    /// Synchronously resolve the given completion item. This is used when
    /// accepting a completion.
    fn resolve_completion_item(
        language_server: &helix_lsp::Client,
        completion_item: lsp::CompletionItem,
    ) -> Option<lsp::CompletionItem> {
        if !matches!(
            language_server.capabilities().completion_provider,
            Some(lsp::CompletionOptions {
                resolve_provider: Some(true),
                ..
            })
        ) {
            return None;
        }
        let future = language_server.resolve_completion_item(&completion_item);
        let response = helix_lsp::block_on(future);
        match response {
            Ok(item) => Some(item),
            Err(err) => {
                log::error!("Failed to resolve completion item: {}", err);
                None
            }
        }
    }

    /// Appends (`c: Some(c)`) or removes (`c: None`) a character to/from the filter
    /// this should be called whenever the user types or deletes a character in insert mode.
    pub fn update_filter(&mut self, c: Option<char>) {
        // recompute menu based on matches
        let menu = self.popup.contents_mut();
        match c {
            Some(c) => self.filter.push(c),
            None => {
                self.filter.pop();
                if self.filter.is_empty() {
                    menu.clear();
                    return;
                }
            }
        }
        self.score(c.is_some());
        self.popup.contents_mut().reset_cursor();
    }

    pub fn replace_provider_completions(
        &mut self,
        response: &mut CompletionResponse,
        is_incomplete: bool,
    ) {
        let menu = self.popup.contents_mut();
        let (_, options) = menu.update_options();
        if is_incomplete {
            options.retain(|item| item.provider() != response.provider)
        }
        response.take_items(options);
        self.score(false);
        let menu = self.popup.contents_mut();
        menu.ensure_cursor_in_bounds();
    }

    pub fn is_empty(&self) -> bool {
        self.popup.contents().is_empty()
    }

    pub fn replace_item(
        &mut self,
        old_item: &impl PartialEq<CompletionItem>,
        new_item: CompletionItem,
    ) {
        self.popup.contents_mut().replace_option(old_item, new_item);
    }

    pub fn area(&mut self, viewport: Rect, editor: &Editor) -> Rect {
        self.popup.area(viewport, editor)
    }
}

impl Component for Completion {
    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        self.popup.handle_event(event, cx)
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        self.popup.required_size(viewport)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        self.popup.render(area, surface, cx);

        // if we have a selection, render a markdown popup on top/below with info
        let option = match self.popup.contents_mut().selection_mut() {
            Some(option) => option,
            None => return,
        };
        if let CompletionItem::Lsp(option) = option {
            self.resolve_handler.ensure_item_resolved(cx.editor, option);
        }
        // need to render:
        // option.detail
        // ---
        // option.documentation

        let Some(coords) = cx.editor.cursor().0 else {
            return;
        };
        let cursor_pos = coords.row as u16;
        let doc = doc!(cx.editor);
        let language = doc.language_name().unwrap_or("");

        let markdowned = |lang: &str, detail: Option<&str>, doc: Option<&str>| {
            let md = match (detail, doc) {
                (Some(detail), Some(doc)) => format!("```{lang}\n{detail}\n```\n{doc}"),
                (Some(detail), None) => format!("```{lang}\n{detail}\n```"),
                (None, Some(doc)) => doc.to_string(),
                (None, None) => String::new(),
            };
            Markdown::new(md, cx.editor.syn_loader.clone())
        };

        let mut markdown_doc = match option {
            CompletionItem::Lsp(option) => match &option.item.documentation {
                Some(lsp::Documentation::String(contents))
                | Some(lsp::Documentation::MarkupContent(lsp::MarkupContent {
                    kind: lsp::MarkupKind::PlainText,
                    value: contents,
                })) => {
                    // TODO: convert to wrapped text
                    markdowned(language, option.item.detail.as_deref(), Some(contents))
                }
                Some(lsp::Documentation::MarkupContent(lsp::MarkupContent {
                    kind: lsp::MarkupKind::Markdown,
                    value: contents,
                })) => {
                    // TODO: set language based on doc scope
                    markdowned(language, option.item.detail.as_deref(), Some(contents))
                }
                None if option.item.detail.is_some() => {
                    // TODO: set language based on doc scope
                    markdowned(language, option.item.detail.as_deref(), None)
                }
                None => return,
            },
            CompletionItem::Other(option) => {
                let Some(doc) = option.documentation.as_deref() else {
                    return;
                };
                markdowned(language, None, Some(doc))
            }
        };

        let popup_area = self.popup.area(area, cx.editor);
        let doc_width_available = area.width.saturating_sub(popup_area.right());
        let doc_area = if doc_width_available > 30 {
            let mut doc_width = doc_width_available;
            let mut doc_height = area.height.saturating_sub(popup_area.top());
            let x = popup_area.right();
            let y = popup_area.top();

            if let Some((rel_width, rel_height)) =
                markdown_doc.required_size((doc_width, doc_height))
            {
                doc_width = rel_width.min(doc_width);
                doc_height = rel_height.min(doc_height);
            }
            Rect::new(x, y, doc_width, doc_height)
        } else {
            // Documentation should not cover the cursor or the completion popup
            // Completion popup could be above or below the current line
            let avail_height_above = cursor_pos.min(popup_area.top()).saturating_sub(1);
            let avail_height_below = area
                .height
                .saturating_sub(cursor_pos.max(popup_area.bottom()) + 1 /* padding */);
            let (y, avail_height) = if avail_height_below >= avail_height_above {
                (
                    area.height.saturating_sub(avail_height_below),
                    avail_height_below,
                )
            } else {
                (0, avail_height_above)
            };
            if avail_height <= 1 {
                return;
            }

            Rect::new(0, y, area.width, avail_height.min(15))
        };

        // clear area
        let background = cx.editor.theme.get("ui.popup");
        surface.clear_with(doc_area, background);

        if cx.editor.popup_border() {
            use tui::widgets::{Block, Widget};
            Widget::render(Block::bordered(), doc_area, surface);
        }

        markdown_doc.render(doc_area, surface, cx);
    }
}
fn lsp_item_to_transaction(
    doc: &Document,
    view_id: ViewId,
    item: &lsp::CompletionItem,
    offset_encoding: OffsetEncoding,
    trigger_offset: usize,
    replace_mode: bool,
) -> (Transaction, Option<RenderedSnippet>) {
    let selection = doc.selection(view_id);
    let text = doc.text().slice(..);
    let primary_cursor = selection.primary().cursor(text);

    let (edit_offset, new_text) = if let Some(edit) = &item.text_edit {
        let edit = match edit {
            lsp::CompletionTextEdit::Edit(edit) => edit.clone(),
            lsp::CompletionTextEdit::InsertAndReplace(item) => {
                let range = if replace_mode {
                    item.replace
                } else {
                    item.insert
                };
                lsp::TextEdit::new(range, item.new_text.clone())
            }
        };

        let Some(range) = util::lsp_range_to_range(doc.text(), edit.range, offset_encoding) else {
            return (Transaction::new(doc.text()), None);
        };

        let start_offset = range.anchor as i128 - primary_cursor as i128;
        let end_offset = range.head as i128 - primary_cursor as i128;

        (Some((start_offset, end_offset)), edit.new_text)
    } else {
        let new_text = item
            .insert_text
            .clone()
            .unwrap_or_else(|| item.label.clone());
        // check that we are still at the correct savepoint
        // we can still generate a transaction regardless but if the
        // document changed (and not just the selection) then we will
        // likely delete the wrong text (same if we applied an edit sent by the LS)
        debug_assert!(primary_cursor == trigger_offset);
        (None, new_text)
    };

    if matches!(item.kind, Some(lsp::CompletionItemKind::SNIPPET))
        || matches!(
            item.insert_text_format,
            Some(lsp::InsertTextFormat::SNIPPET)
        )
    {
        let Ok(snippet) = Snippet::parse(&new_text) else {
            log::error!("Failed to parse snippet: {new_text:?}",);
            return (Transaction::new(doc.text()), None);
        };
        let (transaction, snippet) = util::generate_transaction_from_snippet(
            doc.text(),
            selection,
            edit_offset,
            replace_mode,
            snippet,
            &mut doc.snippet_ctx(),
        );
        (transaction, Some(snippet))
    } else {
        let transaction = util::generate_transaction_from_completion_edit(
            doc.text(),
            selection,
            edit_offset,
            replace_mode,
            new_text,
        );
        (transaction, None)
    }
}

fn completion_changes(transaction: &Transaction, trigger_offset: usize) -> Vec<Change> {
    transaction
        .changes_iter()
        .filter(|(start, end, _)| (*start..=*end).contains(&trigger_offset))
        .collect()
}
