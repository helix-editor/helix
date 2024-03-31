use std::time::Duration;
use helix_core::{coords_at_pos, softwrapped_dimensions, Rope};
use helix_event::{
    cancelable_future, cancelation, register_hook, send_blocking, CancelRx, CancelTx,
};
use helix_lsp::copilot_types::DocCompletion;
use helix_lsp::util::{lsp_pos_to_pos, lsp_range_to_range};
use helix_view::document::{Copilot, Mode};
use helix_view::Editor;
use tokio::time::Instant;
use crate::commands::MappableCommand;
use crate::events::{OnModeSwitch, PostCommand, PostInsertChar};
use crate::handlers::Handlers;
use helix_view::{handlers::lsp::CopilotEvent, DocumentId};
use crate::job::{dispatch, dispatch_blocking};

pub struct CopilotHandler {
    doc_id: Option<DocumentId>,
    cancel: Option<CancelTx>,
}

impl CopilotHandler {
    pub fn new() -> Self {
        Self {
            doc_id: None,
            cancel: None,
        }
    }
}

impl helix_event::AsyncHook for CopilotHandler {
    type Event = CopilotEvent;
    fn handle_event(
        &mut self,
        event: Self::Event,
        _: Option<Instant>,
    ) -> Option<Instant> {
        match event {
            CopilotEvent::RequestCompletion { doc_id } => {
                self.doc_id = Some(doc_id);
                self.cancel.take();
                Some(Instant::now() + Duration::from_millis(100))
            }
            CopilotEvent::CancelInFlightCompletion => {
                self.cancel.take();
                None
            }
        }
    }

    fn finish_debounce(&mut self) {
        let Some(doc_id) = self.doc_id else {return;};
        let (tx, rx) = cancelation();
        self.cancel = Some(tx);

        dispatch_blocking(move |editor, _| {
            copilot_completion(editor, doc_id, rx);
        });
    }
}

fn copilot_completion(editor: &mut Editor, doc_id: DocumentId, cancel: CancelRx) {
    let (view, doc) = current_ref!(editor);
    if doc.id() != doc_id {
        return;
    }

    let Some(copilot_ls) = doc
        .language_servers()
        .filter(|ls| ls.name() == "copilot")
        .next()
    else { return; };
    let offset_encoding = copilot_ls.offset_encoding();

    let copilot_future = if let Some(copilot_doc) = doc.copilot_document(view.id, offset_encoding) {
        copilot_ls.copilot_completion(copilot_doc)
    } else {
        return;
    };

    tokio::spawn(async move {
        if let Some(item) = cancelable_future(copilot_future, cancel).await {
            if let Ok(Some(completion_reponse)) = item {
                dispatch(move |editor, _| {
                    let completions = if completion_reponse.completions.len() > 0 {
                        completion_reponse.completions
                    } else {
                        return;
                    };

                    let (view, doc) = current!(editor);
                    let doc_completion = completions
                        .into_iter()
                        .filter_map(|completion| {
                            /*
                            NOTE:
                            The computation below is neccesary because:

                            1. If we're typing the string ' let x = Vec::From(*);' where * is cursor position, copilot will sometimes respond with the
                            completion '[1,2,3]' where '[1,2,3]' is to be inserted between the two brackets. It will not give any other information about 
                            the first line. In this case, we cannot only render the virtual text '[1,2,3]' (rendering virtual text has no effect on the position 
                            of the doc's non-virtual text (it will not cause ');' to move rightwards).
                            Hence we need to calculate what the whole first line will look like post applying copilot's completion.

                            2. We also need to calculate what the whole first line will look like post applying the completion in order to calculate the number of 
                            additional lines needed for first line's softwrap. Eg ' let x = String::From(*);' may not require softwrapping, but 
                            ' let x = String::From([1,2,3]);'  may do. 

                            The remaining additional lines that copilot may insert will not be interleaved with the doc's text, so the above problems are only relevant 
                            to the completion's first line
                            */
                            let txt_fmt = doc.text_format(view.inner_width(&doc), None);

                            let Some(range) = lsp_range_to_range(doc.text(), completion.range, offset_encoding) else {return None;};
                            let (start_coords, end_coords) = (coords_at_pos(doc.text().slice(..).into(), range.anchor), coords_at_pos(doc.text().slice(..).into(), range.head));

                            let display_pos =
                                lsp_pos_to_pos(doc.text(), completion.position, offset_encoding)?;
                            let display_coords =
                                coords_at_pos(doc.text().slice(..).into(), display_pos);

                            let line_idx = doc.text().char_to_line(display_pos);
                            let mut line_rope = Rope::from(doc.text().get_line(line_idx)?);
                            let pre_insert_softwrap =
                                softwrapped_dimensions(line_rope.slice(..), &txt_fmt).0;

                            line_rope.remove(start_coords.col..end_coords.col);
                            line_rope.insert(start_coords.col, &completion.text);
                            let post_insert_softwrap =
                                softwrapped_dimensions(line_rope.slice(..), &txt_fmt).0;
                            
                            Some(DocCompletion {
                                text: completion.text,
                                lsp_range: completion.range,
                                display_text: line_rope.to_string(),
                                display_coords,
                                additional_softwrap: post_insert_softwrap - pre_insert_softwrap,
                                doc_version: doc.version() as usize,
                            })
                        })
                        .collect::<Vec<DocCompletion>>();

                    doc.copilot = Some(Copilot {
                        should_render: editor.auto_render_copilot,
                        completions: doc_completion,
                        idx: 0,
                        offset_encoding,
                    });
                })
                .await;
            }
        }
    });
}

pub(super) fn try_register_hooks(handlers: &Handlers) {
    let Some(copilot_handler) = handlers.copilot.clone() else {return;};

    register_hook!(move |event: &mut PostCommand<'_, '_>| {
        if let MappableCommand::Static { name: "copilot_show_completion", .. } = event.command {
            return Ok(());
        };

        let (_, doc) = current!(event.cx.editor);
        doc.clear_copilot_completions();
        Ok(())
    });

    let tx = copilot_handler.clone();
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        let (_, doc) = current!(event.cx.editor);
        doc.clear_copilot_completions();
        if event.old_mode == Mode::Insert {
            send_blocking(&tx, CopilotEvent::CancelInFlightCompletion);
        } else if event.new_mode == Mode::Insert {
            send_blocking(&tx, CopilotEvent::RequestCompletion { doc_id: doc.id() });
        }
        Ok(())
    });

    let tx = copilot_handler.clone();
    register_hook!(move |event: &mut PostInsertChar<'_, '_>| {
        let (_, doc) = current!(event.cx.editor);
        doc.clear_copilot_completions();
        send_blocking(&tx, CopilotEvent::RequestCompletion { doc_id: doc.id() });
        Ok(())
    });
}
