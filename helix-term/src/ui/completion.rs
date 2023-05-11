use crate::compositor::{Component, Context, Event, EventResult};
use helix_view::{
    document::SavePoint,
    editor::CompleteAction,
    theme::{Modifier, Style},
    Theme, ViewId,
};
use tui::{buffer::Buffer as Surface, text::Span};

use std::{borrow::Cow, sync::Arc};

use helix_core::{Change, Transaction};
use helix_view::{graphics::Rect, Document, Editor};

use crate::commands;
use crate::ui::{menu, Markdown, Menu, Popup, PromptEvent};

use helix_lsp::{lsp, util};
use lsp::CompletionItem;

impl menu::Item for CompletionItem {
    type Data = ();
    fn sort_text(&self, data: &Self::Data) -> Cow<str> {
        self.filter_text(data)
    }

    #[inline]
    fn filter_text(&self, _data: &Self::Data) -> Cow<str> {
        self.filter_text
            .as_ref()
            .unwrap_or(&self.label)
            .as_str()
            .into()
    }

    // fn label(&self, _data: &Self::Data) -> Spans {
    //     self.label.as_str().into()
    // }

    fn format(&self, _data: &Self::Data, theme: Option<&Theme>) -> menu::Row {
        let (lsp_type_label, style) = match self.kind {
            Some(lsp::CompletionItemKind::TEXT) => ("text", Some("ui.text")),
            Some(lsp::CompletionItemKind::METHOD) => ("method", Some("function.method")),
            Some(lsp::CompletionItemKind::FUNCTION) => ("function", Some("function")),
            Some(lsp::CompletionItemKind::CONSTRUCTOR) => ("constructor", Some("constructor")),
            Some(lsp::CompletionItemKind::FIELD) => ("field", Some("variable.other.member")),
            Some(lsp::CompletionItemKind::VARIABLE) => ("variable", Some("variable")),
            Some(lsp::CompletionItemKind::CLASS) => ("class", Some("type")),
            Some(lsp::CompletionItemKind::INTERFACE) => ("interface", Some("type")),
            Some(lsp::CompletionItemKind::MODULE) => ("module", Some("module")),
            Some(lsp::CompletionItemKind::PROPERTY) => ("property", Some("attributes")),
            Some(lsp::CompletionItemKind::UNIT) => ("unit", Some("constant")),
            Some(lsp::CompletionItemKind::VALUE) => ("value", Some("string")),
            Some(lsp::CompletionItemKind::ENUM) => ("enum", Some("type")),
            Some(lsp::CompletionItemKind::KEYWORD) => ("keyword", Some("keyword")),
            Some(lsp::CompletionItemKind::SNIPPET) => ("snippet", None),
            Some(lsp::CompletionItemKind::COLOR) => ("color", None),
            Some(lsp::CompletionItemKind::FILE) => ("file", None),
            Some(lsp::CompletionItemKind::REFERENCE) => ("reference", None),
            Some(lsp::CompletionItemKind::FOLDER) => ("folder", None),
            Some(lsp::CompletionItemKind::ENUM_MEMBER) => {
                ("enum_member", Some("type.enum.variant"))
            }
            Some(lsp::CompletionItemKind::CONSTANT) => ("constant", Some("constant")),
            Some(lsp::CompletionItemKind::STRUCT) => ("struct", Some("type")),
            Some(lsp::CompletionItemKind::EVENT) => ("event", None),
            Some(lsp::CompletionItemKind::OPERATOR) => ("operator", Some("operator")),
            Some(lsp::CompletionItemKind::TYPE_PARAMETER) => {
                ("type_param", Some("function.parameter"))
            }
            Some(kind) => unimplemented!("{:?}", kind),
            None => ("", None),
        };
        let mut lsp_type_style = theme
            .zip(style)
            .map(|(theme, style)| theme.get(style))
            .unwrap_or_default()
            .remove_modifier(Modifier::all())
            .add_modifier(Modifier::ITALIC);
        lsp_type_style.bg = None;

        let deprecated = self.deprecated.unwrap_or_default()
            || self.tags.as_ref().map_or(false, |tags| {
                tags.contains(&lsp::CompletionItemTag::DEPRECATED)
            });

        menu::Row::new(vec![
            menu::Cell::from(Span::styled(
                self.label.as_str(),
                if deprecated {
                    Style::default().add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default()
                },
            )),
            menu::Cell::from(lsp_type_label).style(lsp_type_style),
            // self.detail.as_deref().unwrap_or("")
            // self.label_details
            //     .as_ref()
            //     .or(self.detail())
            //     .as_str(),
        ])
    }
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
        offset_encoding: helix_lsp::OffsetEncoding,
        start_offset: usize,
        trigger_offset: usize,
    ) -> Self {
        let replace_mode = editor.config().completion_replace;
        // Sort completion items according to their preselect status (given by the LSP server)
        items.sort_by_key(|item| !item.preselect.unwrap_or(false));

        // Then create the menu
        let menu = Menu::new(items, (), move |editor: &mut Editor, item, event| {
            fn item_to_transaction(
                doc: &Document,
                view_id: ViewId,
                item: &CompletionItem,
                offset_encoding: helix_lsp::OffsetEncoding,
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

                    let Some(range) = util::lsp_range_to_range(doc.text(), edit.range, offset_encoding) else{
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

            // if more text was entered, remove it
            doc.restore(view, &savepoint);

            match event {
                PromptEvent::Abort => {
                    editor.last_completion = None;
                }
                PromptEvent::Update => {
                    // always present here
                    let item = item.unwrap();

                    let transaction = item_to_transaction(
                        doc,
                        view.id,
                        item,
                        offset_encoding,
                        trigger_offset,
                        true,
                        replace_mode,
                    );

                    // initialize a savepoint
                    doc.apply(&transaction, view.id);

                    editor.last_completion = Some(CompleteAction {
                        trigger_offset,
                        changes: completion_changes(&transaction, trigger_offset),
                    });
                }
                PromptEvent::Validate => {
                    // always present here
                    let item = item.unwrap();

                    let transaction = item_to_transaction(
                        doc,
                        view.id,
                        item,
                        offset_encoding,
                        trigger_offset,
                        false,
                        replace_mode,
                    );

                    doc.apply(&transaction, view.id);

                    editor.last_completion = Some(CompleteAction {
                        trigger_offset,
                        changes: completion_changes(&transaction, trigger_offset),
                    });

                    // apply additional edits, mostly used to auto import unqualified types
                    let resolved_item = if item
                        .additional_text_edits
                        .as_ref()
                        .map(|edits| !edits.is_empty())
                        .unwrap_or(false)
                    {
                        None
                    } else {
                        Self::resolve_completion_item(doc, item.clone())
                    };

                    if let Some(additional_edits) = resolved_item
                        .as_ref()
                        .and_then(|item| item.additional_text_edits.as_ref())
                        .or(item.additional_text_edits.as_ref())
                    {
                        if !additional_edits.is_empty() {
                            let transaction = util::generate_transaction_from_edits(
                                doc.text(),
                                additional_edits.clone(),
                                offset_encoding, // TODO: should probably transcode in Client
                            );
                            doc.apply(&transaction, view.id);
                        }
                    }
                }
            };
        });
        let popup = Popup::new(Self::ID, menu)
            .with_scrollbar(false)
            .ignore_escape_key(true);
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
        doc: &Document,
        completion_item: lsp::CompletionItem,
    ) -> Option<CompletionItem> {
        let language_server = doc.language_server()?;

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

    fn replace_item(&mut self, old_item: lsp::CompletionItem, new_item: lsp::CompletionItem) {
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
            Some(item) if item.documentation.is_none() => item.clone(),
            _ => return false,
        };

        let language_server = match doc!(cx.editor).language_server() {
            Some(language_server) => language_server,
            None => return false,
        };

        // This method should not block the compositor so we handle the response asynchronously.
        let future = match language_server.resolve_completion_item(current_item.clone()) {
            Some(future) => future,
            None => return false,
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

        let mut markdown_doc = match &option.documentation {
            Some(lsp::Documentation::String(contents))
            | Some(lsp::Documentation::MarkupContent(lsp::MarkupContent {
                kind: lsp::MarkupKind::PlainText,
                value: contents,
            })) => {
                // TODO: convert to wrapped text
                markdowned(language, option.detail.as_deref(), Some(contents))
            }
            Some(lsp::Documentation::MarkupContent(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value: contents,
            })) => {
                // TODO: set language based on doc scope
                markdowned(language, option.detail.as_deref(), Some(contents))
            }
            None if option.detail.is_some() => {
                // TODO: set language based on doc scope
                markdowned(language, option.detail.as_deref(), None)
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
        markdown_doc.render(doc_area, surface, cx);
    }
}
