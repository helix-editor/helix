use crate::compositor::{Component, Context, Event, EventResult};
use helix_view::{
    document::SavePoint,
    editor::CompleteAction,
    graphics::Margin,
    theme::{Modifier, Style},
    ViewId,
};
use tui::{buffer::Buffer as Surface, text::Span};

use std::{borrow::Cow, sync::Arc};

use helix_core::{Change, Transaction};
use helix_view::{graphics::Rect, Document, Editor};

use crate::commands;
use crate::ui::{menu, Markdown, Menu, Popup, PromptEvent};

use helix_lsp::{lsp, util, OffsetEncoding};

impl menu::Item for CompletionItem {
    type Data = ();
    fn sort_text(&self, data: &Self::Data) -> Cow<str> {
        self.filter_text(data)
    }

    #[inline]
    fn filter_text(&self, _data: &Self::Data) -> Cow<str> {
        self.item
            .filter_text
            .as_ref()
            .unwrap_or(&self.item.label)
            .as_str()
            .into()
    }

    fn format(&self, _data: &Self::Data) -> menu::Row {
        let deprecated = self.item.deprecated.unwrap_or_default()
            || self.item.tags.as_ref().map_or(false, |tags| {
                tags.contains(&lsp::CompletionItemTag::DEPRECATED)
            });

        menu::Row::new(vec![
            menu::Cell::from(Span::styled(
                self.item.label.as_str(),
                if deprecated {
                    Style::default().add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default()
                },
            )),
            menu::Cell::from(match self.item.kind {
                Some(lsp::CompletionItemKind::TEXT) => "text",
                Some(lsp::CompletionItemKind::METHOD) => "method",
                Some(lsp::CompletionItemKind::FUNCTION) => "function",
                Some(lsp::CompletionItemKind::CONSTRUCTOR) => "constructor",
                Some(lsp::CompletionItemKind::FIELD) => "field",
                Some(lsp::CompletionItemKind::VARIABLE) => "variable",
                Some(lsp::CompletionItemKind::CLASS) => "class",
                Some(lsp::CompletionItemKind::INTERFACE) => "interface",
                Some(lsp::CompletionItemKind::MODULE) => "module",
                Some(lsp::CompletionItemKind::PROPERTY) => "property",
                Some(lsp::CompletionItemKind::UNIT) => "unit",
                Some(lsp::CompletionItemKind::VALUE) => "value",
                Some(lsp::CompletionItemKind::ENUM) => "enum",
                Some(lsp::CompletionItemKind::KEYWORD) => "keyword",
                Some(lsp::CompletionItemKind::SNIPPET) => "snippet",
                Some(lsp::CompletionItemKind::COLOR) => "color",
                Some(lsp::CompletionItemKind::FILE) => "file",
                Some(lsp::CompletionItemKind::REFERENCE) => "reference",
                Some(lsp::CompletionItemKind::FOLDER) => "folder",
                Some(lsp::CompletionItemKind::ENUM_MEMBER) => "enum_member",
                Some(lsp::CompletionItemKind::CONSTANT) => "constant",
                Some(lsp::CompletionItemKind::STRUCT) => "struct",
                Some(lsp::CompletionItemKind::EVENT) => "event",
                Some(lsp::CompletionItemKind::OPERATOR) => "operator",
                Some(lsp::CompletionItemKind::TYPE_PARAMETER) => "type_param",
                Some(kind) => {
                    log::error!("Received unknown completion item kind: {:?}", kind);
                    ""
                }
                None => "",
            }),
        ])
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct CompletionItem {
    pub item: lsp::CompletionItem,
    pub language_server_id: usize,
    pub resolved: bool,
}

/// Wraps a Menu.
pub struct Completion {
    popup: Popup<Menu<CompletionItem>>,
    start_offset: usize,
    #[allow(dead_code)]
    trigger_offset: usize,
    // TODO: maintain a completioncontext with trigger kind & trigger char
}

impl Completion {
    pub const ID: &'static str = "completion";

    pub fn new(
        editor: &Editor,
        savepoint: Arc<SavePoint>,
        mut items: Vec<CompletionItem>,
        start_offset: usize,
        trigger_offset: usize,
    ) -> Self {
        let preview_completion_insert = editor.config().preview_completion_insert;
        let replace_mode = editor.config().completion_replace;
        // Sort completion items according to their preselect status (given by the LSP server)
        items.sort_by_key(|item| !item.item.preselect.unwrap_or(false));

        // Then create the menu
        let menu = Menu::new(items, (), move |editor: &mut Editor, item, event| {
            fn item_to_transaction(
                doc: &Document,
                view_id: ViewId,
                item: &lsp::CompletionItem,
                offset_encoding: OffsetEncoding,
                trigger_offset: usize,
                include_placeholder: bool,
                replace_mode: bool,
            ) -> Transaction {
                use helix_lsp::snippet;
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

                    let Some(range) =
                        util::lsp_range_to_range(doc.text(), edit.range, offset_encoding)
                    else {
                        return Transaction::new(doc.text());
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
                    match snippet::parse(&new_text) {
                        Ok(snippet) => util::generate_transaction_from_snippet(
                            doc.text(),
                            selection,
                            edit_offset,
                            replace_mode,
                            snippet,
                            doc.line_ending.as_str(),
                            include_placeholder,
                            doc.tab_width(),
                            doc.indent_width(),
                        ),
                        Err(err) => {
                            log::error!(
                                "Failed to parse snippet: {:?}, remaining output: {}",
                                &new_text,
                                err
                            );
                            Transaction::new(doc.text())
                        }
                    }
                } else {
                    util::generate_transaction_from_completion_edit(
                        doc.text(),
                        selection,
                        edit_offset,
                        replace_mode,
                        new_text,
                    )
                }
            }

            fn completion_changes(transaction: &Transaction, trigger_offset: usize) -> Vec<Change> {
                transaction
                    .changes_iter()
                    .filter(|(start, end, _)| (*start..=*end).contains(&trigger_offset))
                    .collect()
            }

            let (view, doc) = current!(editor);

            macro_rules! language_server {
                ($item:expr) => {
                    match editor
                        .language_servers
                        .get_by_id($item.language_server_id)
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
                    if editor.last_completion.is_none() {
                        editor.last_completion = Some(CompleteAction::Selected {
                            savepoint: doc.savepoint(view),
                        })
                    }
                    // if more text was entered, remove it
                    doc.restore(view, &savepoint, false);
                    // always present here
                    let item = item.unwrap();

                    let transaction = item_to_transaction(
                        doc,
                        view.id,
                        &item.item,
                        language_server!(item).offset_encoding(),
                        trigger_offset,
                        true,
                        replace_mode,
                    );
                    doc.apply_temporary(&transaction, view.id);
                }
                PromptEvent::Update => {}
                PromptEvent::Validate => {
                    if let Some(CompleteAction::Selected { savepoint }) =
                        editor.last_completion.take()
                    {
                        doc.restore(view, &savepoint, false);
                    }
                    // always present here
                    let mut item = item.unwrap().clone();

                    let language_server = language_server!(item);
                    let offset_encoding = language_server.offset_encoding();

                    let language_server = editor
                        .language_servers
                        .get_by_id(item.language_server_id)
                        .unwrap();

                    // resolve item if not yet resolved
                    if !item.resolved {
                        if let Some(resolved) =
                            Self::resolve_completion_item(language_server, item.item.clone())
                        {
                            item.item = resolved;
                        }
                    };
                    // if more text was entered, remove it
                    doc.restore(view, &savepoint, true);
                    // save an undo checkpoint before the completion
                    doc.append_changes_to_history(view);
                    let transaction = item_to_transaction(
                        doc,
                        view.id,
                        &item.item,
                        offset_encoding,
                        trigger_offset,
                        false,
                        replace_mode,
                    );
                    doc.apply(&transaction, view.id);

                    editor.last_completion = Some(CompleteAction::Applied {
                        trigger_offset,
                        changes: completion_changes(&transaction, trigger_offset),
                    });

                    // TODO: add additional _edits to completion_changes?
                    if let Some(additional_edits) = item.item.additional_text_edits {
                        if !additional_edits.is_empty() {
                            let transaction = util::generate_transaction_from_edits(
                                doc.text(),
                                additional_edits,
                                offset_encoding, // TODO: should probably transcode in Client
                            );
                            doc.apply(&transaction, view.id);
                        }
                    }
                }
            };
        });

        let margin = if editor.menu_border() {
            Margin::vertical(1)
        } else {
            Margin::none()
        };

        let popup = Popup::new(Self::ID, menu)
            .with_scrollbar(false)
            .ignore_escape_key(true)
            .margin(margin);

        let mut completion = Self {
            popup,
            start_offset,
            trigger_offset,
        };

        // need to recompute immediately in case start_offset != trigger_offset
        completion.recompute_filter(editor);

        completion
    }

    fn resolve_completion_item(
        language_server: &helix_lsp::Client,
        completion_item: lsp::CompletionItem,
    ) -> Option<lsp::CompletionItem> {
        let future = language_server.resolve_completion_item(completion_item)?;
        let response = helix_lsp::block_on(future);
        match response {
            Ok(value) => serde_json::from_value(value).ok(),
            Err(err) => {
                log::error!("Failed to resolve completion item: {}", err);
                None
            }
        }
    }

    pub fn recompute_filter(&mut self, editor: &Editor) {
        // recompute menu based on matches
        let menu = self.popup.contents_mut();
        let (view, doc) = current_ref!(editor);

        // cx.hooks()
        // cx.add_hook(enum type,  ||)
        // cx.trigger_hook(enum type, &str, ...) <-- there has to be enough to identify doc/view
        // callback with editor & compositor
        //
        // trigger_hook sends event into channel, that's consumed in the global loop and
        // triggers all registered callbacks
        // TODO: hooks should get processed immediately so maybe do it after select!(), before
        // looping?

        let cursor = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));
        if self.trigger_offset <= cursor {
            let fragment = doc.text().slice(self.start_offset..cursor);
            let text = Cow::from(fragment);
            // TODO: logic is same as ui/picker
            menu.score(&text);
        } else {
            // we backspaced before the start offset, clear the menu
            // this will cause the editor to remove the completion popup
            menu.clear();
        }
    }

    pub fn update(&mut self, cx: &mut commands::Context) {
        self.recompute_filter(cx.editor)
    }

    pub fn is_empty(&self) -> bool {
        self.popup.contents().is_empty()
    }

    fn replace_item(&mut self, old_item: CompletionItem, new_item: CompletionItem) {
        self.popup.contents_mut().replace_option(old_item, new_item);
    }

    /// Asynchronously requests that the currently selection completion item is
    /// resolved through LSP `completionItem/resolve`.
    pub fn ensure_item_resolved(&mut self, cx: &mut commands::Context) -> bool {
        // > If computing full completion items is expensive, servers can additionally provide a
        // > handler for the completion item resolve request. ...
        // > A typical use case is for example: the `textDocument/completion` request doesn't fill
        // > in the `documentation` property for returned completion items since it is expensive
        // > to compute. When the item is selected in the user interface then a
        // > 'completionItem/resolve' request is sent with the selected completion item as a parameter.
        // > The returned completion item should have the documentation property filled in.
        // https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
        let current_item = match self.popup.contents().selection() {
            Some(item) if !item.resolved => item.clone(),
            _ => return false,
        };

        let Some(language_server) = cx
            .editor
            .language_server_by_id(current_item.language_server_id)
        else {
            return false;
        };

        // This method should not block the compositor so we handle the response asynchronously.
        let Some(future) = language_server.resolve_completion_item(current_item.item.clone())
        else {
            return false;
        };

        cx.callback(
            future,
            move |_editor, compositor, response: Option<lsp::CompletionItem>| {
                let resolved_item = match response {
                    Some(item) => item,
                    None => return,
                };

                if let Some(completion) = &mut compositor
                    .find::<crate::ui::EditorView>()
                    .unwrap()
                    .completion
                {
                    let resolved_item = CompletionItem {
                        item: resolved_item,
                        language_server_id: current_item.language_server_id,
                        resolved: true,
                    };

                    completion.replace_item(current_item, resolved_item);
                }
            },
        );

        true
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
        let option = match self.popup.contents().selection() {
            Some(option) => option,
            None => return,
        };
        // need to render:
        // option.detail
        // ---
        // option.documentation

        let (view, doc) = current!(cx.editor);
        let language = doc.language_name().unwrap_or("");
        let text = doc.text().slice(..);
        let cursor_pos = doc.selection(view.id).primary().cursor(text);
        let coords = view
            .screen_coords_at_pos(doc, text, cursor_pos)
            .expect("cursor must be in view");
        let cursor_pos = coords.row as u16;

        let markdowned = |lang: &str, detail: Option<&str>, doc: Option<&str>| {
            let md = match (detail, doc) {
                (Some(detail), Some(doc)) => format!("```{lang}\n{detail}\n```\n{doc}"),
                (Some(detail), None) => format!("```{lang}\n{detail}\n```"),
                (None, Some(doc)) => doc.to_string(),
                (None, None) => String::new(),
            };
            Markdown::new(md, cx.editor.syn_loader.clone())
        };

        let mut markdown_doc = match &option.item.documentation {
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
        };

        let popup_area = {
            let (popup_x, popup_y) = self.popup.get_rel_position(area, cx.editor);
            let (popup_width, popup_height) = self.popup.get_size();
            Rect::new(popup_x, popup_y, popup_width, popup_height)
        };

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
            use tui::widgets::{Block, Borders, Widget};
            Widget::render(Block::default().borders(Borders::ALL), doc_area, surface);
        }

        markdown_doc.render(doc_area, surface, cx);
    }
}
