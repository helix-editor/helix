use super::{Prompt, TreeItem, TreeOp, TreeView};
use crate::{
    compositor::{Component, Compositor, Context, EventResult},
    ctrl, key, shift, ui,
};
use anyhow::{bail, ensure, Result};
use helix_core::Position;
use helix_view::{
    editor::{Action, ExplorerPositionEmbed},
    graphics::{CursorKind, Rect},
    input::{Event, KeyEvent},
    Editor,
};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, Borders, Widget},
};

macro_rules! get_theme {
    ($theme: expr, $s1: expr, $s2: expr) => {
        $theme.try_get($s1).unwrap_or_else(|| $theme.get($s2))
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FileType {
    File,
    Folder,
    Root,
}

#[derive(Debug, Clone)]
struct FileInfo {
    file_type: FileType,
    path: PathBuf,
}

impl FileInfo {
    fn new(path: PathBuf, file_type: FileType) -> Self {
        Self { path, file_type }
    }

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

impl TreeItem for FileInfo {
    type Params = State;

    fn is_child(&self, other: &Self) -> bool {
        self.path.parent().map_or(false, |p| p == other.path)
    }

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

    fn get_children(&self) -> Result<Vec<Self>> {
        match self.file_type {
            FileType::Root | FileType::Folder => {}
            _ => return Ok(vec![]),
        };
        let ret: Vec<_> = std::fs::read_dir(&self.path)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                entry.metadata().ok().map(|meta| {
                    let file_type = match meta.is_dir() {
                        true => FileType::Folder,
                        false => FileType::File,
                    };
                    Self {
                        file_type,
                        path: self.path.join(entry.file_name()),
                    }
                })
            })
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

#[derive(Clone, Debug)]
enum PromptAction {
    Search {
        search_next: bool,
    }, // search next/search pre
    CreateFolder {
        folder_path: PathBuf,
        parent_index: usize,
    },
    CreateFile {
        folder_path: PathBuf,
        parent_index: usize,
    },
    RemoveDir,
    RemoveFile,
    RenameFile,
    Filter,
}

#[derive(Clone, Debug)]
struct State {
    focus: bool,
    open: bool,
    current_root: PathBuf,
    area_width: u16,
}

impl State {
    fn new(focus: bool, current_root: PathBuf) -> Self {
        Self {
            focus,
            current_root,
            open: true,
            area_width: 0,
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
    on_next_key: Option<Box<dyn FnMut(&mut Context, &mut Self, KeyEvent) -> EventResult>>,
    #[allow(clippy::type_complexity)]
    repeat_motion: Option<Box<dyn FnMut(&mut Self, PromptAction, &mut Context) + 'static>>,
    column_width: u16,
}

impl Explorer {
    pub fn new(cx: &mut Context) -> Result<Self> {
        let current_root = std::env::current_dir().unwrap_or_else(|_| "./".into());
        Ok(Self {
            tree: Self::new_tree_view(current_root.clone())?,
            history: vec![],
            show_help: false,
            state: State::new(true, current_root),
            repeat_motion: None,
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

    fn reveal_file(&mut self, cx: &mut Context, path: PathBuf) {
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
        match self.tree.reveal_item(segments) {
            Ok(_) => {
                self.focus();
            }
            Err(error) => cx.editor.set_error(error.to_string()),
        }
    }

    pub fn reveal_current_file(&mut self, cx: &mut Context) {
        let current_document_path = doc!(cx.editor).path().cloned();
        match current_document_path {
            None => cx.editor.set_error("No opened document."),
            Some(current_path) => self.reveal_file(cx, current_path),
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
        // if area.height <= 2 || area.width < 60 {
        //     return;
        // }
        let item = self.tree.current().item();
        let head_area = render_block(area.clip_bottom(area.height - 2), surface, Borders::BOTTOM);
        let path_str = format!("{}", item.path.display());
        surface.set_stringn(
            head_area.x,
            head_area.y,
            if self.show_help {
                "[HELP]".to_string()
            } else {
                path_str
            },
            head_area.width as usize,
            get_theme!(editor.theme, "ui.explorer.dir", "ui.text"),
        );

        let body_area = area.clip_top(2);
        let style = editor.theme.get("ui.text");
        let content = if self.show_help {
            vec![
                "?    Toggle help",
                "a    Add file",
                "A    Add folder",
                "r    Rename file/folder",
                "d    Delete file",
                "/    Search",
                "f    Filter",
                "[    Change root to parent folder",
                "]    Change root to current folder",
                "^o   Go to previous root",
                "R    Refresh",
                "+    Increase size",
                "-    Decrease size",
                "q    Close",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .chain(ui::tree::tree_view_help())
            .collect()
        } else {
            get_preview(&item.path, body_area.height as usize)
                .unwrap_or_else(|err| vec![err.to_string()])
        };
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

    fn new_search_prompt(&mut self, search_next: bool) {
        self.tree.save_view();
        self.prompt = Some((
            PromptAction::Search { search_next },
            Prompt::new(" Search: ".into(), None, ui::completers::none, |_, _, _| {}),
        ))
    }

    fn new_filter_prompt(&mut self) {
        self.tree.save_view();
        self.prompt = Some((
            PromptAction::Filter,
            Prompt::new(" Filter: ".into(), None, ui::completers::none, |_, _, _| {}),
        ))
    }

    fn new_create_folder_prompt(&mut self) -> Result<()> {
        let (parent_index, folder_path) = self.nearest_folder()?;
        self.prompt = Some((
            PromptAction::CreateFolder {
                parent_index,
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
        let (parent_index, folder_path) = self.nearest_folder()?;
        self.prompt = Some((
            PromptAction::CreateFile {
                parent_index,
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

    fn nearest_folder(&self) -> Result<(usize, PathBuf)> {
        let current = self.tree.current();
        if current.item().is_parent() {
            Ok((current.index(), current.item().path.to_path_buf()))
        } else {
            let parent_index = current.parent_index().ok_or_else(|| {
                anyhow::anyhow!(format!(
                    "Unable to get parent index of '{}'",
                    current.item().path.to_string_lossy()
                ))
            })?;
            let parent_path = current.item().path.parent().ok_or_else(|| {
                anyhow::anyhow!(format!(
                    "Unable to get parent path of '{}'",
                    current.item().path.to_string_lossy()
                ))
            })?;
            Ok((parent_index, parent_path.to_path_buf()))
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

    fn new_rename_prompt(&mut self) {
        let name = self.tree.current_item().path.to_string_lossy();
        self.prompt = Some((
            PromptAction::RenameFile,
            Prompt::new(
                format!(" Rename to ").into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            )
            .with_line(name.to_string()),
        ));
    }

    fn new_remove_file_prompt(&mut self, cx: &mut Context) {
        let item = self.tree.current_item();
        let check = || {
            ensure!(item.path.is_file(), "The path is not a file");
            let doc = cx.editor.document_by_path(&item.path);
            ensure!(doc.is_none(), "The file is opened");
            Ok(())
        };
        if let Err(e) = check() {
            cx.editor.set_error(format!("{e}"));
            return;
        }
        let p = format!(" Delete file: '{}'? y/n: ", item.path.display());
        self.prompt = Some((
            PromptAction::RemoveFile,
            Prompt::new(p.into(), None, ui::completers::none, |_, _, _| {}),
        ));
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

    fn toggle_current(
        item: &mut FileInfo,
        cx: &mut Context,
        state: &mut State,
    ) -> TreeOp<FileInfo> {
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
        self.render_preview(preview_area, surface, cx.editor);

        let list_area = render_block(area.clip_right(preview_area.width), surface, Borders::RIGHT);
        surface.set_stringn(
            list_area.x,
            list_area.y,
            " Explorer: press ? for help",
            list_area.width.into(),
            cx.editor.theme.get("ui.text"),
        );
        self.tree
            .render(list_area.clip_top(1), surface, cx, &mut self.state);
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
        surface.set_stringn(
            list_area.x.saturating_sub(1),
            list_area.y,
            " Explorer: press ? for help",
            list_area.width.into(),
            cx.editor.theme.get("ui.text"),
        );
        self.tree
            .render(list_area.clip_top(1), surface, cx, &mut self.state);

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
            // surface.set_string_truncated(
            //     area.x,
            //     area.y,
            //     &self.path_state.root.to_string_lossy(),
            //     area.width as usize,
            //     |_| statusline,
            //     true,
            //     true,
            // );
        }

        if self.is_focus() {
            const PREVIEW_AREA_MAX_WIDTH: u16 = 90;
            const PREVIEW_AREA_MAX_HEIGHT: u16 = 30;
            let preview_area_width = (area.width - side_area.width).min(PREVIEW_AREA_MAX_WIDTH);
            let preview_area_height = area.height.min(PREVIEW_AREA_MAX_HEIGHT);

            let preview_area = match position {
                ExplorerPositionEmbed::Left => area.clip_left(side_area.width),
                ExplorerPositionEmbed::Right => (Rect {
                    x: area.width - side_area.width - preview_area_width,
                    ..area
                })
                .clip_right(side_area.width),
            }
            .clip_bottom(2);
            if preview_area.width < 30 || preview_area.height < 3 {
                return;
            }
            let y = self.tree.row().saturating_sub(1) as u16;
            let y = if (preview_area_height + y) > preview_area.height {
                preview_area.height - preview_area_height
            } else {
                y
            };
            let area = Rect::new(preview_area.x, y, preview_area_width, preview_area_height);
            surface.clear_with(area, background);
            let area = render_block(area, surface, Borders::all());
            self.render_preview(area, surface, cx.editor);
        }

        if let Some((_, prompt)) = self.prompt.as_mut() {
            prompt.render_prompt(prompt_area, surface, cx)
        }
    }

    fn handle_filter_event(&mut self, event: KeyEvent, cx: &mut Context) -> EventResult {
        let (action, mut prompt) = self.prompt.take().unwrap();
        match event.into() {
            key!(Tab) | key!(Down) | ctrl!('j') => {
                self.tree.clean_recycle();
                return self
                    .tree
                    .handle_event(Event::Key(event), cx, &mut self.state);
            }
            key!(Enter) => {
                if let EventResult::Consumed(_) = prompt.handle_event(Event::Key(event), cx) {
                    self.tree.filter(prompt.line(), cx, &mut self.state);
                }
            }
            key!(Esc) | ctrl!('c') => self.tree.restore_recycle(),
            _ => {
                if let EventResult::Consumed(_) = prompt.handle_event(Event::Key(event), cx) {
                    self.tree.filter(prompt.line(), cx, &mut self.state);
                }
                self.prompt = Some((action, prompt));
            }
        };
        EventResult::Consumed(None)
    }

    fn handle_search_event(&mut self, event: KeyEvent, cx: &mut Context) -> EventResult {
        let (action, mut prompt) = self.prompt.take().unwrap();
        let search_next = match action {
            PromptAction::Search { search_next } => search_next,
            _ => return EventResult::Ignored(None),
        };
        match event.into() {
            key!(Tab) | key!(Down) | ctrl!('j') => {
                return self
                    .tree
                    .handle_event(Event::Key(event), cx, &mut self.state)
            }
            key!(Enter) => {
                let search_str = prompt.line().clone();
                if !search_str.is_empty() {
                    self.repeat_motion = Some(Box::new(move |explorer, action, cx| {
                        if let PromptAction::Search {
                            search_next: is_next,
                        } = action
                        {
                            explorer.tree.save_view();
                            if is_next == search_next {
                                explorer
                                    .tree
                                    .search_next(cx, &search_str, &mut explorer.state);
                            } else {
                                explorer
                                    .tree
                                    .search_previous(cx, &search_str, &mut explorer.state);
                            }
                        }
                    }))
                } else {
                    self.repeat_motion = None;
                }
                // return self
                //     .tree
                //     .handle_event(Event::Key(event), cx, &mut self.state);
            }
            key!(Esc) | ctrl!('c') => self.tree.restore_view(),
            _ => {
                if let EventResult::Consumed(_) = prompt.handle_event(Event::Key(event), cx) {
                    if search_next {
                        self.tree.search_next(cx, prompt.line(), &mut self.state);
                    } else {
                        self.tree
                            .search_previous(cx, prompt.line(), &mut self.state);
                    }
                }
                self.prompt = Some((action, prompt));
            }
        };
        EventResult::Consumed(None)
    }

    fn handle_prompt_event(&mut self, event: KeyEvent, cx: &mut Context) -> EventResult {
        match &self.prompt {
            Some((PromptAction::Search { .. }, _)) => return self.handle_search_event(event, cx),
            Some((PromptAction::Filter, _)) => return self.handle_filter_event(event, cx),
            _ => {}
        };
        let (action, mut prompt) = match self.prompt.take() {
            Some((action, p)) => (action, p),
            _ => return EventResult::Ignored(None),
        };
        let line = prompt.line();
        match (&action, event.into()) {
            (
                PromptAction::CreateFolder {
                    folder_path,
                    parent_index,
                },
                key!(Enter),
            ) => {
                if let Err(e) = self.new_path(folder_path.clone(), line, true, *parent_index) {
                    cx.editor.set_error(format!("{e}"))
                }
            }
            (
                PromptAction::CreateFile {
                    folder_path,
                    parent_index,
                },
                key!(Enter),
            ) => {
                if let Err(e) = self.new_path(folder_path.clone(), line, false, *parent_index) {
                    cx.editor.set_error(format!("{e}"))
                }
            }
            (PromptAction::RemoveDir, key!(Enter)) => {
                if line == "y" {
                    let item = self.tree.current_item();
                    if let Err(e) = std::fs::remove_dir_all(&item.path) {
                        cx.editor.set_error(format!("{e}"));
                    } else {
                        self.tree.fold_current_child();
                        self.tree.remove_current();
                    }
                }
            }
            (PromptAction::RemoveFile, key!(Enter)) => {
                if line == "y" {
                    let item = self.tree.current_item();
                    if let Err(e) = std::fs::remove_file(&item.path) {
                        cx.editor.set_error(format!("{e}"));
                    } else {
                        self.tree.remove_current();
                    }
                }
            }
            (PromptAction::RenameFile, key!(Enter)) => {
                let item = self.tree.current_item();
                if let Err(e) = std::fs::rename(&item.path, line) {
                    cx.editor.set_error(format!("{e}"));
                } else {
                    self.tree.remove_current();
                    self.reveal_file(cx, PathBuf::from(line))
                }
            }
            (_, key!(Esc) | ctrl!('c')) => {}
            _ => {
                prompt.handle_event(Event::Key(event), cx);
                self.prompt = Some((action, prompt));
            }
        }
        EventResult::Consumed(None)
    }

    fn new_path(
        &mut self,
        current_parent: PathBuf,
        file_name: &str,
        is_dir: bool,
        parent_index: usize,
    ) -> Result<()> {
        let path = helix_core::path::get_normalized_path(&current_parent.join(file_name));
        match path.parent() {
            Some(p) if p == current_parent => {}
            _ => bail!("The file name is not illegal"),
        };

        let file = if is_dir {
            std::fs::create_dir(&path)?;
            FileInfo::new(path, FileType::Folder)
        } else {
            let mut fd = std::fs::OpenOptions::new();
            fd.create_new(true).write(true).open(&path)?;
            FileInfo::new(path, FileType::File)
        };
        self.tree.add_child(parent_index, file)?;
        Ok(())
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
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
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

        match key_event.into() {
            key!(Esc) => self.unfocus(),
            key!('q') => self.close(),
            key!('n') => {
                if let Some(mut repeat_motion) = self.repeat_motion.take() {
                    repeat_motion(self, PromptAction::Search { search_next: true }, cx);
                    self.repeat_motion = Some(repeat_motion);
                }
            }
            shift!('N') => {
                if let Some(mut repeat_motion) = self.repeat_motion.take() {
                    repeat_motion(self, PromptAction::Search { search_next: false }, cx);
                    self.repeat_motion = Some(repeat_motion);
                }
            }
            key!('f') => self.new_filter_prompt(),
            key!('/') => self.new_search_prompt(true),
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
            key!('[') => {
                if let Some(parent) = self.state.current_root.parent().clone() {
                    self.change_root(cx, parent.to_path_buf())
                }
            }
            key!(']') => self.change_root(cx, self.tree.current_item().path.clone()),
            ctrl!('o') => self.go_to_previous_root(),
            key!('d') => self.new_remove_prompt(cx),
            key!('r') => self.new_rename_prompt(),
            shift!('R') => {
                if let Err(error) = self.tree.refresh() {
                    cx.editor.set_error(error.to_string())
                }
            }
            key!('-') => self.decrease_size(),
            key!('+') => self.increase_size(),
            _ => {
                self.tree
                    .handle_event(Event::Key(key_event), cx, &mut self.state);
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
                (area.x + colw + 2, area.y + area.height - 2)
            } else {
                return (None, CursorKind::Hidden);
            }
        } else {
            (area.x, area.y + area.height - 1)
        };
        prompt.cursor(Rect::new(x, y, area.width, 1), editor)
    }
}

fn get_preview(p: impl AsRef<Path>, max_line: usize) -> Result<Vec<String>> {
    let p = p.as_ref();
    if p.is_dir() {
        return Ok(p
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .take(max_line)
            .map(|entry| {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    format!("{}/", entry.file_name().to_string_lossy())
                } else {
                    format!("{}", entry.file_name().to_string_lossy())
                }
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
