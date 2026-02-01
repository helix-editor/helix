use std::{
    collections::HashSet,
    iter,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use futures_util::FutureExt;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{sinks, BinaryDetection, SearcherBuilder};
use helix_core::{
    syntax::{Loader, QueryIterEvent},
    Rope, RopeSlice, Selection, Syntax, Uri,
};
use helix_stdx::{
    path,
    rope::{self, RopeSliceExt},
};
use helix_view::{
    align_view,
    document::{from_reader, SCRATCH_BUFFER_NAME},
    Align, Document, DocumentId, Editor,
};
use ignore::{DirEntry, WalkBuilder, WalkState};

use crate::{
    filter_picker_entry,
    ui::{
        overlay::overlaid,
        picker::{Injector, PathOrId},
        Picker, PickerColumn,
    },
};

use super::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagKind {
    Class,
    Constant,
    Function,
    Interface,
    Macro,
    Module,
    Section,
    Struct,
    Type,
}

impl TagKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Constant => "constant",
            Self::Function => "function",
            Self::Interface => "interface",
            Self::Macro => "macro",
            Self::Module => "module",
            Self::Section => "section",
            Self::Struct => "struct",
            Self::Type => "type",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "class" => Some(TagKind::Class),
            "constant" => Some(TagKind::Constant),
            "function" => Some(TagKind::Function),
            "interface" => Some(TagKind::Interface),
            "macro" => Some(TagKind::Macro),
            "module" => Some(TagKind::Module),
            "section" => Some(TagKind::Section),
            "struct" => Some(TagKind::Struct),
            "type" => Some(TagKind::Type),
            _ => None,
        }
    }
}

// NOTE: Uri is cheap to clone and DocumentId is Copy
#[derive(Debug, Clone)]
enum UriOrDocumentId {
    Uri(Uri),
    Id(DocumentId),
}

impl UriOrDocumentId {
    fn path_or_id(&self) -> Option<PathOrId<'_>> {
        match self {
            Self::Id(id) => Some(PathOrId::Id(*id)),
            Self::Uri(uri) => uri.as_path().map(PathOrId::Path),
        }
    }
}

#[derive(Debug)]
struct Tag {
    kind: TagKind,
    name: String,
    start: usize,
    end: usize,
    start_line: usize,
    end_line: usize,
    doc: UriOrDocumentId,
}

fn tags_iter<'a>(
    syntax: &'a Syntax,
    loader: &'a Loader,
    text: RopeSlice<'a>,
    doc: UriOrDocumentId,
    pattern: Option<&'a rope::Regex>,
) -> impl Iterator<Item = Tag> + 'a {
    let mut tags_iter = syntax.tags(text, loader, ..);

    iter::from_fn(move || loop {
        let QueryIterEvent::Match(mat) = tags_iter.next()? else {
            continue;
        };
        let query = &loader
            .tag_query(tags_iter.current_language())
            .expect("must have a tags query to emit matches")
            .query;
        let Some(kind) = query
            .capture_name(mat.capture)
            .strip_prefix("definition.")
            .and_then(TagKind::from_name)
        else {
            continue;
        };
        let range = mat.node.byte_range();
        if pattern.is_some_and(|pattern| {
            !pattern.is_match(text.regex_input_at_bytes(range.start as usize..range.end as usize))
        }) {
            continue;
        }
        let start = text.byte_to_char(range.start as usize);
        let end = text.byte_to_char(range.end as usize);
        return Some(Tag {
            kind,
            name: text.slice(start..end).to_string(),
            start,
            end,
            start_line: text.char_to_line(start),
            end_line: text.char_to_line(end),
            doc: doc.clone(),
        });
    })
}

pub fn syntax_symbol_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let Some(syntax) = doc.syntax() else {
        cx.editor
            .set_error("Syntax tree is not available on this buffer");
        return;
    };
    let doc_id = doc.id();
    let text = doc.text().slice(..);
    let loader = cx.editor.syn_loader.load();
    let tags = tags_iter(syntax, &loader, text, UriOrDocumentId::Id(doc.id()), None);

    let columns = vec![
        PickerColumn::new("kind", |tag: &Tag, _| tag.kind.as_str().into()),
        PickerColumn::new("name", |tag: &Tag, _| tag.name.as_str().into()),
    ];

    let picker = Picker::new(
        columns,
        1, // name
        tags,
        (),
        move |cx, tag, action| {
            cx.editor.switch(doc_id, action);
            let view = view_mut!(cx.editor);
            let doc = doc_mut!(cx.editor, &doc_id);
            doc.set_selection(view.id, Selection::single(tag.start, tag.end));
            if action.align_view(view, doc.id()) {
                align_view(doc, view, Align::Center)
            }
        },
    )
    .with_preview(|_editor, tag| {
        Some((tag.doc.path_or_id()?, Some((tag.start_line, tag.end_line))))
    })
    .truncate_start(false);

    cx.push_layer(Box::new(overlaid(picker)));
}

pub fn syntax_workspace_symbol_picker(cx: &mut Context) {
    #[derive(Debug)]
    struct SearchState {
        searcher_builder: SearcherBuilder,
        walk_builder: WalkBuilder,
        regex_matcher_builder: RegexMatcherBuilder,
        rope_regex_builder: rope::RegexBuilder,
        search_root: PathBuf,
        /// A cache of files that have been parsed in prior searches.
        syntax_cache: DashMap<PathBuf, Option<(Rope, Syntax)>>,
    }

    let mut searcher_builder = SearcherBuilder::new();
    searcher_builder.binary_detection(BinaryDetection::quit(b'\x00'));

    // Search from the workspace that the currently focused document is within. This behaves like global
    // search most of the time but helps when you have two projects open in splits.
    let search_root = if let Some(path) = doc!(cx.editor).path() {
        helix_loader::find_workspace_in(path).0
    } else {
        helix_loader::find_workspace().0
    };

    let absolute_root = search_root
        .canonicalize()
        .unwrap_or_else(|_| search_root.clone());

    let config = cx.editor.config();
    let dedup_symlinks = config.file_picker.deduplicate_links;

    let mut walk_builder = WalkBuilder::new(&search_root);
    walk_builder
        .hidden(config.file_picker.hidden)
        .parents(config.file_picker.parents)
        .ignore(config.file_picker.ignore)
        .follow_links(config.file_picker.follow_symlinks)
        .git_ignore(config.file_picker.git_ignore)
        .git_global(config.file_picker.git_global)
        .git_exclude(config.file_picker.git_exclude)
        .max_depth(config.file_picker.max_depth)
        .filter_entry(move |entry| filter_picker_entry(entry, &absolute_root, dedup_symlinks))
        .add_custom_ignore_filename(helix_loader::config_dir().join("ignore"))
        .add_custom_ignore_filename(".helix/ignore");

    let mut regex_matcher_builder = RegexMatcherBuilder::new();
    regex_matcher_builder.case_smart(config.search.smart_case);
    let mut rope_regex_builder = rope::RegexBuilder::new();
    rope_regex_builder.syntax(rope::Config::new().case_insensitive(config.search.smart_case));
    let state = SearchState {
        searcher_builder,
        walk_builder,
        regex_matcher_builder,
        rope_regex_builder,
        search_root,
        syntax_cache: DashMap::default(),
    };
    let reg = cx.register.unwrap_or('/');
    cx.editor.registers.last_search_register = reg;
    let columns = vec![
        PickerColumn::new("kind", |tag: &Tag, _| tag.kind.as_str().into()),
        PickerColumn::new("name", |tag: &Tag, _| tag.name.as_str().into()).without_filtering(),
        PickerColumn::new("path", |tag: &Tag, state: &SearchState| {
            match &tag.doc {
                UriOrDocumentId::Uri(uri) => {
                    if let Some(path) = uri.as_path() {
                        let path = if let Ok(stripped) = path.strip_prefix(&state.search_root) {
                            stripped
                        } else {
                            path
                        };
                        path.to_string_lossy().into()
                    } else {
                        uri.to_string().into()
                    }
                }
                // This picker only uses `Id` for scratch buffers for better display.
                UriOrDocumentId::Id(_) => SCRATCH_BUFFER_NAME.into(),
            }
        }),
    ];

    let get_tags = |query: &str,
                    editor: &mut Editor,
                    state: Arc<SearchState>,
                    injector: &Injector<_, _>| {
        if query.len() < 3 {
            return async { Ok(()) }.boxed();
        }
        // Attempt to find the tag in any open documents.
        let pattern = match state.rope_regex_builder.build(query) {
            Ok(pattern) => pattern,
            Err(err) => return async { Err(anyhow::anyhow!(err)) }.boxed(),
        };
        let loader = editor.syn_loader.load();
        for doc in editor.documents() {
            let Some(syntax) = doc.syntax() else { continue };
            let text = doc.text().slice(..);
            let uri_or_id = doc
                .uri()
                .map(UriOrDocumentId::Uri)
                .unwrap_or_else(|| UriOrDocumentId::Id(doc.id()));
            for tag in tags_iter(syntax, &loader, text.slice(..), uri_or_id, Some(&pattern)) {
                if injector.push(tag).is_err() {
                    return async { Ok(()) }.boxed();
                }
            }
        }
        if !state.search_root.exists() {
            return async { Err(anyhow::anyhow!("Current working directory does not exist")) }
                .boxed();
        }
        let matcher = match state.regex_matcher_builder.build(query) {
            Ok(matcher) => {
                // Clear any "Failed to compile regex" errors out of the statusline.
                editor.clear_status();
                matcher
            }
            Err(err) => {
                log::info!(
                    "Failed to compile search pattern in workspace symbol search: {}",
                    err
                );
                return async { Err(anyhow::anyhow!("Failed to compile regex")) }.boxed();
            }
        };
        let pattern = Arc::new(pattern);
        let injector = injector.clone();
        let loader = editor.syn_loader.load();
        let documents: HashSet<_> = editor
            .documents()
            .filter_map(Document::path)
            .cloned()
            .collect();
        async move {
            let searcher = state.searcher_builder.build();
            state.walk_builder.build_parallel().run(|| {
                let mut searcher = searcher.clone();
                let matcher = matcher.clone();
                let injector = injector.clone();
                let loader = loader.clone();
                let documents = &documents;
                let pattern = pattern.clone();
                let syntax_cache = &state.syntax_cache;
                Box::new(move |entry: Result<DirEntry, ignore::Error>| -> WalkState {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(_) => return WalkState::Continue,
                    };
                    if !entry.path().is_file() {
                        return WalkState::Continue;
                    }
                    let path = entry.path();
                    // If this document is open, skip it because we've already processed it above.
                    if documents.contains(path) {
                        return WalkState::Continue;
                    };
                    let mut quit = false;
                    let sink = sinks::UTF8(|_line, _content| {
                        if !syntax_cache.contains_key(path) {
                            // Read the file into a Rope and attempt to recognize the language
                            // and parse it with tree-sitter. Save the Rope and Syntax for future
                            // queries.
                            syntax_cache.insert(path.to_path_buf(), syntax_for_path(path, &loader));
                        };
                        let entry = syntax_cache.get(path).unwrap();
                        let Some((text, syntax)) = entry.value() else {
                            // If the file couldn't be parsed, move on.
                            return Ok(false);
                        };
                        let uri = Uri::from(path::normalize(path));
                        for tag in tags_iter(
                            syntax,
                            &loader,
                            text.slice(..),
                            UriOrDocumentId::Uri(uri),
                            Some(&pattern),
                        ) {
                            if injector.push(tag).is_err() {
                                quit = true;
                                break;
                            }
                        }
                        // Quit after seeing the first regex match. We only care to find files
                        // that contain the pattern and then we run the tags query within
                        // those. The location and contents of a match are irrelevant - it's
                        // only important _if_ a file matches.
                        Ok(false)
                    });
                    if let Err(err) = searcher.search_path(&matcher, path, sink) {
                        log::info!("Workspace syntax search error: {}, {}", path.display(), err);
                    }
                    if quit {
                        WalkState::Quit
                    } else {
                        WalkState::Continue
                    }
                })
            });
            Ok(())
        }
        .boxed()
    };
    let picker = Picker::new(
        columns,
        1, // name
        [],
        state,
        move |cx, tag, action| {
            let doc_id = match &tag.doc {
                UriOrDocumentId::Id(id) => *id,
                UriOrDocumentId::Uri(uri) => match cx.editor.open(uri.as_path().expect(""), action) {
                    Ok(id) => id,
                    Err(e) => {
                        cx.editor
                            .set_error(format!("Failed to open file '{uri:?}': {e}"));
                        return;
                    }
                }
            };
            let doc = doc_mut!(cx.editor, &doc_id);
            let view = view_mut!(cx.editor);
            let len_chars = doc.text().len_chars();
            if tag.start >= len_chars || tag.end > len_chars {
                cx.editor.set_error("The location you jumped to does not exist anymore because the file has changed.");
                return;
            }
            doc.set_selection(view.id, Selection::single(tag.start, tag.end));
            if action.align_view(view, doc.id()) {
                align_view(doc, view, Align::Center)
            }
        },
    )
    .with_dynamic_query(get_tags, Some(275))
    .with_preview(move |_editor, tag| {
        Some((
            tag.doc.path_or_id()?,
            Some((tag.start_line, tag.end_line)),
        ))
    })
    .with_history_register(Some(reg))
    .truncate_start(false);
    cx.push_layer(Box::new(overlaid(picker)));
}

/// Create a Rope and language config for a given existing path without creating a full Document.
fn syntax_for_path(path: &Path, loader: &Loader) -> Option<(Rope, Syntax)> {
    let mut file = std::fs::File::open(path).ok()?;
    let (rope, _encoding, _has_bom) = from_reader(&mut file, None).ok()?;
    let text = rope.slice(..);
    let language = loader
        .language_for_filename(path)
        .or_else(|| loader.language_for_shebang(text))?;
    Syntax::new(text, language, loader)
        .ok()
        .map(|syntax| (rope, syntax))
}
