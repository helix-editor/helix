use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    str::FromStr as _,
    sync::Arc,
};

use helix_core::{self as core, completion::CompletionProvider, Selection, Transaction};
use helix_event::TaskHandle;
use helix_stdx::path::{self, canonicalize, fold_home_dir, get_path_suffix};
use helix_view::{document::SavePoint, handlers::completion::ResponseContext, Document};
use url::Url;

use crate::handlers::completion::{item::CompletionResponse, CompletionItem, CompletionItems};

pub(crate) fn path_completion(
    selection: Selection,
    doc: &Document,
    handle: TaskHandle,
    savepoint: Arc<SavePoint>,
) -> Option<impl FnOnce() -> CompletionResponse> {
    if !doc.path_completion_enabled() {
        return None;
    }

    let text = doc.text().clone();
    let cursor = selection.primary().cursor(text.slice(..));
    let cur_line = text.char_to_line(cursor);
    let start = text.line_to_char(cur_line).max(cursor.saturating_sub(1000));
    let line_until_cursor = text.slice(start..cursor);

    let (dir_path, typed_file_name) =
        get_path_suffix(line_until_cursor, false).and_then(|matched_path| {
            let matched_path = Cow::from(matched_path);
            let path: Cow<_> = if matched_path.starts_with("file://") {
                Url::from_str(&matched_path)
                    .ok()
                    .and_then(|url| url.to_file_path().ok())?
                    .into()
            } else {
                Path::new(&*matched_path).into()
            };
            let path = path::expand(&path);
            let parent_dir = doc.path().and_then(|dp| dp.parent());
            let path = match parent_dir {
                Some(parent_dir) if path.is_relative() => parent_dir.join(&path),
                _ => path.into_owned(),
            };
            #[cfg(windows)]
            let ends_with_slash = matches!(matched_path.as_bytes().last(), Some(b'/' | b'\\'));
            #[cfg(not(windows))]
            let ends_with_slash = matches!(matched_path.as_bytes().last(), Some(b'/'));

            if ends_with_slash {
                Some((PathBuf::from(path.as_path()), None))
            } else {
                path.parent().map(|parent_path| {
                    (
                        PathBuf::from(parent_path),
                        path.file_name().and_then(|f| f.to_str().map(String::from)),
                    )
                })
            }
        })?;

    if handle.is_canceled() {
        return None;
    }

    // TODO: handle properly in the future
    const PRIORITY: i8 = 1;
    let future = move || {
        let Ok(read_dir) = std::fs::read_dir(&dir_path) else {
            return CompletionResponse {
                items: CompletionItems::Other(Vec::new()),
                provider: CompletionProvider::Path,
                context: ResponseContext {
                    is_incomplete: false,
                    priority: PRIORITY,
                    savepoint,
                },
            };
        };

        let edit_diff = typed_file_name
            .as_ref()
            .map(|s| s.chars().count())
            .unwrap_or_default();

        let res: Vec<_> = read_dir
            .filter_map(Result::ok)
            .filter_map(|dir_entry| {
                dir_entry
                    .metadata()
                    .ok()
                    .and_then(|md| Some((dir_entry.file_name().into_string().ok()?, md)))
            })
            .map_while(|(file_name, md)| {
                if handle.is_canceled() {
                    return None;
                }

                let kind = path_kind(&md);
                let documentation = path_documentation(&md, &dir_path.join(&file_name), kind);

                let transaction = Transaction::change_by_selection(&text, &selection, |range| {
                    let cursor = range.cursor(text.slice(..));
                    (cursor - edit_diff, cursor, Some((&file_name).into()))
                });

                Some(CompletionItem::Other(core::CompletionItem {
                    kind: Cow::Borrowed(kind),
                    label: file_name.into(),
                    transaction,
                    documentation: Some(documentation),
                    provider: CompletionProvider::Path,
                }))
            })
            .collect();
        CompletionResponse {
            items: CompletionItems::Other(res),
            provider: CompletionProvider::Path,
            context: ResponseContext {
                is_incomplete: false,
                priority: PRIORITY,
                savepoint,
            },
        }
    };

    Some(future)
}

#[cfg(unix)]
fn path_documentation(md: &fs::Metadata, full_path: &Path, kind: &str) -> String {
    let full_path = fold_home_dir(canonicalize(full_path));
    let full_path_name = full_path.to_string_lossy();

    use std::os::unix::prelude::PermissionsExt;
    let mode = md.permissions().mode();

    let perms = [
        (libc::S_IRUSR, 'r'),
        (libc::S_IWUSR, 'w'),
        (libc::S_IXUSR, 'x'),
        (libc::S_IRGRP, 'r'),
        (libc::S_IWGRP, 'w'),
        (libc::S_IXGRP, 'x'),
        (libc::S_IROTH, 'r'),
        (libc::S_IWOTH, 'w'),
        (libc::S_IXOTH, 'x'),
    ]
    .into_iter()
    .fold(String::with_capacity(9), |mut acc, (p, s)| {
        // This cast is necessary on some platforms such as macos as `mode_t` is u16 there
        #[allow(clippy::unnecessary_cast)]
        acc.push(if mode & (p as u32) > 0 { s } else { '-' });
        acc
    });

    // TODO it would be great to be able to individually color the documentation,
    // but this will likely require a custom doc implementation (i.e. not `lsp::Documentation`)
    // and/or different rendering in completion.rs
    format!(
        "type: `{kind}`\n\
         permissions: `[{perms}]`\n\
         full path: `{full_path_name}`",
    )
}

#[cfg(not(unix))]
fn path_documentation(_md: &fs::Metadata, full_path: &Path, kind: &str) -> String {
    let full_path = fold_home_dir(canonicalize(full_path));
    let full_path_name = full_path.to_string_lossy();
    format!("type: `{kind}`\nfull path: `{full_path_name}`",)
}

#[cfg(unix)]
fn path_kind(md: &fs::Metadata) -> &'static str {
    if md.is_symlink() {
        "link"
    } else if md.is_dir() {
        "folder"
    } else {
        use std::os::unix::fs::FileTypeExt;
        if md.file_type().is_block_device() {
            "block"
        } else if md.file_type().is_socket() {
            "socket"
        } else if md.file_type().is_char_device() {
            "char_device"
        } else if md.file_type().is_fifo() {
            "fifo"
        } else {
            "file"
        }
    }
}

#[cfg(not(unix))]
fn path_kind(md: &fs::Metadata) -> &'static str {
    if md.is_symlink() {
        "link"
    } else if md.is_dir() {
        "folder"
    } else {
        "file"
    }
}
