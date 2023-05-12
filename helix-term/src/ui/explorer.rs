use super::{Prompt, TreeOp, TreeView, TreeViewItem};
use crate::{
    compositor::{Component, Context, EventResult},
    ctrl, key, shift, ui,
};
use anyhow::{bail, ensure, Result};
use helix_core::Position;
use helix_view::{
    editor::{Action, ExplorerPosition},
    graphics::{CursorKind, Rect},
    info::Info,
    input::{Event, KeyEvent},
    theme::Modifier,
    Editor,
};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::{borrow::Cow, fs::DirEntry};
use tui::{
    buffer::Buffer as Surface,
    widgets::{Block, Borders, Widget},
};

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
        let text = match self.file_type {
            FileType::Root => self.path.display().to_string(),
            FileType::File | FileType::Folder => self
                .path
                .file_name()
                .map_or("/".into(), |p| p.to_string_lossy().into_owned()),
        };

        #[cfg(test)]
        let text = text.replace(std::path::MAIN_SEPARATOR, "/");

        text.into()
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
        matches!(self.file_type, FileType::Folder | FileType::Root)
    }
}

fn dir_entry_to_file_info(entry: DirEntry, path: &Path) -> Option<FileInfo> {
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
    CreateFileOrFolder,
    RemoveFolder,
    RemoveFile,
    RenameFile,
}

#[derive(Clone, Debug, Default)]
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

struct ExplorerHistory {
    tree: TreeView<FileInfo>,
    current_root: PathBuf,
}

pub struct Explorer {
    tree: TreeView<FileInfo>,
    history: Vec<ExplorerHistory>,
    show_help: bool,
    state: State,
    prompt: Option<(PromptAction, Prompt)>,
    #[allow(clippy::type_complexity)]
    on_next_key: Option<Box<dyn FnMut(&mut Context, &mut Self, &KeyEvent) -> EventResult>>,
    column_width: u16,
}

impl Explorer {
    pub fn new(cx: &mut Context) -> Result<Self> {
        let current_root = std::env::current_dir()
            .unwrap_or_else(|_| "./".into())
            .canonicalize()?;
        Ok(Self {
            tree: Self::new_tree_view(current_root.clone())?,
            history: vec![],
            show_help: false,
            state: State::new(true, current_root),
            prompt: None,
            on_next_key: None,
            column_width: cx.editor.config().explorer.column_width as u16,
        })
    }

    #[cfg(test)]
    fn from_path(root: PathBuf, column_width: u16) -> Result<Self> {
        Ok(Self {
            tree: Self::new_tree_view(root.clone())?,
            history: vec![],
            show_help: false,
            state: State::new(true, root),
            prompt: None,
            on_next_key: None,
            column_width,
        })
    }

    fn new_tree_view(root: PathBuf) -> Result<TreeView<FileInfo>> {
        let root = FileInfo::root(root);
        Ok(TreeView::build_tree(root)?.with_enter_fn(Self::toggle_current))
    }

    fn push_history(&mut self, tree_view: TreeView<FileInfo>, current_root: PathBuf) {
        self.history.push(ExplorerHistory {
            tree: tree_view,
            current_root,
        });
        const MAX_HISTORY_SIZE: usize = 20;
        Vec::truncate(&mut self.history, MAX_HISTORY_SIZE)
    }

    fn change_root(&mut self, root: PathBuf) -> Result<()> {
        if self.state.current_root.eq(&root) {
            return Ok(());
        }
        let tree = Self::new_tree_view(root.clone())?;
        let old_tree = std::mem::replace(&mut self.tree, tree);
        self.push_history(old_tree, self.state.current_root.clone());
        self.state.current_root = root;
        Ok(())
    }

    pub fn reveal_file(&mut self, path: PathBuf) -> Result<()> {
        let current_root = &self.state.current_root.canonicalize()?;
        let current_path = &path.canonicalize()?;
        let segments = {
            let stripped = match current_path.strip_prefix(current_root) {
                Ok(stripped) => Ok(stripped),
                Err(_) => {
                    let parent = path.parent().ok_or_else(|| {
                        anyhow::anyhow!("Failed get parent of '{}'", current_path.to_string_lossy())
                    })?;
                    self.change_root(parent.into())?;
                    current_path
                        .strip_prefix(parent.canonicalize()?)
                        .map_err(|_| {
                            anyhow::anyhow!(
                                "Failed to strip prefix (parent) '{}' from '{}'",
                                parent.to_string_lossy(),
                                current_path.to_string_lossy()
                            )
                        })
                }
            }?;

            stripped
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
        };
        self.tree.reveal_item(segments)?;
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

    fn unfocus(&mut self) {
        self.state.focus = false;
    }

    fn close(&mut self) {
        self.state.focus = false;
        self.state.open = false;
    }

    pub fn is_focus(&self) -> bool {
        self.state.focus
    }

    fn new_create_file_or_folder_prompt(&mut self, cx: &mut Context) -> Result<()> {
        let folder_path = self.nearest_folder()?;
        self.prompt = Some((
            PromptAction::CreateFileOrFolder,
            Prompt::new(
                format!(
                    " New file or folder (ends with '{}'): ",
                    std::path::MAIN_SEPARATOR
                )
                .into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            )
            .with_line(format!("{}/", folder_path.to_string_lossy()), cx.editor),
        ));
        Ok(())
    }

    fn nearest_folder(&self) -> Result<PathBuf> {
        let current = self.tree.current()?.item();
        if current.is_parent() {
            Ok(current.path.to_path_buf())
        } else {
            let parent_path = current.path.parent().ok_or_else(|| {
                anyhow::anyhow!(format!(
                    "Unable to get parent path of '{}'",
                    current.path.to_string_lossy()
                ))
            })?;
            Ok(parent_path.to_path_buf())
        }
    }

    fn new_remove_prompt(&mut self) -> Result<()> {
        let item = self.tree.current()?.item();
        match item.file_type {
            FileType::Folder => self.new_remove_folder_prompt(),
            FileType::File => self.new_remove_file_prompt(),
            FileType::Root => bail!("Root is not removable"),
        }
    }

    fn new_rename_prompt(&mut self, cx: &mut Context) -> Result<()> {
        let path = self.tree.current_item()?.path.clone();
        self.prompt = Some((
            PromptAction::RenameFile,
            Prompt::new(
                " Rename to ".into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            )
            .with_line(path.to_string_lossy().to_string(), cx.editor),
        ));
        Ok(())
    }

    fn new_remove_file_prompt(&mut self) -> Result<()> {
        let item = self.tree.current_item()?;
        ensure!(
            item.path.is_file(),
            "The path '{}' is not a file",
            item.path.to_string_lossy()
        );
        self.prompt = Some((
            PromptAction::RemoveFile,
            Prompt::new(
                format!(" Delete file: '{}'? y/N: ", item.path.display()).into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            ),
        ));
        Ok(())
    }

    fn new_remove_folder_prompt(&mut self) -> Result<()> {
        let item = self.tree.current_item()?;
        ensure!(
            item.path.is_dir(),
            "The path '{}' is not a folder",
            item.path.to_string_lossy()
        );

        self.prompt = Some((
            PromptAction::RemoveFolder,
            Prompt::new(
                format!(" Delete folder: '{}'? y/N: ", item.path.display()).into(),
                None,
                ui::completers::none,
                |_, _, _| {},
            ),
        ));
        Ok(())
    }

    fn toggle_current(item: &mut FileInfo, cx: &mut Context, state: &mut State) -> TreeOp {
        (|| -> Result<TreeOp> {
            if item.path == Path::new("") {
                return Ok(TreeOp::Noop);
            }
            let meta = std::fs::metadata(&item.path)?;
            if meta.is_file() {
                cx.editor.open(&item.path, Action::Replace)?;
                state.focus = false;
                return Ok(TreeOp::Noop);
            }

            if item.path.is_dir() {
                return Ok(TreeOp::GetChildsAndInsert);
            }

            Err(anyhow::anyhow!("Unknown file type: {:?}", meta.file_type()))
        })()
        .unwrap_or_else(|err| {
            cx.editor.set_error(format!("{err}"));
            TreeOp::Noop
        })
    }

    fn render_tree(
        &mut self,
        area: Rect,
        prompt_area: Rect,
        surface: &mut Surface,
        cx: &mut Context,
    ) {
        self.tree.render(area, prompt_area, surface, cx);
    }

    fn render_embed(
        &mut self,
        area: Rect,
        surface: &mut Surface,
        cx: &mut Context,
        position: &ExplorerPosition,
    ) {
        if !self.state.open {
            return;
        }
        let width = area.width.min(self.column_width + 2);

        self.state.area_width = area.width;

        let side_area = match position {
            ExplorerPosition::Left => Rect { width, ..area },
            ExplorerPosition::Right => Rect {
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
            ExplorerPosition::Left => {
                render_block(side_area.clip_left(1), surface, Borders::RIGHT).clip_bottom(1)
            }
            ExplorerPosition::Right => {
                render_block(side_area.clip_right(1), surface, Borders::LEFT).clip_bottom(1)
            }
        };
        self.render_tree(list_area, prompt_area, surface, cx);

        {
            let statusline = if self.is_focus() {
                cx.editor.theme.get("ui.statusline")
            } else {
                cx.editor.theme.get("ui.statusline.inactive")
            };
            let area = side_area.clip_top(list_area.height);
            let area = match position {
                ExplorerPosition::Left => area.clip_right(1),
                ExplorerPosition::Right => area.clip_left(1),
            };
            surface.clear_with(area, statusline);

            let title_style = cx.editor.theme.get("ui.text");
            let title_style = if self.is_focus() {
                title_style.add_modifier(Modifier::BOLD)
            } else {
                title_style
            };
            surface.set_stringn(
                area.x,
                area.y,
                if self.is_focus() {
                    " EXPLORER: press ? for help"
                } else {
                    " EXPLORER"
                },
                area.width.into(),
                title_style,
            );
        }

        if self.is_focus() && self.show_help {
            let help_area = match position {
                ExplorerPosition::Left => area,
                ExplorerPosition::Right => area.clip_right(list_area.width.saturating_add(2)),
            };
            self.render_help(help_area, surface, cx);
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
                ("a", "Add file/folder"),
                ("r", "Rename file/folder"),
                ("d", "Delete file"),
                ("B", "Change root to parent folder"),
                ("]", "Change root to current folder"),
                ("[", "Go to previous root"),
                ("+, =", "Increase size"),
                ("-, _", "Decrease size"),
                ("q", "Close"),
            ]
            .into_iter()
            .chain(ui::tree::tree_view_help().into_iter())
            .collect::<Vec<_>>(),
        )
        .render(area, surface, cx)
    }

    fn handle_prompt_event(&mut self, event: &KeyEvent, cx: &mut Context) -> EventResult {
        let result = (|| -> Result<EventResult> {
            let (action, mut prompt) = match self.prompt.take() {
                Some((action, p)) => (action, p),
                _ => return Ok(EventResult::Ignored(None)),
            };
            let line = prompt.line();

            let current_item_path = self.tree.current_item()?.path.clone();
            match (&action, event) {
                (PromptAction::CreateFileOrFolder, key!(Enter)) => {
                    if line.ends_with(std::path::MAIN_SEPARATOR) {
                        self.new_folder(line)?
                    } else {
                        self.new_file(line)?
                    }
                }
                (PromptAction::RemoveFolder, key) => {
                    if let key!('y') = key {
                        close_documents(current_item_path, cx)?;
                        self.remove_folder()?;
                    }
                }
                (PromptAction::RemoveFile, key) => {
                    if let key!('y') = key {
                        close_documents(current_item_path, cx)?;
                        self.remove_file()?;
                    }
                }
                (PromptAction::RenameFile, key!(Enter)) => {
                    close_documents(current_item_path, cx)?;
                    self.rename_current(line)?;
                }
                (_, key!(Esc) | ctrl!('c')) => {}
                _ => {
                    prompt.handle_event(&Event::Key(*event), cx);
                    self.prompt = Some((action, prompt));
                }
            }
            Ok(EventResult::Consumed(None))
        })();
        match result {
            Ok(event_result) => event_result,
            Err(err) => {
                cx.editor.set_error(err.to_string());
                EventResult::Consumed(None)
            }
        }
    }

    fn new_file(&mut self, path: &str) -> Result<()> {
        let path = helix_core::path::get_normalized_path(&PathBuf::from(path));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut fd = std::fs::OpenOptions::new();
        fd.create_new(true).write(true).open(&path)?;
        self.reveal_file(path)
    }

    fn new_folder(&mut self, path: &str) -> Result<()> {
        let path = helix_core::path::get_normalized_path(&PathBuf::from(path));
        std::fs::create_dir_all(&path)?;
        self.reveal_file(path)
    }

    fn toggle_help(&mut self) {
        self.show_help = !self.show_help
    }

    fn go_to_previous_root(&mut self) {
        if let Some(history) = self.history.pop() {
            self.tree = history.tree;
            self.state.current_root = history.current_root
        }
    }

    fn change_root_to_current_folder(&mut self) -> Result<()> {
        self.change_root(self.tree.current_item()?.path.clone())
    }

    fn change_root_parent_folder(&mut self) -> Result<()> {
        if let Some(parent) = self.state.current_root.parent() {
            let path = parent.to_path_buf();
            self.change_root(path)
        } else {
            Ok(())
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

    fn rename_current(&mut self, line: &String) -> Result<()> {
        let item = self.tree.current_item()?;
        let path = PathBuf::from(line);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&item.path, &path)?;
        self.tree.refresh()?;
        self.reveal_file(path)
    }

    fn remove_folder(&mut self) -> Result<()> {
        let item = self.tree.current_item()?;
        std::fs::remove_dir_all(&item.path)?;
        self.tree.refresh()
    }

    fn remove_file(&mut self) -> Result<()> {
        let item = self.tree.current_item()?;
        std::fs::remove_file(&item.path)?;
        self.tree.refresh()
    }
}

fn close_documents(current_item_path: PathBuf, cx: &mut Context) -> Result<()> {
    let ids = cx
        .editor
        .documents
        .iter()
        .filter_map(|(id, doc)| {
            if doc.path()?.starts_with(&current_item_path) {
                Some(*id)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for id in ids {
        cx.editor.close_document(id, true)?;
    }
    Ok(())
}

impl Component for Explorer {
    /// Process input events, return true if handled.
    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        if self.tree.prompting() {
            return self.tree.handle_event(event, cx, &mut self.state);
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

        (|| -> Result<()> {
            match key_event {
                key!(Esc) => self.unfocus(),
                key!('q') => self.close(),
                key!('?') => self.toggle_help(),
                key!('a') => self.new_create_file_or_folder_prompt(cx)?,
                shift!('B') => self.change_root_parent_folder()?,
                key!(']') => self.change_root_to_current_folder()?,
                key!('[') => self.go_to_previous_root(),
                key!('d') => self.new_remove_prompt()?,
                key!('r') => self.new_rename_prompt(cx)?,
                key!('-') | key!('_') => self.decrease_size(),
                key!('+') | key!('=') => self.increase_size(),
                _ => {
                    self.tree
                        .handle_event(&Event::Key(*key_event), cx, &mut self.state);
                }
            };
            Ok(())
        })()
        .unwrap_or_else(|err| cx.editor.set_error(format!("{err}")));

        EventResult::Consumed(None)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        if area.width < 10 || area.height < 5 {
            cx.editor.set_error("explorer render area is too small");
            return;
        }
        let config = &cx.editor.config().explorer;
        let position = config.position;
        self.render_embed(area, surface, cx, &position);
    }

    fn cursor(&self, area: Rect, editor: &Editor) -> (Option<Position>, CursorKind) {
        if let Some(prompt) = self
            .prompt
            .as_ref()
            .map(|(_, prompt)| prompt)
            .or_else(|| self.tree.prompt())
        {
            let (x, y) = (area.x, area.y + area.height.saturating_sub(1));
            prompt.cursor(Rect::new(x, y, area.width, 1), editor)
        } else {
            (None, CursorKind::Hidden)
        }
    }
}

fn render_block(area: Rect, surface: &mut Surface, borders: Borders) -> Rect {
    let block = Block::default().borders(borders);
    let inner = block.inner(area);
    block.render(area, surface);
    inner
}

#[cfg(test)]
mod test_explorer {
    use crate::compositor::Component;

    use super::Explorer;
    use helix_view::graphics::Rect;
    use std::{fs, path::PathBuf};

    /// This code should create the following file tree:
    ///
    ///   <temp_path>
    ///   ├── index.html
    ///   ├── .gitignore
    ///   ├── scripts
    ///   │   └── main.js
    ///   └── styles
    ///      ├── style.css
    ///      └── public
    ///          └── file
    ///
    fn dummy_file_tree() -> PathBuf {
        let path = tempfile::tempdir().unwrap().path().to_path_buf();
        if path.exists() {
            fs::remove_dir_all(path.clone()).unwrap();
        }
        fs::create_dir_all(path.clone()).unwrap();
        fs::write(path.join("index.html"), "").unwrap();
        fs::write(path.join(".gitignore"), "").unwrap();

        fs::create_dir_all(path.join("scripts")).unwrap();
        fs::write(path.join("scripts").join("main.js"), "").unwrap();

        fs::create_dir_all(path.join("styles")).unwrap();
        fs::write(path.join("styles").join("style.css"), "").unwrap();

        fs::create_dir_all(path.join("styles").join("public")).unwrap();
        fs::write(path.join("styles").join("public").join("file"), "").unwrap();

        path
    }

    fn render(explorer: &mut Explorer) -> String {
        explorer.tree.render_to_string(Rect::new(0, 0, 100, 10))
    }

    fn new_explorer() -> (PathBuf, Explorer) {
        let path = dummy_file_tree();
        (path.clone(), Explorer::from_path(path, 100).unwrap())
    }

    #[test]
    fn test_reveal_file() {
        let (path, mut explorer) = new_explorer();

        let path_str = path.display().to_string();

        // 0a. Expect the "scripts" folder is not opened
        assert_eq!(
            render(&mut explorer),
            format!(
                "
({path_str})
⏵ scripts
⏵ styles
  .gitignore
  index.html
"
            )
            .trim()
        );

        // 1. Reveal "scripts/main.js"
        explorer.reveal_file(path.join("scripts/main.js")).unwrap();

        // 1a. Expect the "scripts" folder is opened, and "main.js" is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏷ [scripts]
    (main.js)
⏵ styles
  .gitignore
  index.html
"
            )
            .trim()
        );

        // 2. Change root to "scripts"
        explorer.tree.move_up(1);
        explorer.change_root_to_current_folder().unwrap();

        // 2a. Expect the current root is "scripts"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
({path_str}/scripts)
  main.js
"
            )
            .trim()
        );

        // 3. Reveal "styles/public/file", which is outside of the current root
        explorer
            .reveal_file(path.join("styles/public/file"))
            .unwrap();

        // 3a. Expect the current root is "public", and "file" is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}/styles/public]
  (file)
"
            )
            .trim()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_rename() {
        let (path, mut explorer) = new_explorer();
        let path_str = path.display().to_string();

        explorer.handle_events("jjj").unwrap();
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏵ styles
  (.gitignore)
  index.html
"
            )
            .trim()
        );

        // 0. Open the rename file prompt
        explorer.handle_events("r").unwrap();

        // 0.1 Expect the current prompt to be related to file renaming
        let prompt = &explorer.prompt.as_ref().unwrap().1;
        assert_eq!(prompt.prompt(), " Rename to ");
        assert_eq!(
            prompt.line().replace(std::path::MAIN_SEPARATOR, "/"),
            format!("{path_str}/.gitignore")
        );

        // 1. Rename the current file to a name that is lexicographically greater than "index.html"
        explorer.handle_events("<C-w>who.is<ret>").unwrap();

        // 1a. Expect the file is renamed, and is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏵ styles
  index.html
  (who.is)
"
            )
            .trim()
        );

        assert!(path.join("who.is").exists());

        // 2. Rename the current file into an existing folder
        explorer
            .handle_events(&format!(
                "r<C-w>styles{}lol<ret>",
                std::path::MAIN_SEPARATOR
            ))
            .unwrap();

        // 2a. Expect the file is moved to the folder, and is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏷ [styles]
  ⏵ public
    (lol)
    style.css
  index.html
"
            )
            .trim()
        );

        assert!(path.join("styles/lol").exists());

        // 3. Rename the current file into a non-existent folder
        explorer
            .handle_events(&format!(
                "r<C-u>{}<ret>",
                path.join("new_folder/sponge/bob").display()
            ))
            .unwrap();

        // 3a. Expect the non-existent folder to be created,
        //     and the file is moved into it,
        //     and the renamed file is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏷ [new_folder]
  ⏷ [sponge]
      (bob)
⏵ scripts
⏷ styles
  ⏵ public
    style.css
  index.html
"
            )
            .trim()
        );

        assert!(path.join("new_folder/sponge/bob").exists());

        // 4. Change current root to "new_folder/sponge"
        explorer.handle_events("k]").unwrap();

        // 4a. Expect the current root to be "sponge"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
({path_str}/new_folder/sponge)
  bob
"
            )
            .trim()
        );

        // 5. Move cursor to "bob", and rename it outside of the current root
        explorer.handle_events("j").unwrap();
        explorer
            .handle_events(&format!(
                "r<C-u>{}<ret>",
                path.join("scripts/bob").display()
            ))
            .unwrap();

        // 5a. Expect the current root to be "scripts"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}/scripts]
  (bob)
  main.js
"
            )
            .trim()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_new_folder() {
        let (path, mut explorer) = new_explorer();
        let path_str = path.display().to_string();

        // 0. Open the add file/folder prompt
        explorer.handle_events("a").unwrap();
        let prompt = &explorer.prompt.as_ref().unwrap().1;
        fn to_forward_slash(s: &str) -> String {
            s.replace(std::path::MAIN_SEPARATOR, "/")
        }
        fn to_os_main_separator(s: &str) -> String {
            s.replace('/', format!("{}", std::path::MAIN_SEPARATOR).as_str())
        }
        assert_eq!(
            to_forward_slash(prompt.prompt()),
            " New file or folder (ends with '/'): "
        );
        assert_eq!(to_forward_slash(prompt.line()), format!("{path_str}/"));

        // 1. Add a new folder at the root
        explorer
            .handle_events(&to_os_main_separator("yoyo/<ret>"))
            .unwrap();

        // 1a. Expect the new folder is added, and is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏵ styles
⏷ (yoyo)
  .gitignore
  index.html
"
            )
            .trim()
        );

        assert!(fs::read_dir(path.join("yoyo")).is_ok());

        // 2. Move up to "styles"
        explorer.handle_events("k").unwrap();

        // 3. Add a new folder
        explorer
            .handle_events(&to_os_main_separator("asus.sass/<ret>"))
            .unwrap();

        // 3a. Expect the new folder is added under "styles", although "styles" is not opened
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏷ [styles]
  ⏵ public
  ⏷ (sus.sass)
    style.css
⏷ yoyo
  .gitignore
  index.html
"
            )
            .trim()
        );

        assert!(fs::read_dir(path.join("styles/sus.sass")).is_ok());

        // 4. Add a new folder with non-existent parents
        explorer
            .handle_events(&to_os_main_separator("aa/b/c/<ret>"))
            .unwrap();

        // 4a. Expect the non-existent parents are created,
        //     and the new folder is created,
        //     and is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏷ [styles]
  ⏷ [sus.sass]
    ⏷ [a]
      ⏷ [b]
        ⏷ (c)
    style.css
⏷ yoyo
  .gitignore
  index.html
"
            )
            .trim()
        );

        assert!(fs::read_dir(path.join("styles/sus.sass/a/b/c")).is_ok());

        // 5. Move to "style.css"
        explorer.handle_events("j").unwrap();

        // 6. Add a new folder here
        explorer
            .handle_events(&to_os_main_separator("afoobar/<ret>"))
            .unwrap();

        // 6a. Expect the folder is added under "styles",
        //     because the folder of the current item, "style.css" is "styles/"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏷ [styles]
  ⏷ (foobar)
  ⏵ public
  ⏷ sus.sass
    ⏷ a
      ⏷ b
        ⏷ c
    style.css
"
            )
            .trim()
        );

        assert!(fs::read_dir(path.join("styles/foobar")).is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_new_file() {
        let (path, mut explorer) = new_explorer();
        let path_str = path.display().to_string();

        // 1. Add a new file at the root
        explorer.handle_events("ayoyo<ret>").unwrap();

        // 1a. Expect the new file is added, and is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏵ styles
  .gitignore
  index.html
  (yoyo)
"
            )
            .trim()
        );

        assert!(fs::read_to_string(path.join("yoyo")).is_ok());

        // 2. Move up to "styles"
        explorer.tree.move_up(3);

        // 3. Add a new file
        explorer.handle_events("asus.sass<ret>").unwrap();

        // 3a. Expect the new file is added under "styles", although "styles" is not opened
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏷ [styles]
  ⏵ public
    style.css
    (sus.sass)
  .gitignore
  index.html
  yoyo
"
            )
            .trim()
        );

        assert!(fs::read_to_string(path.join("styles/sus.sass")).is_ok());

        // 4. Add a new file with non-existent parents
        explorer.handle_events("aa/b/c<ret>").unwrap();

        // 4a. Expect the non-existent parents are created,
        //     and the new file is created,
        //     and is focused
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏷ [styles]
  ⏷ [a]
    ⏷ [b]
        (c)
  ⏵ public
    style.css
    sus.sass
  .gitignore
"
            )
            .trim()
        );

        assert!(fs::read_to_string(path.join("styles/a/b/c")).is_ok());

        // 5. Move to "style.css"
        explorer.handle_events("jj").unwrap();

        // 6. Add a new file here
        explorer.handle_events("afoobar<ret>").unwrap();

        // 6a. Expect the file is added under "styles",
        //     because the folder of the current item, "style.css" is "styles/"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏷ [styles]
    ⏷ b
        c
  ⏵ public
    (foobar)
    style.css
    sus.sass
  .gitignore
  index.html
"
            )
            .trim()
        );

        assert!(fs::read_to_string(path.join("styles/foobar")).is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_remove_file() {
        let (path, mut explorer) = new_explorer();
        let path_str = path.display().to_string();

        // 1. Move to ".gitignore"
        explorer.handle_events("/.gitignore<ret>").unwrap();

        // 1a. Expect the cursor is at ".gitignore"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏵ styles
  (.gitignore)
  index.html
"
            )
            .trim()
        );

        assert!(fs::read_to_string(path.join(".gitignore")).is_ok());

        // 2. Remove the current file
        explorer.handle_events("dy").unwrap();

        // 3. Expect ".gitignore" is deleted, and the cursor moved down
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏵ styles
  (index.html)
"
            )
            .trim()
        );

        assert!(fs::read_to_string(path.join(".gitignore")).is_err());

        // 3a. Expect "index.html" exists
        assert!(fs::read_to_string(path.join("index.html")).is_ok());

        // 4. Remove the current file
        explorer.handle_events("dy").unwrap();

        // 4a. Expect "index.html" is deleted, at the cursor moved up
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏵ (styles)
"
            )
            .trim()
        );

        assert!(fs::read_to_string(path.join("index.html")).is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_remove_folder() {
        let (path, mut explorer) = new_explorer();
        let path_str = path.display().to_string();

        // 1. Move to "styles/"
        explorer.handle_events("/styles<ret>o").unwrap();

        // 1a. Expect the cursor is at "styles"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏷ (styles)
  ⏵ public
    style.css
  .gitignore
  index.html
"
            )
            .trim()
        );

        assert!(fs::read_dir(path.join("styles")).is_ok());

        // 2. Remove the current folder
        explorer.handle_events("dy").unwrap();

        // 3. Expect "styles" is deleted, and the cursor moved down
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
  (.gitignore)
  index.html
"
            )
            .trim()
        );

        assert!(fs::read_dir(path.join("styles")).is_err());
    }

    #[test]
    fn test_change_root() {
        let (path, mut explorer) = new_explorer();
        let path_str = path.display().to_string();

        // 1. Move cursor to "styles"
        explorer.reveal_file(path.join("styles")).unwrap();

        // 2. Change root to current folder, and move cursor down
        explorer.change_root_to_current_folder().unwrap();
        explorer.tree.move_down(1);

        // 2a. Expect the current root to be "styles", and the cursor is at "public"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}/styles]
⏵ (public)
  style.css
"
            )
            .trim()
        );

        let current_root = explorer.state.current_root.clone();

        // 3. Change root to the parent of current folder
        explorer.change_root_parent_folder().unwrap();

        // 3a. Expect the current root to be "change_root"
        assert_eq!(
            render(&mut explorer),
            format!(
                "
({path_str})
⏵ scripts
⏵ styles
  .gitignore
  index.html
"
            )
            .trim()
        );

        // 4. Go back to previous root
        explorer.go_to_previous_root();

        // 4a. Expect the root te become "styles", and the cursor position is not forgotten
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}/styles]
⏵ (public)
  style.css
"
            )
            .trim()
        );
        assert_eq!(explorer.state.current_root, current_root);

        // 5. Go back to previous root again
        explorer.go_to_previous_root();

        // 5a. Expect the current root to be "change_root" again,
        //     but this time the "styles" folder is opened,
        //     because it was opened before any change of root
        assert_eq!(
            render(&mut explorer),
            format!(
                "
[{path_str}]
⏵ scripts
⏷ (styles)
  ⏵ public
    style.css
  .gitignore
  index.html
"
            )
            .trim()
        );
    }
}
