use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use arc_swap::ArcSwapAny;
use dashmap::DashMap;
use futures_util::FutureExt;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{sinks, BinaryDetection, SearcherBuilder};
use helix_core::{
    syntax::{LanguageLayer, Loader},
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
        picker::{Injector, PathOrId, PreviewRange},
        Picker, PickerColumn,
    },
};

use super::Context;

// TODO: try to mirror lsp::SymbolKind or come up with our own
// internal symbol kinds and convert lsp::SymbolKind to that?
// TODO: move this to helix syntax?
#[derive(Debug)]
enum SymbolKind {
    Function,
    Macro,
    Module,
    Constant,
    Struct,
    Interface,
    Type,
    Class,
}

impl SymbolKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Macro => "macro",
            Self::Module => "module",
            Self::Constant => "constant",
            Self::Struct => "struct",
            Self::Interface => "interface",
            Self::Type => "type",
            Self::Class => "class",
        }
    }
}

fn definition_symbol_kind_for_capture(
    layer: &LanguageLayer,
    capture_index: usize,
) -> Option<SymbolKind> {
    match layer
        .config
        .symbols_query
        .as_ref()
        .expect("layer must have symbols query in order to match")
        .capture_names()[capture_index]
    {
        "definition.function" => Some(SymbolKind::Function),
        "definition.macro" => Some(SymbolKind::Macro),
        "definition.module" => Some(SymbolKind::Module),
        "definition.constant" => Some(SymbolKind::Constant),
        "definition.struct" => Some(SymbolKind::Struct),
        "definition.interface" => Some(SymbolKind::Interface),
        "definition.type" => Some(SymbolKind::Type),
        "definition.class" => Some(SymbolKind::Class),
        _ => None,
    }
}

#[derive(Debug, Clone)]
enum UriOrDocumentId {
    Uri(Arc<Uri>),
    Id(DocumentId),
}

impl From<DocumentId> for UriOrDocumentId {
    fn from(id: DocumentId) -> Self {
        Self::Id(id)
    }
}

impl From<Uri> for UriOrDocumentId {
    fn from(uri: Uri) -> Self {
        Self::Uri(uri.into())
    }
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
struct Symbol {
    kind: SymbolKind,
    name: String,
    start: usize,
    end: usize,
    doc: UriOrDocumentId,
}

pub fn syntax_symbol_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let Some(syntax) = doc.syntax() else {
        cx.editor
            .set_error("Syntax tree is not available on this buffer");
        return;
    };
    if !syntax.has_symbols_query() {
        cx.editor
            .set_error("Syntax-based symbols information is not available in this buffer");
        return;
    };

    let doc_id = doc.id();
    let text = doc.text();

    let columns = vec![
        PickerColumn::new("kind", |symbol: &Symbol, _| symbol.kind.as_str().into()),
        PickerColumn::new("name", |symbol: &Symbol, _| symbol.name.as_str().into()),
    ];

    let symbols =
        syntax
            .symbols(text.slice(..), None)
            .filter_map(|(layer, match_, capture_index)| {
                let capture = match_.captures[capture_index];
                let kind = definition_symbol_kind_for_capture(layer, capture.index as usize)?;
                let node = capture.node;

                let start = text.byte_to_char(node.start_byte());
                let end = text.byte_to_char(node.end_byte());
                Some(Symbol {
                    kind,
                    name: text.slice(start..end).to_string(),
                    start,
                    end,
                    doc: doc_id.into(),
                })
            });

    let picker = Picker::new(
        columns,
        1, // name
        symbols,
        (),
        move |cx, symbol, action| {
            cx.editor.switch(doc_id, action);
            let view = view_mut!(cx.editor);
            let doc = doc_mut!(cx.editor, &doc_id);
            doc.set_selection(view.id, Selection::single(symbol.start, symbol.end));
            if action.align_view(view, doc.id()) {
                align_view(doc, view, Align::Center)
            }
        },
    )
    .with_preview(move |_editor, symbol| {
        Some((
            symbol.doc.path_or_id()?,
            Some(PreviewRange::chars(symbol.start, symbol.end)),
        ))
    })
    .truncate_start(false);

    cx.push_layer(Box::new(overlaid(picker)));
}

pub fn syntax_workspace_symbol_picker(cx: &mut Context) {
    fn symbols_matching_pattern<'a>(
        syntax: &'a Syntax,
        text: RopeSlice<'a>,
        doc: &'a UriOrDocumentId,
        pattern: &'a rope::Regex,
    ) -> impl Iterator<Item = Symbol> + 'a {
        syntax
            .symbols(text, None)
            .filter_map(move |(layer, match_, capture_index)| {
                let capture = match_.captures[capture_index];
                let kind = definition_symbol_kind_for_capture(layer, capture.index as usize)?;
                let node = capture.node;
                if !pattern.is_match(text.regex_input_at_bytes(node.start_byte()..node.end_byte()))
                {
                    return None;
                }

                let start = text.byte_to_char(node.start_byte());
                let end = text.byte_to_char(node.end_byte());
                Some(Symbol {
                    kind,
                    name: text.slice(start..end).to_string(),
                    start,
                    end,
                    doc: doc.clone(),
                })
            })
    }

    #[derive(Debug)]
    struct SearchState {
        searcher_builder: SearcherBuilder,
        walk_builder: WalkBuilder,
        regex_matcher_builder: RegexMatcherBuilder,
        search_root: PathBuf,
        /// A cache of files that have been parsed in prior searches.
        syntax_cache: DashMap<PathBuf, Option<(Rope, Syntax)>>,
    }

    let mut searcher_builder = SearcherBuilder::new();
    searcher_builder.binary_detection(BinaryDetection::quit(b'\x00'));

    // Search from the workspace that the currently focused document is within. This behaves like global
    // search most of the time but helps when you have two projects open in splits.
    // TODO: investigate handling multiple workspaces better generally.
    let cwd = helix_stdx::env::current_working_dir();
    let search_root = if let Some(path) = doc!(cx.editor).path() {
        path.ancestors()
            .find_map(|ancestor| {
                (ancestor.join(".git").exists() || ancestor.join(".helix").exists())
                    .then(|| path::normalize(ancestor))
            })
            .unwrap_or(cwd)
    } else {
        cwd
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

    let state = SearchState {
        searcher_builder,
        walk_builder,
        regex_matcher_builder,
        search_root,
        syntax_cache: DashMap::default(),
    };

    let reg = cx.register.unwrap_or('/');
    cx.editor.registers.last_search_register = reg;

    let columns = vec![
        PickerColumn::new("kind", |symbol: &Symbol, _| symbol.kind.as_str().into()),
        PickerColumn::new("name", |symbol: &Symbol, _| symbol.name.as_str().into())
            .without_filtering(),
        PickerColumn::new("path", |symbol: &Symbol, state: &SearchState| {
            match &symbol.doc {
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

    let get_symbols = |query: &str,
                       editor: &mut Editor,
                       state: Arc<SearchState>,
                       injector: &Injector<_, _>| {
        if query.len() < 3 {
            return async { Ok(()) }.boxed();
        }

        // Attempt to find the symbol in any open documents.
        let pattern = match rope::Regex::new(query) {
            Ok(pattern) => pattern,
            Err(err) => return async { Err(anyhow::anyhow!(err)) }.boxed(),
        };
        for doc in editor.documents() {
            let Some(syntax) = doc.syntax() else { continue };
            let text = doc.text().slice(..);
            let uri_or_id = doc
                .uri()
                .map(|uri| uri.into())
                .unwrap_or_else(|| doc.id().into());

            for symbol in symbols_matching_pattern(syntax, text.slice(..), &uri_or_id, &pattern) {
                if injector.push(symbol).is_err() {
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

        let pattern = Arc::from(pattern);
        let injector = injector.clone();
        let loader = editor.syn_loader.clone();
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

                    match entry.file_type() {
                        Some(entry) if entry.is_file() => {}
                        // skip everything else
                        _ => return WalkState::Continue,
                    };

                    let path = entry.path();

                    // If this document is open, skip it because
                    // we've already processed it above.
                    if documents.contains(path) {
                        return WalkState::Continue;
                    };

                    let mut quit = false;
                    let sink = sinks::UTF8(|_line, _content| {
                        if !syntax_cache.contains_key(path) {
                            // Read the file into a Rope and attempt to recognize the language
                            // and parse it with tree-sitter.
                            // TODO: Also save the symbols? On the other hand, it's very cheap to query
                            // them and there may be many.
                            syntax_cache.insert(path.to_path_buf(), syntax_for_path(path, &loader));
                        };
                        // If the file couldn't be parsed, move on.
                        let entry = syntax_cache.get(path).unwrap();
                        let Some((text, syntax)) = entry.value() else {
                            return Ok(false);
                        };
                        let uri = Uri::from(path::normalize(path));

                        for symbol in
                            symbols_matching_pattern(syntax, text.slice(..), &uri.into(), &pattern)
                        {
                            if injector.push(symbol).is_err() {
                                quit = true;
                                break;
                            }
                        }

                        // Quit after seeing the first match. We only care to find
                        // files that contain the pattern and then we run the symbols
                        // query within those. The location and contents of a match
                        // are irrelevant - it's only important _if_ a file matches.
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
        move |cx, symbol, action| {
            let doc_id = match &symbol.doc {
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
            if symbol.start >= len_chars || symbol.end > len_chars {
                cx.editor.set_error("The location you jumped to does not exist anymore because the file has changed.");
                return;
            }

            doc.set_selection(view.id, Selection::single(symbol.start, symbol.end));
            if action.align_view(view, doc.id()) {
                align_view(doc, view, Align::Center)
            }
        },
    )
    .with_dynamic_query(get_symbols, Some(200))
    .with_preview(move |_editor, symbol| {
        Some((
            symbol.doc.path_or_id()?,
            Some(PreviewRange::chars(symbol.start, symbol.end)),
        ))
    })
    .truncate_start(false);

    cx.push_layer(Box::new(overlaid(picker)));
}

/// Create a [`Syntax`] for a given existing path without creating
/// a full [`Document`].
fn syntax_for_path(path: &Path, loader: &Arc<ArcSwapAny<Arc<Loader>>>) -> Option<(Rope, Syntax)> {
    let mut file = std::fs::File::open(path).ok()?;
    let (rope, _encoding, _has_bom) = from_reader(&mut file, None).ok()?;
    let text = rope.slice(..);
    let lang_config = loader
        .load()
        .language_config_for_file_name(path)
        .or_else(|| loader.load().language_config_for_shebang(text))?;
    let highlight_config = lang_config.highlight_config(&loader.load())?;
    Syntax::new(text, highlight_config, loader.clone()).map(|syntax| (rope, syntax))
}

// TODO: goto definition. Find the `reference.<whatever>` under the cursor then do a
// workspace search. Find all `definition.<whatever>` with the same name.

// TODO: goto references. Roughly the same as above but reversed: find all `reference.<whatever>`
// with the same name as the current `definition.<whatever>`.

// TODO: others?
// * LSP rename is nice because it's accurate.
//   Let's not have a syntax tree equivalent for
//   that because it would probably edit extraneous things.
// * LSP document highlight to set up multiple cursors
//   for all references in the current scope? Maybe left
//   for the future.
