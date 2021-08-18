mod typable;

use crate::{compositor::Compositor, job, DocumentId, Editor};
use helix_core::Transaction;
use std::future::Future;

/// Sometimes when applying formatting changes we want to mark the buffer as unmodified, for
/// example because we just applied the same changes while saving.
enum Modified {
    SetUnmodified,
    LeaveModified,
}

// Creates an LspCallback that waits for formatting changes to be computed. When they're done,
// it applies them, but only if the doc hasn't changed.
//
// TODO: provide some way to cancel this, probably as part of a more general job cancellation
// scheme
async fn make_format_callback(
    doc_id: DocumentId,
    doc_version: i32,
    modified: Modified,
    format: impl Future<Output = helix_lsp::util::LspFormatting> + Send + 'static,
) -> anyhow::Result<job::Callback> {
    let format = format.await;
    let call: job::Callback = Box::new(
        move |editor: &mut Editor, _compositor: &mut dyn Compositor| {
            let view_id = view!(editor).id;
            if let Some(doc) = editor.document_mut(doc_id) {
                if doc.version() == doc_version {
                    doc.apply(&Transaction::from(format), view_id);
                    doc.append_changes_to_history(view_id);
                    if let Modified::SetUnmodified = modified {
                        doc.reset_modified();
                    }
                } else {
                    log::info!("discarded formatting changes because the document changed");
                }
            }
        },
    );
    Ok(call)
}

