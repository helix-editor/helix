use super::{Prompt, Tree, TreeItem, TreeOp};
use crate::{
    compositor::{Component, Compositor, Context, EventResult},
    ctrl, key, shift, ui,
};
use anyhow::{bail, ensure, Result};
use helix_core::Position;
use helix_view::{
    editor::{Action, ExplorerPositionEmbed},
    graphics::{CursorKind, Modifier, Rect},
    input::{Event, KeyEvent},
    Editor,
};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use tui::{
    buffer::Buffer as Surface,
    text::{Span, Spans},
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
    Dir,
    Exe,
    Placeholder,
    Parent,
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

    fn parent(path: &Path) -> Self {
        let p = path.parent().unwrap_or_else(|| Path::new(""));
        Self {
            file_type: FileType::Parent,
            path: p.to_path_buf(),
        }
    }

    fn get_text(&self) -> Cow<'static, str> {
        match self.file_type {
            FileType::Parent => "..".into(),
            FileType::Placeholder => "---".into(),
            FileType::Root => return format!("{}", self.path.display()).into(),
            FileType::File | FileType::Exe | FileType::Dir => self
                .path
                .file_name()
                .map_or("/".into(), |p| p.to_string_lossy().into_owned().into()),
        }
    }
}

impl TreeItem for FileInfo {
    type Params = State;

    fn text(&self, cx: &mut Context, selected: bool, state: &mut State) -> Spans {
        let text = self.get_text();
        let theme = &cx.editor.theme;

        let style = match self.file_type {
            FileType::Parent | FileType::Dir | FileType::Root => "ui.explorer.dir",
            FileType::File | FileType::Exe | FileType::Placeholder => "ui.explorer.file",
        };
        let mut style = theme.try_get(style).unwrap_or_else(|| theme.get("ui.text"));
        if selected {
            let patch = match state.focus {
                true => "ui.explorer.focus",
                false => "ui.explorer.unfocus",
            };
            if let Some(patch) = theme.try_get(patch) {
                style = style.patch(patch);
            } else {
                style = style.add_modifier(Modifier::REVERSED);
            }
        }
        Spans::from(Span::styled(text, style))
    }

    fn is_child(&self, other: &Self) -> bool {
        if let FileType::Parent = other.file_type {
            return false;
        }
        if let FileType::Placeholder = self.file_type {
            self.path == other.path
        } else {
            self.path.parent().map_or(false, |p| p == other.path)
        }
    }

    fn cmp(&self, other: &Self) -> Ordering {
        use FileType::*;
        match (self.file_type, other.file_type) {
            (Parent, _) => return Ordering::Less,
            (_, Parent) => return Ordering::Greater,
            (Root, _) => return Ordering::Less,
            (_, Root) => return Ordering::Greater,
            _ => {}
        };

        if self.path == other.path {
            match (self.file_type, other.file_type) {
                (_, Placeholder) => return Ordering::Less,
                (Placeholder, _) => return Ordering::Greater,
                _ => {}
            };
        }

        if let (Some(p1), Some(p2)) = (self.path.parent(), other.path.parent()) {
            if p1 == p2 {
                match (self.file_type, other.file_type) {
                    (Dir, File | Exe) => return Ordering::Less,
                    (File | Exe, Dir) => return Ordering::Greater,
                    _ => {}
                };
            }
        }
        self.path.cmp(&other.path)
    }

    fn get_childs(&self) -> Result<Vec<Self>> {
        match self.file_type {
            FileType::Root | FileType::Dir => {}
            _ => return Ok(vec![]),
        };
        let mut ret: Vec<_> = std::fs::read_dir(&self.path)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                entry.metadata().ok().map(|meta| {
                    let is_exe = false;
                    let file_type = match (meta.is_dir(), is_exe) {
                        (true, _) => FileType::Dir,
                        (_, false) => FileType::File,
                        (_, true) => FileType::Exe,
                    };
                    Self {
                        file_type,
                        path: self.path.join(entry.file_name()),
                    }
                })
            })
            .collect();
        if ret.is_empty() {
            ret.push(Self {
                path: self.path.clone(),
                file_type: FileType::Placeholder,
            })
        }
        Ok(ret)
    }

    fn filter(&self, _cx: &mut Context, s: &str, _params: &mut Self::Params) -> bool {
        if s.is_empty() {
            false
        } else {
            self.get_text().contains(s)
        }
    }

    fn text_string(&self) -> String {
        self.get_text().to_string()
    }
}

// #[derive(Default, Debug, Clone)]
// struct PathState {
//     root: PathBuf,
//     sub_items: Vec<FileInfo>,
//     selected: usize,
//     save_view: (usize, usize), // (selected, row)
//     row: usize,
//     col: usize,
//     max_len: usize,
// }

// impl PathState {

//     fn mkdir(&mut self, dir: &str) -> Result<()> {
//         self.new_path(dir, FileType::Dir)
//     }

//     fn create_file(&mut self, f: &str) -> Result<()> {
//         self.new_path(f, FileType::File)
//     }

//     fn remove_current_file(&mut self) -> Result<()> {
//         let item = &self.sub_items[self.selected];
//         std::fs::remove_file(item.path_with_root(&self.root))?;
//         self.sub_items.remove(self.selected);
//         if self.selected >= self.sub_items.len() {
//             self.selected = self.sub_items.len() - 1;
//         }
//         Ok(())
//     }

// }

#[derive(Clone, Copy, Debug)]
enum PromptAction {
    Search(bool), // search next/search pre
    Mkdir,
    CreateFile,
    RemoveDir,
    RemoveFile,
    Filter,
}

#[derive(Clone, Debug)]
struct State {
    focus: bool,
    current_root: PathBuf,
}

impl State {
    fn new(focus: bool, current_root: PathBuf) -> Self {
        Self {
            focus,
            current_root,
        }
    }
}

pub struct Explorer {
    tree: Tree<FileInfo>,
    state: State,
    prompt: Option<(PromptAction, Prompt)>,
    #[allow(clippy::type_complexity)]
    on_next_key: Option<Box<dyn FnMut(&mut Context, &mut Self, KeyEvent) -> EventResult>>,
    #[allow(clippy::type_complexity)]
    repeat_motion: Option<Box<dyn FnMut(&mut Self, PromptAction, &mut Context) + 'static>>,
}

impl Explorer {
    pub fn new(cx: &mut Context) -> Result<Self> {
        let current_root = std::env::current_dir().unwrap_or_else(|_| "./".into());
        let items = Self::get_items(current_root.clone(), cx)?;
        Ok(Self {
            tree: Tree::build_tree(items).with_enter_fn(Self::toggle_current),
            state: State::new(true, current_root),
            repeat_motion: None,
            prompt: None,
            on_next_key: None,
        })
    }

    pub fn focus_current_file(&mut self, cx: &mut Context) {
        let current_document_path = doc!(cx.editor).path().cloned();
        match current_document_path {
            None => cx.editor.set_error("No opened document."),
            Some(path) => {
                self.tree.focus_path(cx, path, &self.state.current_root);
                self.focus();
            }
        }
    }

    // pub fn new_with_uri(uri: String) -> Result<Self> {
    //     // support remote file?

    //     let p = Path::new(&uri);
    //     ensure!(p.exists(), "path: {uri} is not exist");
    //     ensure!(p.is_dir(), "path: {uri} is not dir");
    //     Ok(Self::default().with_list(get_sub(p, None)?))
    // }

    pub fn focus(&mut self) {
        self.state.focus = true;
    }

    pub fn unfocus(&mut self) {
        self.state.focus = false;
    }

    pub fn is_focus(&self) -> bool {
        self.state.focus
    }

    fn get_items(p: PathBuf, cx: &mut Context) -> Result<Vec<FileInfo>> {
        let mut items = vec![FileInfo::parent(p.as_path())];
        let root = FileInfo::root(p);
        let childs = root.get_childs()?;
        if cx.editor.config().explorer.is_tree() {
            items.push(root)
        }
        items.extend(childs);
        Ok(items)
    }

    fn render_preview(&mut self, area: Rect, surface: &mut Surface, editor: &Editor) {
        if area.height <= 2 || area.width < 60 {
            return;
        }
        let item = self.tree.current().item();
        if item.file_type == FileType::Placeholder {
            return;
        }
        let head_area = render_block(area.clip_bottom(area.height - 2), surface, Borders::BOTTOM);
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
        if let Ok(preview_content) = get_preview(&item.path, body_area.height as usize) {
            preview_content
                .into_iter()
                .enumerate()
                .for_each(|(row, line)| {
                    surface.set_stringn(
                        body_area.x,
                        body_area.y + row as u16,
                        line,
                        body_area.width as usize,
                        style,
                    );
                })
        }
    }

    fn new_search_prompt(&mut self, search_next: bool) {
        self.tree.save_view();
        self.prompt = Some((
            PromptAction::Search(search_next),
            Prompt::new("search: ".into(), None, ui::completers::none, |_, _, _| {}),
        ))
    }

    fn new_filter_prompt(&mut self) {
        self.tree.save_view();
        self.prompt = Some((
            PromptAction::Filter,
            Prompt::new("filter: ".into(), None, ui::completers::none, |_, _, _| {}),
        ))
    }

    fn new_mkdir_prompt(&mut self) {
        self.prompt = Some((
            PromptAction::Mkdir,
            Prompt::new("mkdir: ".into(), None, ui::completers::none, |_, _, _| {}),
        ));
    }

    fn new_create_file_prompt(&mut self) {
        self.prompt = Some((
            PromptAction::CreateFile,
            Prompt::new(
                "create file: ".into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            ),
        ));
    }

    fn new_remove_file_prompt(&mut self, cx: &mut Context) {
        let item = self.tree.current_item();
        let check = || {
            ensure!(item.file_type != FileType::Placeholder, "The path is empty");
            ensure!(
                item.file_type != FileType::Parent,
                "can not remove parent dir"
            );
            ensure!(item.path.is_file(), "The path is not a file");
            let doc = cx.editor.document_by_path(&item.path);
            ensure!(doc.is_none(), "The file is opened");
            Ok(())
        };
        if let Err(e) = check() {
            cx.editor.set_error(format!("{e}"));
            return;
        }
        let p = format!("remove file: {}, YES? ", item.path.display());
        self.prompt = Some((
            PromptAction::RemoveFile,
            Prompt::new(p.into(), None, ui::completers::none, |_, _, _| {}),
        ));
    }

    fn new_remove_dir_prompt(&mut self, cx: &mut Context) {
        let item = self.tree.current_item();
        let check = || {
            ensure!(item.file_type != FileType::Placeholder, "The path is empty");
            ensure!(
                item.file_type != FileType::Parent,
                "can not remove parent dir"
            );
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
        let p = format!("remove dir: {}, YES? ", item.path.display());
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
        if item.file_type == FileType::Placeholder {
            return TreeOp::Noop;
        }
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
            if cx.editor.config().explorer.is_list() || item.file_type == FileType::Parent {
                match Self::get_items(item.path.clone(), cx) {
                    Ok(items) => {
                        state.current_root = item.path.clone();
                        return TreeOp::ReplaceTree(items);
                    }
                    Err(e) => cx.editor.set_error(format!("{e}")),
                }
            } else {
                return TreeOp::GetChildsAndInsert;
            }
        }
        cx.editor.set_error("unkonw file type");
        TreeOp::Noop
    }

    fn render_float(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        let background = cx.editor.theme.get("ui.background");
        let column_width = cx.editor.config().explorer.column_width as u16;
        surface.clear_with(area, background);
        let area = render_block(area, surface, Borders::ALL);

        let mut preview_area = area.clip_left(column_width + 1);
        if let Some((_, prompt)) = self.prompt.as_mut() {
            let area = preview_area.clip_bottom(2);
            let promp_area =
                render_block(preview_area.clip_top(area.height), surface, Borders::TOP);
            prompt.render(promp_area, surface, cx);
            preview_area = area;
        }
        self.render_preview(preview_area, surface, cx.editor);

        let list_area = render_block(area.clip_right(preview_area.width), surface, Borders::RIGHT);
        self.tree.render(list_area, surface, cx, &mut self.state);
    }

    pub fn render_embed(
        &mut self,
        area: Rect,
        surface: &mut Surface,
        cx: &mut Context,
        position: &ExplorerPositionEmbed,
    ) {
        let config = &cx.editor.config().explorer;

        let width = area.width.min(config.column_width as u16 + 2);

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
        self.tree.render(list_area, surface, cx, &mut self.state);

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
            const PREVIEW_AREA_MAX_HEIGHT: u16 = 25;
            let preview_area_width = (area.width - side_area.width).min(PREVIEW_AREA_MAX_WIDTH);
            let preview_area_height = area.height.min(PREVIEW_AREA_MAX_HEIGHT);

            let preview_area = match position {
                ExplorerPositionEmbed::Left => area.clip_left(side_area.width).clip_bottom(2),
                ExplorerPositionEmbed::Right => (Rect {
                    x: area.width - side_area.width - preview_area_width,
                    ..area
                })
                .clip_right(side_area.width)
                .clip_bottom(2),
            };
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
                self.tree.clean_recycle();
                return self
                    .tree
                    .handle_event(Event::Key(event), cx, &mut self.state);
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
            PromptAction::Search(search_next) => search_next,
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
                        if let PromptAction::Search(is_next) = action {
                            explorer.tree.save_view();
                            if is_next == search_next {
                                explorer
                                    .tree
                                    .search_next(cx, &search_str, &mut explorer.state);
                            } else {
                                explorer
                                    .tree
                                    .search_pre(cx, &search_str, &mut explorer.state);
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
                        self.tree.search_pre(cx, prompt.line(), &mut self.state);
                    }
                }
                self.prompt = Some((action, prompt));
            }
        };
        EventResult::Consumed(None)
    }

    fn handle_prompt_event(&mut self, event: KeyEvent, cx: &mut Context) -> EventResult {
        match &self.prompt {
            Some((PromptAction::Search(_), _)) => return self.handle_search_event(event, cx),
            Some((PromptAction::Filter, _)) => return self.handle_filter_event(event, cx),
            _ => {}
        };
        let (action, mut prompt) = match self.prompt.take() {
            Some((action, p)) => (action, p),
            _ => return EventResult::Ignored(None),
        };
        let line = prompt.line();
        match (action, event.into()) {
            (PromptAction::Mkdir, key!(Enter)) => {
                if let Err(e) = self.new_path(line, true) {
                    cx.editor.set_error(format!("{e}"))
                }
            }
            (PromptAction::CreateFile, key!(Enter)) => {
                if let Err(e) = self.new_path(line, false) {
                    cx.editor.set_error(format!("{e}"))
                }
            }
            (PromptAction::RemoveDir, key!(Enter)) => {
                let item = self.tree.current_item();
                if let Err(e) = std::fs::remove_dir_all(&item.path) {
                    cx.editor.set_error(format!("{e}"));
                } else {
                    self.tree.fold_current_child();
                    self.tree.remove_current();
                }
            }
            (PromptAction::RemoveFile, key!(Enter)) => {
                if line == "YES" {
                    let item = self.tree.current_item();
                    if let Err(e) = std::fs::remove_file(&item.path) {
                        cx.editor.set_error(format!("{e}"));
                    } else {
                        self.tree.remove_current();
                    }
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

    fn new_path(&mut self, file_name: &str, is_dir: bool) -> Result<()> {
        let current = self.tree.current_item();
        let current_parent = if current.file_type == FileType::Placeholder {
            &current.path
        } else {
            current
                .path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("can not get parent dir"))?
        };
        let p = helix_core::path::get_normalized_path(&current_parent.join(file_name));
        match p.parent() {
            Some(p) if p == current_parent => {}
            _ => bail!("The file name is not illegal"),
        };

        let f = if is_dir {
            std::fs::create_dir(&p)?;
            FileInfo::new(p, FileType::Dir)
        } else {
            let mut fd = std::fs::OpenOptions::new();
            fd.create_new(true).write(true).open(&p)?;
            FileInfo::new(p, FileType::File)
        };
        if current.file_type == FileType::Placeholder {
            self.tree.replace_current(f);
        } else {
            self.tree.insert_current_level(f);
        }
        Ok(())
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

        let close_fn = EventResult::Consumed(Some(Box::new(|compositor: &mut Compositor, _| {
            if let Some(editor) = compositor.find::<ui::EditorView>() {
                editor.explorer = None;
            }
        })));

        match key_event.into() {
            key!(Esc) => self.unfocus(),
            ctrl!('c') => return close_fn,
            key!('n') => {
                if let Some(mut repeat_motion) = self.repeat_motion.take() {
                    repeat_motion(self, PromptAction::Search(true), cx);
                    self.repeat_motion = Some(repeat_motion);
                }
            }
            shift!('N') => {
                if let Some(mut repeat_motion) = self.repeat_motion.take() {
                    repeat_motion(self, PromptAction::Search(false), cx);
                    self.repeat_motion = Some(repeat_motion);
                }
            }
            key!('b') => {
                if let Some(p) = self.state.current_root.parent() {
                    match Self::get_items(p.to_path_buf(), cx) {
                        Ok(items) => {
                            self.state.current_root = p.to_path_buf();
                            self.tree = Tree::build_tree(items).with_enter_fn(Self::toggle_current);
                        }
                        Err(e) => cx.editor.set_error(format!("{e}")),
                    }
                }
            }
            key!('f') => self.new_filter_prompt(),
            key!('/') => self.new_search_prompt(true),
            key!('?') => self.new_search_prompt(false),
            key!('m') => {
                self.on_next_key = Some(Box::new(|_, explorer, event| {
                    match event.into() {
                        key!('d') => explorer.new_mkdir_prompt(),
                        key!('f') => explorer.new_create_file_prompt(),
                        _ => return EventResult::Ignored(None),
                    };
                    EventResult::Consumed(None)
                }));
            }
            key!('r') => {
                self.on_next_key = Some(Box::new(|cx, explorer, event| {
                    match event.into() {
                        key!('d') => explorer.new_remove_dir_prompt(cx),
                        key!('f') => explorer.new_remove_file_prompt(cx),
                        _ => return EventResult::Ignored(None),
                    };
                    EventResult::Consumed(None)
                }));
            }
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
            let colw = config.column_width as u16;
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
