use std::{borrow::Cow, sync::Arc};

use helix_core::{
    self as core, chars::char_is_word, completion::CompletionProvider, movement, Transaction,
};
use helix_event::TaskHandle;
use helix_stdx::rope::RopeSliceExt as _;
use helix_view::{
    document::SavePoint, handlers::completion::ResponseContext, Document, Editor, ViewId,
};

use super::{request::TriggerKind, CompletionItem, CompletionItems, CompletionResponse, Trigger};

const COMPLETION_KIND: &str = "word";

pub(super) fn completion(
    editor: &Editor,
    trigger: Trigger,
    handle: TaskHandle,
    savepoint: Arc<SavePoint>,
) -> Option<impl FnOnce() -> CompletionResponse> {
    if !doc!(editor).word_completion_enabled() {
        return None;
    }
    let config = editor.config().word_completion;
    let doc_config = doc!(editor)
        .language_config()
        .and_then(|config| config.word_completion);
    let trigger_length = doc_config
        .and_then(|c| c.trigger_length)
        .unwrap_or(config.trigger_length)
        .get() as usize;

    let (view, doc) = current_ref!(editor);
    let rope = doc.text().clone();
    let word_index = editor.handlers.word_index().clone();
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).clone();
    let pos = selection.primary().cursor(text);

    let cursor = movement::move_prev_word_start(text, core::Range::point(pos), 1);
    if cursor.head == pos {
        return None;
    }
    if trigger.kind != TriggerKind::Manual
        && text
            .slice(cursor.head..)
            .graphemes()
            .take(trigger_length)
            .take_while(|g| g.chars().all(char_is_word))
            .count()
            != trigger_length
    {
        return None;
    }

    let typed_word_range = cursor.head..pos;
    let typed_word = text.slice(typed_word_range.clone());
    let edit_diff = if typed_word
        .char(typed_word.len_chars().saturating_sub(1))
        .is_whitespace()
    {
        0
    } else {
        typed_word.len_chars()
    };

    if handle.is_canceled() {
        return None;
    }

    let future = move || {
        let text = rope.slice(..);
        let typed_word: Cow<_> = text.slice(typed_word_range).into();
        let items = word_index
            .matches(&typed_word)
            .into_iter()
            .filter(|word| word.as_str() != typed_word.as_ref())
            .map(|word| {
                let transaction = Transaction::change_by_selection(&rope, &selection, |range| {
                    let cursor = range.cursor(text);
                    (cursor - edit_diff, cursor, Some((&word).into()))
                });
                CompletionItem::Other(core::CompletionItem {
                    transaction,
                    label: word.into(),
                    kind: Cow::Borrowed(COMPLETION_KIND),
                    documentation: None,
                    provider: CompletionProvider::Word,
                })
            })
            .collect();

        CompletionResponse {
            items: CompletionItems::Other(items),
            provider: CompletionProvider::Word,
            context: ResponseContext {
                is_incomplete: false,
                priority: 0,
                savepoint,
            },
        }
    };

    Some(future)
}

pub(super) fn retain_valid_completions(
    trigger: Trigger,
    doc: &Document,
    view_id: ViewId,
    items: &mut Vec<CompletionItem>,
) {
    if trigger.kind == TriggerKind::Manual {
        return;
    }

    let text = doc.text().slice(..);
    let cursor = doc.selection(view_id).primary().cursor(text);
    if text
        .get_char(cursor.saturating_sub(1))
        .is_some_and(|ch| ch.is_whitespace())
    {
        items.retain(|item| {
            !matches!(
                item,
                CompletionItem::Other(core::CompletionItem {
                    provider: CompletionProvider::Word,
                    ..
                })
            )
        });
    }
}
