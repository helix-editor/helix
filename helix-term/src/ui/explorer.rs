use super::{Prompt, TreeOp, TreeView, TreeViewItem};
use crate::{
    compositor::{Component, Context, EventResult},
    ctrl, key, shift, ui,
};
use anyhow::{ensure, Result};
use helix_core::Position;
use helix_view::{
    editor::{Action, ExplorerPositionEmbed},
    graphics::{CursorKind, Rect},
    info::Info,
    input::{Event, KeyEvent},
    theme::Modifier,
    DocumentId, Editor,
};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::{borrow::Cow, fs::DirEntry};
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, Borders, Widget},
};

macro_rules! get_theme {
    ($theme: expr, $s1: expr, $s2: expr) => {
        $theme.try_get($s1).unwrap_or_else(|| $theme.get($s2))
    };
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
enum FileType {
    File,
    Folder,
    Root,
}

#[derive(PartialEq, Eq, Debug, Clone)]
struct FileInfo {
    file_type: FileType,
    path: PathBuf,
}

impl FileInfo {
    fn root(path: PathBuf) -> Self {
        Self {
            file_type: FileType::Root,
            path,
        }
    }

    fn get_text(&self) -> Cow<'static, str> {
        match self.file_type {
            FileType::Root => return format!("{}", self.path.display()).into(),
            FileType::File | FileType::Folder => self
                .path
                .file_name()
                .map_or("/".into(), |p| p.to_string_lossy().into_owned().into()),
        }
    }
}

impl PartialOrd for FileInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        use FileType::*;
        match (self.file_type, other.file_type) {
            (Root, _) => return Ordering::Less,
            (_, Root) => return Ordering::Greater,
            _ => {}
        };

        if let (Some(p1), Some(p2)) = (self.path.parent(), other.path.parent()) {
            if p1 == p2 {
                match (self.file_type, other.file_type) {
                    (Folder, File) => return Ordering::Less,
                    (File, Folder) => return Ordering::Greater,
                    _ => {}
                };
            }
        }
        self.path.cmp(&other.path)
    }
}

impl TreeViewItem for FileInfo {
    type Params = State;

    fn get_children(&self) -> Result<Vec<Self>> {
        match self.file_type {
            FileType::Root | FileType::Folder => {}
            _ => return Ok(vec![]),
        };
        let ret: Vec<_> = std::fs::read_dir(&self.path)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| dir_entry_to_file_info(entry, &self.path))
            .collect();
        Ok(ret)
    }

    fn name(&self) -> String {
        self.get_text().to_string()
    }

    fn is_parent(&self) -> bool {
        match self.file_type {
            FileType::Folder | FileType::Root => true,
            _ => false,
        }
    }
}

fn dir_entry_to_file_info(entry: DirEntry, path: &PathBuf) -> Option<FileInfo> {
    entry.metadata().ok().map(|meta| {
        let file_type = match meta.is_dir() {
            true => FileType::Folder,
            false => FileType::File,
        };
        FileInfo {
            file_type,
            path: path.join(entry.file_name()),
        }
    })
}

#[derive(Clone, Debug)]
enum PromptAction {
    CreateFolder { folder_path: PathBuf },
    CreateFile { folder_path: PathBuf },
    RemoveDir,
    RemoveFile(Option<DocumentId>),
    RenameFile(Option<DocumentId>),
}

#[derive(Clone, Debug)]
struct State {
    focus: bool,
    open: bool,
    current_root: PathBuf,
    area_width: u16,
    filter: String,
}

impl State {
    fn new(focus: bool, current_root: PathBuf) -> Self {
        Self {
            focus,
            current_root,
            open: true,
            area_width: 0,
            filter: "".to_string(),
        }
    }
}

pub struct Explorer {
    tree: TreeView<FileInfo>,
    history: Vec<TreeView<FileInfo>>,
    show_help: bool,
    state: State,
    prompt: Option<(PromptAction, Prompt)>,
    #[allow(clippy::type_complexity)]
    on_next_key: Option<Box<dyn FnMut(&mut Context, &mut Self, &KeyEvent) -> EventResult>>,
    column_width: u16,
}

impl Explorer {
    pub fn new(cx: &mut Context) -> Result<Self> {
        let current_root = std::env::current_dir().unwrap_or_else(|_| "./".into());
        Ok(Self {
            tree: Self::new_tree_view(current_root.clone())?,
            history: vec![],
            show_help: true,
            state: State::new(true, current_root),
            prompt: None,
            on_next_key: None,
            column_width: cx.editor.config().explorer.column_width as u16,
        })
    }

    fn new_tree_view(root: PathBuf) -> Result<TreeView<FileInfo>> {
        let root = FileInfo::root(root.clone());
        let children = root.get_children()?;
        Ok(TreeView::build_tree(root, children).with_enter_fn(Self::toggle_current))
    }

    fn push_history(&mut self, tree_view: TreeView<FileInfo>) {
        self.history.push(tree_view);
        const MAX_HISTORY_SIZE: usize = 20;
        Vec::truncate(&mut self.history, MAX_HISTORY_SIZE)
    }

    fn change_root(&mut self, cx: &mut Context, root: PathBuf) {
        if self.state.current_root.eq(&root) {
            return;
        }
        match Self::new_tree_view(root.clone()) {
            Ok(tree) => {
                let old_tree = std::mem::replace(&mut self.tree, tree);
                self.push_history(old_tree);
                self.state.current_root = root;
            }
            Err(e) => cx.editor.set_error(format!("{e}")),
        }
    }

    fn reveal_file(&mut self, path: PathBuf) -> Result<()> {
        let current_root = &self.state.current_root;
        let current_path = path.as_path().to_string_lossy().to_string();
        let current_root = current_root.as_path().to_string_lossy().to_string() + "/";
        let segments = current_path
            .strip_prefix(current_root.as_str())
            .expect(
                format!(
                    "Failed to strip prefix '{}' from '{}'",
                    current_root, current_path
                )
                .as_str(),
            )
            .split(std::path::MAIN_SEPARATOR)
            .collect::<Vec<_>>();
        self.tree.reveal_item(segments, &self.state.filter)?;
        Ok(())
    }

    pub fn reveal_current_file(&mut self, cx: &mut Context) -> Result<()> {
        self.focus();
        let current_document_path = doc!(cx.editor).path().cloned();
        match current_document_path {
            None => Ok(()),
            Some(current_path) => self.reveal_file(current_path),
        }
    }

    pub fn focus(&mut self) {
        self.state.focus = true;
        self.state.open = true;
    }

    pub fn unfocus(&mut self) {
        self.state.focus = false;
    }

    pub fn close(&mut self) {
        self.state.focus = false;
        self.state.open = false;
    }

    pub fn is_focus(&self) -> bool {
        self.state.focus
    }

    fn render_preview(&mut self, area: Rect, surface: &mut Surface, editor: &Editor) {
        let item = self.tree.current().item();
        let head_area = render_block(
            area.clip_bottom(area.height.saturating_sub(2)),
            surface,
            Borders::BOTTOM,
        );
        let path_str = format!("{}", item.path.display());
        surface.set_stringn(
            head_area.x,
            head_area.y,
            path_str,
            head_area.width as usize,
            get_theme!(editor.theme, "ui.explorer.dir", "ui.text"),
        );

        let body_area = area.clip_top(2);
        let style = editor.theme.get("ui.text");
        let content = get_preview(&item.path, body_area.height as usize)
            .unwrap_or_else(|err| vec![err.to_string()]);
        content.into_iter().enumerate().for_each(|(row, line)| {
            surface.set_stringn(
                body_area.x,
                body_area.y + row as u16,
                line,
                body_area.width as usize,
                style,
            );
        })
    }

    fn new_create_folder_prompt(&mut self) -> Result<()> {
        let folder_path = self.nearest_folder()?;
        self.prompt = Some((
            PromptAction::CreateFolder {
                folder_path: folder_path.clone(),
            },
            Prompt::new(
                format!(" New folder: {}/", folder_path.to_string_lossy()).into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            ),
        ));
        Ok(())
    }

    fn new_create_file_prompt(&mut self) -> Result<()> {
        let folder_path = self.nearest_folder()?;
        self.prompt = Some((
            PromptAction::CreateFile {
                folder_path: folder_path.clone(),
            },
            Prompt::new(
                format!(" New file: {}/", folder_path.to_string_lossy()).into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            ),
        ));
        Ok(())
    }

    fn nearest_folder(&self) -> Result<PathBuf> {
        let current = self.tree.current();
        if current.item().is_parent() {
            Ok(current.item().path.to_path_buf())
        } else {
            let parent_path = current.item().path.parent().ok_or_else(|| {
                anyhow::anyhow!(format!(
                    "Unable to get parent path of '{}'",
                    current.item().path.to_string_lossy()
                ))
            })?;
            Ok(parent_path.to_path_buf())
        }
    }

    fn new_remove_prompt(&mut self, cx: &mut Context) {
        let item = self.tree.current().item();
        match item.file_type {
            FileType::Folder => self.new_remove_dir_prompt(cx),
            FileType::File => self.new_remove_file_prompt(cx),
            FileType::Root => cx.editor.set_error("Root is not removable"),
        }
    }

    fn new_rename_prompt(&mut self, cx: &mut Context) {
        let path = self.tree.current_item().path.clone();
        self.prompt = Some((
            PromptAction::RenameFile(cx.editor.document_by_path(&path).map(|doc| doc.id())),
            Prompt::new(
                format!(" Rename to ").into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            )
            .with_line(path.to_string_lossy().to_string(), cx.editor),
        ));
    }

    fn new_remove_file_prompt(&mut self, cx: &mut Context) {
        let item = self.tree.current_item();
        let check = || {
            ensure!(item.path.is_file(), "The path is not a file");
            let doc = cx.editor.document_by_path(&item.path);
            Ok(doc.map(|doc| doc.id()))
        };
        match check() {
            Err(err) => cx.editor.set_error(format!("{err}")),
            Ok(document_id) => {
                let p = format!(" Delete file: '{}'? y/n: ", item.path.display());
                self.prompt = Some((
                    PromptAction::RemoveFile(document_id),
                    Prompt::new(p.into(), None, ui::completers::none, |_, _, _| {}),
                ));
            }
        }
    }

    fn new_remove_dir_prompt(&mut self, cx: &mut Context) {
        let item = self.tree.current_item();
        let check = || {
            ensure!(item.path.is_dir(), "The path is not a dir");
            let doc = cx.editor.documents().find(|doc| {
                doc.path()
                    .map(|p| p.starts_with(&item.path))
                    .unwrap_or(false)
            });
            ensure!(doc.is_none(), "There are files opened under the dir");
            Ok(())
        };
        if let Err(e) = check() {
            cx.editor.set_error(format!("{e}"));
            return;
        }
        let p = format!(" Delete folder: '{}'? y/n: ", item.path.display());
        self.prompt = Some((
            PromptAction::RemoveDir,
            Prompt::new(p.into(), None, ui::completers::none, |_, _, _| {}),
        ));
    }

    fn toggle_current(item: &mut FileInfo, cx: &mut Context, state: &mut State) -> TreeOp {
        if item.path == Path::new("") {
            return TreeOp::Noop;
        }
        let meta = match std::fs::metadata(&item.path) {
            Ok(meta) => meta,
            Err(e) => {
                cx.editor.set_error(format!("{e}"));
                return TreeOp::Noop;
            }
        };
        if meta.is_file() {
            if let Err(e) = cx.editor.open(&item.path, Action::Replace) {
                cx.editor.set_error(format!("{e}"));
            }
            state.focus = false;
            return TreeOp::Noop;
        }

        if item.path.is_dir() {
            return TreeOp::GetChildsAndInsert;
        }
        cx.editor.set_error("unkonw file type");
        TreeOp::Noop
    }

    fn render_float(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let background = cx.editor.theme.get("ui.background");
        surface.clear_with(area, background);
        let area = render_block(area, surface, Borders::ALL);

        let mut preview_area = area.clip_left(self.column_width + 1);
        if let Some((_, prompt)) = self.prompt.as_mut() {
            let area = preview_area.clip_bottom(2);
            let promp_area =
                render_block(preview_area.clip_top(area.height), surface, Borders::TOP);
            prompt.render(promp_area, surface, cx);
            preview_area = area;
        }
        if self.show_help {
            self.render_help(preview_area, surface, cx);
        } else {
            self.render_preview(preview_area, surface, cx.editor);
        }

        let list_area = render_block(area.clip_right(preview_area.width), surface, Borders::RIGHT);
        self.render_tree(list_area, surface, cx)
    }

    fn render_tree(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let title_style = cx.editor.theme.get("ui.text");
        let title_style = if self.is_focus() {
            title_style.add_modifier(Modifier::BOLD)
        } else {
            title_style
        };
        surface.set_stringn(
            area.x,
            area.y,
            "Explorer: press ? for help",
            area.width.into(),
            title_style,
        );
        self.tree
            .render(area.clip_top(1), surface, cx, &self.state.filter);
    }

    pub fn render_embed(
        &mut self,
        area: Rect,
        surface: &mut Surface,
        cx: &mut Context,
        position: &ExplorerPositionEmbed,
    ) {
        if !self.state.open {
            return;
        }
        let width = area.width.min(self.column_width + 2);

        self.state.area_width = area.width;

        let side_area = match position {
            ExplorerPositionEmbed::Left => Rect { width, ..area },
            ExplorerPositionEmbed::Right => Rect {
                x: area.width - width,
                width,
                ..area
            },
        }
        .clip_bottom(1);
        let background = cx.editor.theme.get("ui.background");
        surface.clear_with(side_area, background);

        let prompt_area = area.clip_top(side_area.height);

        let list_area = match position {
            ExplorerPositionEmbed::Left => {
                render_block(side_area.clip_left(1), surface, Borders::RIGHT).clip_bottom(1)
            }
            ExplorerPositionEmbed::Right => {
                render_block(side_area.clip_right(1), surface, Borders::LEFT).clip_bottom(1)
            }
        };
        self.render_tree(list_area, surface, cx);

        {
            let statusline = if self.is_focus() {
                cx.editor.theme.get("ui.statusline")
            } else {
                cx.editor.theme.get("ui.statusline.inactive")
            };
            let area = side_area.clip_top(list_area.height);
            let area = match position {
                ExplorerPositionEmbed::Left => area.clip_right(1),
                ExplorerPositionEmbed::Right => area.clip_left(1),
            };
            surface.clear_with(area, statusline);
        }

        if self.is_focus() {
            if self.show_help {
                let help_area = match position {
                    ExplorerPositionEmbed::Left => area,
                    ExplorerPositionEmbed::Right => {
                        area.clip_right(list_area.width.saturating_add(2))
                    }
                };
                self.render_help(help_area, surface, cx);
            } else {
                const PREVIEW_AREA_MAX_WIDTH: u16 = 90;
                const PREVIEW_AREA_MAX_HEIGHT: u16 = 30;
                let preview_area_width =
                    (area.width.saturating_sub(side_area.width)).min(PREVIEW_AREA_MAX_WIDTH);
                let preview_area_height = area.height.min(PREVIEW_AREA_MAX_HEIGHT);

                let preview_area = match position {
                    ExplorerPositionEmbed::Left => area.clip_left(side_area.width),
                    ExplorerPositionEmbed::Right => (Rect {
                        x: area
                            .width
                            .saturating_sub(side_area.width)
                            .saturating_sub(preview_area_width),
                        ..area
                    })
                    .clip_right(side_area.width),
                }
                .clip_bottom(2);
                if preview_area.width < 30 || preview_area.height < 3 {
                    return;
                }
                let y = self.tree.winline().saturating_sub(1) as u16;
                let y = if (preview_area_height + y) > preview_area.height {
                    preview_area.height.saturating_sub(preview_area_height)
                } else {
                    y
                };
                let area = Rect::new(preview_area.x, y, preview_area_width, preview_area_height);
                surface.clear_with(area, background);
                let area = render_block(area, surface, Borders::all());
                self.render_preview(area, surface, cx.editor);
            }
        }

        if let Some((_, prompt)) = self.prompt.as_mut() {
            prompt.render_prompt(prompt_area, surface, cx)
        }
    }

    fn render_help(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        Info::new(
            "Explorer",
            &[
                ("?", "Toggle help"),
                ("a", "Add file"),
                ("A", "Add folder"),
                ("r", "Rename file/folder"),
                ("d", "Delete file"),
                ("b", "Change root to parent folder"),
                ("]", "Change root to current folder"),
                ("[", "Go to previous root"),
                ("+", "Increase size"),
                ("-", "Decrease size"),
                ("q", "Close"),
            ]
            .into_iter()
            .chain(ui::tree::tree_view_help().into_iter())
            .collect::<Vec<_>>(),
        )
        .render(area, surface, cx)
    }

    fn handle_prompt_event(&mut self, event: &KeyEvent, cx: &mut Context) -> EventResult {
        fn handle_prompt_event(
            explorer: &mut Explorer,
            event: &KeyEvent,
            cx: &mut Context,
        ) -> Result<EventResult> {
            let (action, mut prompt) = match explorer.prompt.take() {
                Some((action, p)) => (action, p),
                _ => return Ok(EventResult::Ignored(None)),
            };
            let line = prompt.line();
            match (&action, event) {
                (PromptAction::CreateFolder { folder_path }, key!(Enter)) => {
                    explorer.new_path(folder_path.clone(), line, true)?
                }
                (PromptAction::CreateFile { folder_path }, key!(Enter)) => {
                    explorer.new_path(folder_path.clone(), line, false)?
                }
                (PromptAction::RemoveDir, key!(Enter)) => {
                    if line == "y" {
                        let item = explorer.tree.current_item();
                        std::fs::remove_dir_all(&item.path)?;
                        explorer.tree.refresh()?;
                    }
                }
                (PromptAction::RemoveFile(document_id), key!(Enter)) => {
                    if line == "y" {
                        let item = explorer.tree.current_item();
                        std::fs::remove_file(&item.path).map_err(anyhow::Error::from)?;
                        explorer.tree.refresh()?;
                        if let Some(id) = document_id {
                            cx.editor.close_document(*id, true)?
                        }
                    }
                }
                (PromptAction::RenameFile(document_id), key!(Enter)) => {
                    let item = explorer.tree.current_item();
                    std::fs::rename(&item.path, line)?;
                    explorer.tree.refresh()?;
                    explorer.reveal_file(PathBuf::from(line))?;
                    if let Some(id) = document_id {
                        cx.editor.close_document(*id, true)?
                    }
                }
                (_, key!(Esc) | ctrl!('c')) => {}
                _ => {
                    prompt.handle_event(&Event::Key(*event), cx);
                    explorer.prompt = Some((action, prompt));
                }
            }
            Ok(EventResult::Consumed(None))
        }
        match handle_prompt_event(self, event, cx) {
            Ok(event_result) => event_result,
            Err(err) => {
                cx.editor.set_error(err.to_string());
                EventResult::Consumed(None)
            }
        }
    }

    fn new_path(&mut self, current_parent: PathBuf, file_name: &str, is_dir: bool) -> Result<()> {
        let path = helix_core::path::get_normalized_path(&current_parent.join(file_name));

        if is_dir {
            std::fs::create_dir_all(&path)?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut fd = std::fs::OpenOptions::new();
            fd.create_new(true).write(true).open(&path)?;
        };
        self.reveal_file(path)
    }

    fn toggle_help(&mut self) {
        self.show_help = !self.show_help
    }

    fn go_to_previous_root(&mut self) {
        if let Some(tree) = self.history.pop() {
            self.tree = tree
        }
    }

    pub fn is_opened(&self) -> bool {
        self.state.open
    }

    pub fn column_width(&self) -> u16 {
        self.column_width
    }

    fn increase_size(&mut self) {
        const EDITOR_MIN_WIDTH: u16 = 10;
        self.column_width = std::cmp::min(
            self.state.area_width.saturating_sub(EDITOR_MIN_WIDTH),
            self.column_width.saturating_add(1),
        )
    }

    fn decrease_size(&mut self) {
        self.column_width = self.column_width.saturating_sub(1)
    }
}

impl Component for Explorer {
    /// Process input events, return true if handled.
    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        let filter = self.state.filter.clone();
        if self.tree.prompting() {
            return self.tree.handle_event(event, cx, &mut self.state, &filter);
        }
        let key_event = match event {
            Event::Key(event) => event,
            Event::Resize(..) => return EventResult::Consumed(None),
            _ => return EventResult::Ignored(None),
        };
        if !self.is_focus() {
            return EventResult::Ignored(None);
        }
        if let Some(mut on_next_key) = self.on_next_key.take() {
            return on_next_key(cx, self, key_event);
        }

        if let EventResult::Consumed(c) = self.handle_prompt_event(key_event, cx) {
            return EventResult::Consumed(c);
        }

        match key_event {
            key!(Esc) => self.unfocus(),
            key!('q') => self.close(),
            key!('?') => self.toggle_help(),
            key!('a') => {
                if let Err(error) = self.new_create_file_prompt() {
                    cx.editor.set_error(error.to_string())
                }
            }
            shift!('A') => {
                if let Err(error) = self.new_create_folder_prompt() {
                    cx.editor.set_error(error.to_string())
                }
            }
            key!('b') => {
                if let Some(parent) = self.state.current_root.parent().clone() {
                    let path = parent.to_path_buf();
                    self.change_root(cx, path)
                }
            }
            key!(']') => self.change_root(cx, self.tree.current_item().path.clone()),
            key!('[') => self.go_to_previous_root(),
            key!('d') => self.new_remove_prompt(cx),
            key!('r') => self.new_rename_prompt(cx),
            key!('-') => self.decrease_size(),
            key!('+') => self.increase_size(),
            _ => {
                self.tree
                    .handle_event(&Event::Key(*key_event), cx, &mut self.state, &filter);
            }
        }

        EventResult::Consumed(None)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        if area.width < 10 || area.height < 5 {
            cx.editor.set_error("explorer render area is too small");
            return;
        }
        let config = &cx.editor.config().explorer;
        if let Some(position) = config.is_embed() {
            self.render_embed(area, surface, cx, &position);
        } else {
            self.render_float(area, surface, cx);
        }
    }

    fn cursor(&self, area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
        let prompt = match self.prompt.as_ref() {
            Some((_, prompt)) => prompt,
            None => return (None, CursorKind::Hidden),
        };
        let config = &editor.config().explorer;
        let (x, y) = if config.is_overlay() {
            let colw = self.column_width as u16;
            if area.width > colw {
                (area.x + colw + 2, area.y + area.height.saturating_sub(2))
            } else {
                return (None, CursorKind::Hidden);
            }
        } else {
            (area.x, area.y + area.height.saturating_sub(1))
        };
        prompt.cursor(Rect::new(x, y, area.width, 1), editor)
    }
}

fn get_preview(p: impl AsRef<Path>, max_line: usize) -> Result<Vec<String>> {
    let p = p.as_ref();
    if p.is_dir() {
        let mut entries = p
            .read_dir()?
            .filter_map(|entry| {
                entry
                    .ok()
                    .map(|entry| dir_entry_to_file_info(entry, &p.to_path_buf()))
                    .flatten()
            })
            .take(max_line)
            .collect::<Vec<_>>();

        entries.sort();

        return Ok(entries
            .into_iter()
            .map(|entry| match entry.file_type {
                FileType::Folder => format!("{}/", entry.name()),
                _ => entry.name(),
            })
            .collect());
    }

    ensure!(p.is_file(), "path: {} is not file or dir", p.display());
    use std::fs::OpenOptions;
    use std::io::BufRead;
    let mut fd = OpenOptions::new();
    fd.read(true);
    let fd = fd.open(p)?;
    Ok(std::io::BufReader::new(fd)
        .lines()
        .take(max_line)
        .filter_map(|line| line.ok())
        .map(|line| line.replace('\t', "    "))
        .collect())
}

fn render_block(area: Rect, surface: &mut Surface, borders: Borders) -> Rect {
    let block = Block::default().borders(borders);
    let inner = block.inner(area);
    block.render(area, surface);
    inner
}

#[cfg(test)]
mod test_explore {}
