// use std::fs::File;
// use std::io::BufReader;
// use std::io::BufWriter;
// use std::path::PathBuf;

// #[cfg(unix)]
// use std::os::unix::prelude::OsStrExt;

// use anyhow::Context;
// use anyhow::Result;
// use helix_core::history::deserialize_history;
// use helix_core::history::serialize_history;
// use helix_core::parse::*;

// use crate::Editor;

// use super::Session;

// // TODO: Check if serialized files already exist, and use them.
// // TODO: Maybe have a way to verify that the histories match, and overwrite if they don't.
// pub fn serialize(session: &mut Session, editor: &mut Editor) -> Result<()> {
//     let cwd = std::env::current_dir()?;
//     for doc in editor.documents_mut().filter(|doc| doc.path().is_some()) {

//     }
//     // Handle existing index file to merge.
//     let mut index_file = session.get_mut("undo/index")?;
//     let mut index = deserialize_index(&index_file).context("failed to parse undo index")?;
//     for path in editor.documents().filter_map(|doc| doc.path().cloned()) {
//         if !index.iter().any(|(_, value)| *value == path) {
//             let key = index.last().map(|(key, _)| key + 1).unwrap_or(0);
//             index.push((key, path));
//         }
//     }
//     serialize_index(&mut index_file, &index)?;

//     for (filename, doc_path) in index {
//         let doc = match editor
//             .documents_mut()
//             .find(|doc| doc.path() == Some(&doc_path))
//         {
//             Some(doc) => doc,
//             None => continue,
//         };
//         let filename = format!("undo/{filename}");
//         let file = session.get_mut(&filename)?;
//         let history = doc.history.take();
//         serialize_history(file, &history)?;
//         doc.history.set(history);
//     }

//     Ok(())
// }

// pub fn deserialize(session: &mut Session, editor: &mut Editor) -> Result<()> {
//     let index = session
//         .get("undo/index")
//         .and_then(|file| deserialize_index(&file))
//         .context("failed to parse index file")?;

//     for (filename, doc_path) in index {
//         let id = editor.open(&doc_path, crate::editor::Action::Load)?;
//         let doc = editor.document_mut(id).unwrap();
//         let filename = format!("undo/{filename}");
//         let file = session.get(&filename)?;
//         doc.history = std::cell::Cell::new(deserialize_history(file)?);
//     }

//     Ok(())
// }
