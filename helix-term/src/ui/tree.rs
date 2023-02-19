use std::cmp::Ordering;

use anyhow::Result;
use helix_view::theme::Modifier;

use crate::{
    compositor::{Context, EventResult},
    ctrl, key, shift,
};
use helix_core::movement::Direction;
use helix_view::{
    graphics::Rect,
    input::{Event, KeyEvent},
};
use tui::buffer::Buffer as Surface;

pub trait TreeViewItem: Sized {
    type Params;

    // fn text(&self, cx: &mut Context, selected: bool, params: &mut Self::Params) -> Spans;
    fn name(&self) -> String;
    fn is_parent(&self) -> bool;
    fn cmp(&self, other: &Self) -> Ordering;

    fn filter(&self, s: &str) -> bool {
        self.name().to_lowercase().contains(&s.to_lowercase())
    }

    fn get_children(&self) -> Result<Vec<Self>>;
}

fn tree_item_cmp<T: TreeViewItem>(item1: &T, item2: &T) -> Ordering {
    T::cmp(item1, item2)
}

pub fn vec_to_tree<T: TreeViewItem>(mut items: Vec<T>) -> Vec<Tree<T>> {
    items.sort_by(tree_item_cmp);
    index_elems(
        0,
        items
            .into_iter()
            .map(|item| Tree::new(item, vec![]))
            .collect(),
    )
}

pub enum TreeOp {
    Noop,
    GetChildsAndInsert,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Tree<T> {
    item: T,
    parent_index: Option<usize>,
    index: usize,
    children: Vec<Self>,

    /// Why do we need this property?
    /// Can't we just use `!children.is_empty()`?
    ///
    /// Because we might have for example an open folder that is empty,
    /// and user just added a new file under that folder,
    /// and the user refreshes the whole tree.
    ///
    /// Without `open`, we will not refresh any node without children,
    /// and thus the folder still appears empty after refreshing.
    is_opened: bool,
}

impl<T: Clone> Clone for Tree<T> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            index: self.index,
            children: self.children.clone(),
            is_opened: false,
            parent_index: self.parent_index,
        }
    }
}

#[derive(Clone)]
struct TreeIter<'a, T> {
    current_index_forward: usize,
    current_index_reverse: isize,
    tree: &'a Tree<T>,
}

impl<'a, T> Iterator for TreeIter<'a, T> {
    type Item = &'a Tree<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.current_index_forward;
        if index > self.tree.len().saturating_sub(1) {
            None
        } else {
            self.current_index_forward = self.current_index_forward.saturating_add(1);
            self.tree.get(index)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.tree.len(), Some(self.tree.len()))
    }
}

impl<'a, T> DoubleEndedIterator for TreeIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let index = self.current_index_reverse;
        if index < 0 {
            None
        } else {
            self.current_index_reverse = self.current_index_reverse.saturating_sub(1);
            self.tree.get(index as usize)
        }
    }
}

impl<'a, T> ExactSizeIterator for TreeIter<'a, T> {}

impl<T: TreeViewItem> Tree<T> {
    fn open(&mut self, filter: &String) -> Result<()> {
        if self.item.is_parent() {
            self.children = self.get_filtered_children(filter)?;
            self.is_opened = true;
        }
        Ok(())
    }

    fn close(&mut self) {
        self.is_opened = false;
        self.children = vec![];
    }

    fn refresh(&mut self, filter: &String) -> Result<()> {
        if !self.is_opened {
            return Ok(());
        }
        let latest_children = self.get_filtered_children(filter)?;
        let filtered = std::mem::replace(&mut self.children, vec![])
            .into_iter()
            // Remove children that does not exists in latest_children
            .filter(|tree| {
                latest_children
                    .iter()
                    .any(|child| tree.item.name().eq(&child.item.name()))
            })
            .map(|mut tree| {
                tree.refresh(filter)?;
                Ok(tree)
            })
            .collect::<Result<Vec<_>>>()?;

        // Add new children
        let new_nodes = latest_children
            .into_iter()
            .filter(|child| {
                !filtered
                    .iter()
                    .any(|child_| child.item.name().eq(&child_.item.name()))
            })
            .collect::<Vec<_>>();

        self.children = filtered.into_iter().chain(new_nodes).collect();

        self.sort();

        self.regenerate_index();

        Ok(())
    }

    fn get_filtered_children(&self, filter: &String) -> Result<Vec<Tree<T>>> {
        Ok(vec_to_tree(
            self.item
                .get_children()?
                .into_iter()
                .filter(|item| {
                    item.is_parent() || item.name().to_lowercase().contains(&filter.to_lowercase())
                })
                .collect(),
        ))
    }

    fn sort(&mut self) {
        self.children
            .sort_by(|a, b| tree_item_cmp(&a.item, &b.item))
    }
}

impl<T> Tree<T> {
    pub fn new(item: T, children: Vec<Tree<T>>) -> Self {
        let is_opened = !children.is_empty();
        Self {
            item,
            index: 0,
            parent_index: None,
            children: index_elems(0, children),
            is_opened,
        }
    }

    fn iter(&self) -> TreeIter<T> {
        TreeIter {
            tree: self,
            current_index_forward: 0,
            current_index_reverse: (self.len() - 1) as isize,
        }
    }

    /// Find an element in the tree with given `predicate`.
    /// `start_index` is inclusive if direction is `Forward`.
    /// `start_index` is exclusive if direction is `Backward`.
    pub fn find<F>(&self, start_index: usize, direction: Direction, predicate: F) -> Option<usize>
    where
        F: FnMut(&Tree<T>) -> bool,
    {
        let iter = self.iter();
        match direction {
            Direction::Forward => iter
                .skip(start_index)
                .position(predicate)
                .map(|index| index + start_index),
            Direction::Backward => iter.take(start_index).rposition(predicate),
        }
    }

    pub fn item(&self) -> &T {
        &self.item
    }

    fn get<'a>(&'a self, index: usize) -> Option<&'a Tree<T>> {
        if self.index == index {
            Some(self)
        } else {
            self.children.iter().find_map(|elem| elem.get(index))
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut Tree<T>> {
        if self.index == index {
            Some(self)
        } else {
            self.children
                .iter_mut()
                .find_map(|elem| elem.get_mut(index))
        }
    }

    fn len(&self) -> usize {
        (1 as usize).saturating_add(self.children.iter().map(|elem| elem.len()).sum())
    }

    fn regenerate_index(&mut self) {
        let items = std::mem::take(&mut self.children);
        self.children = index_elems(0, items);
    }

    fn remove(&mut self, index: usize) {
        let children = std::mem::replace(&mut self.children, vec![]);
        self.children = children
            .into_iter()
            .filter_map(|tree| {
                if tree.index == index {
                    None
                } else {
                    Some(tree)
                }
            })
            .map(|mut tree| {
                tree.remove(index);
                tree
            })
            .collect();
        self.regenerate_index()
    }

    pub fn parent_index(&self) -> Option<usize> {
        self.parent_index
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

pub struct TreeView<T: TreeViewItem> {
    tree: Tree<T>,

    /// Selected item idex
    selected: usize,

    /// (selected, row)
    save_view: (usize, usize),

    /// For implementing vertical scroll
    winline: usize,

    previous_area: Rect,

    /// For implementing horizontal scoll
    column: usize,

    /// For implementing horizontal scoll
    max_len: usize,
    count: usize,
    tree_symbol_style: String,
    #[allow(clippy::type_complexity)]
    on_opened_fn: Option<Box<dyn FnMut(&mut T, &mut Context, &mut T::Params) -> TreeOp + 'static>>,
    #[allow(clippy::type_complexity)]
    on_folded_fn: Option<Box<dyn FnMut(&mut T, &mut Context, &mut T::Params) + 'static>>,
    #[allow(clippy::type_complexity)]
    on_next_key: Option<Box<dyn FnMut(&mut Context, &mut Self, &KeyEvent)>>,
}

impl<T: TreeViewItem> TreeView<T> {
    pub fn new(root: T, items: Vec<Tree<T>>) -> Self {
        Self {
            tree: Tree::new(root, items),
            selected: 0,
            save_view: (0, 0),
            winline: 0,
            column: 0,
            max_len: 0,
            count: 0,
            previous_area: Rect::new(0, 0, 0, 0),
            tree_symbol_style: "ui.text".into(),
            on_opened_fn: None,
            on_folded_fn: None,
            on_next_key: None,
        }
    }

    pub fn build_tree(root: T, items: Vec<T>) -> Self {
        Self::new(root, vec_to_tree(items))
    }

    pub fn with_enter_fn<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut T, &mut Context, &mut T::Params) -> TreeOp + 'static,
    {
        self.on_opened_fn = Some(Box::new(f));
        self
    }

    pub fn with_folded_fn<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut T, &mut Context, &mut T::Params) + 'static,
    {
        self.on_folded_fn = Some(Box::new(f));
        self
    }

    pub fn tree_symbol_style(mut self, style: String) -> Self {
        self.tree_symbol_style = style;
        self
    }

    /// Reveal item in the tree based on the given `segments`.
    ///
    /// The name of the root should be excluded.
    ///
    /// Example `segments`:
    /// ```
    /// vec!["helix-term", "src", "ui", "tree.rs"]
    /// ```
    pub fn reveal_item(&mut self, segments: Vec<&str>, filter: &String) -> Result<()> {
        self.tree.refresh(filter)?;

        // Expand the tree
        segments.iter().fold(
            Ok(&mut self.tree),
            |current_tree, segment| match current_tree {
                Err(err) => Err(err),
                Ok(current_tree) => {
                    match current_tree
                        .children
                        .iter_mut()
                        .find(|tree| tree.item.name().eq(segment))
                    {
                        Some(tree) => {
                            if !tree.is_opened {
                                tree.open(filter)?;
                            }
                            Ok(tree)
                        }
                        None => Err(anyhow::anyhow!(format!(
                            "Unable to find path: '{}'. current_segment = {}",
                            segments.join("/"),
                            segment
                        ))),
                    }
                }
            },
        )?;

        // Locate the item
        self.regenerate_index();
        self.set_selected(
            segments
                .iter()
                .fold(&self.tree, |tree, segment| {
                    tree.children
                        .iter()
                        .find(|tree| tree.item.name().eq(segment))
                        .expect("Should be unreachable")
                })
                .index,
        );

        self.align_view_center();
        Ok(())
    }

    fn align_view_center(&mut self) {
        self.winline = self.previous_area.height as usize / 2
    }

    fn align_view_top(&mut self) {
        self.winline = 0
    }

    fn align_view_bottom(&mut self) {
        self.winline = self.previous_area.height as usize
    }

    fn regenerate_index(&mut self) {
        self.tree.regenerate_index();
    }

    fn move_to_parent(&mut self) {
        if let Some(parent) = self.current_parent() {
            let index = parent.index;
            self.set_selected(index)
        }
    }

    fn move_to_children(&mut self, filter: &String) -> Result<()> {
        let current = self.current_mut();
        if current.is_opened {
            self.set_selected(self.selected + 1);
            Ok(())
        } else {
            current.open(filter)?;
            if !current.children.is_empty() {
                self.set_selected(self.selected + 1);
                self.regenerate_index();
            }
            Ok(())
        }
    }

    pub fn refresh(&mut self, filter: &String) -> Result<()> {
        self.tree.refresh(filter)
    }

    fn move_to_first(&mut self) {
        self.move_up(usize::MAX / 2)
    }

    fn move_to_last(&mut self) {
        self.move_down(usize::MAX / 2)
    }

    fn set_previous_area(&mut self, area: Rect) {
        self.previous_area = area
    }

    fn move_leftmost(&mut self) {
        self.move_left(usize::MAX / 2);
    }

    fn move_rightmost(&mut self) {
        self.move_right(usize::MAX / 2)
    }
}

pub fn tree_view_help() -> Vec<(&'static str, &'static str)> {
    vec![
        ("j, down", "Down"),
        ("k, up", "Up"),
        ("h, left", "Go to parent"),
        ("l, right", "Expand"),
        ("L", "Scroll right"),
        ("H", "Scroll left"),
        ("zz", "Align view center"),
        ("zt", "Align view top"),
        ("zb", "Align view bottom"),
        ("gg", "Go to first line"),
        ("ge", "Go to last line"),
        ("gh", "Go to line start"),
        ("gl", "Go to line end"),
        ("C-d", "Page down"),
        ("C-u", "Page up"),
    ]
}

impl<T: TreeViewItem> TreeView<T> {
    pub fn on_enter(
        &mut self,
        cx: &mut Context,
        params: &mut T::Params,
        selected_index: usize,
        filter: &String,
    ) {
        let selected_item = self.get_mut(selected_index);
        if selected_item.is_opened {
            selected_item.close();
            self.regenerate_index();
            return;
        }

        if let Some(mut on_open_fn) = self.on_opened_fn.take() {
            let mut f = || {
                let current = self.current_mut();
                match on_open_fn(&mut current.item, cx, params) {
                    TreeOp::GetChildsAndInsert => {
                        if let Err(err) = current.open(filter) {
                            cx.editor.set_error(format!("{err}"))
                        }
                    }
                    TreeOp::Noop => {}
                };
            };
            f();
            self.regenerate_index();
            self.on_opened_fn = Some(on_open_fn)
        }
    }

    pub fn search_next(&mut self, s: &str) {
        let skip = std::cmp::max(2, self.save_view.0 + 1);
        self.set_selected(
            self.tree
                .find(skip, Direction::Forward, |e| e.item.filter(s))
                .unwrap_or(self.save_view.0),
        );

        self.winline = (self.save_view.1 + self.selected).saturating_sub(self.save_view.0);
    }

    pub fn search_previous(&mut self, s: &str) {
        let take = self.save_view.0;
        self.set_selected(
            self.tree
                .find(take, Direction::Backward, |e| e.item.filter(s))
                .unwrap_or(self.save_view.0),
        );

        self.winline = (self.save_view.1 + self.selected).saturating_sub(self.save_view.0);
    }

    pub fn move_down(&mut self, rows: usize) {
        let len = self.tree.len();
        if len > 0 {
            self.set_selected(std::cmp::min(self.selected + rows, len.saturating_sub(1)))
        }
    }

    fn set_selected(&mut self, selected: usize) {
        if selected > self.selected {
            // Move down
            self.winline = selected.min(
                self.winline
                    .saturating_add(selected.saturating_sub(self.selected)),
            );
        } else {
            // Move up
            self.winline = selected.min(
                self.winline
                    .saturating_sub(self.selected.saturating_sub(selected)),
            );
        }
        self.selected = selected;
    }

    pub fn move_up(&mut self, rows: usize) {
        let len = self.tree.len();
        if len > 0 {
            self.set_selected(self.selected.saturating_sub(rows).max(0))
        }
    }

    pub fn move_left(&mut self, cols: usize) {
        self.column = self.column.saturating_sub(cols);
    }

    pub fn move_right(&mut self, cols: usize) {
        let max_scroll = self
            .max_len
            .saturating_sub(self.previous_area.width as usize)
            .saturating_add(1);
        self.column = max_scroll.min(self.column + cols);
    }

    pub fn move_down_half_page(&mut self) {
        self.move_down(self.previous_area.height as usize / 2)
    }

    pub fn move_up_half_page(&mut self) {
        self.move_up(self.previous_area.height as usize / 2);
    }

    pub fn move_down_page(&mut self) {
        self.move_down(self.previous_area.height as usize);
    }

    pub fn move_up_page(&mut self) {
        self.move_up(self.previous_area.height as usize);
    }

    pub fn save_view(&mut self) {
        self.save_view = (self.selected, self.winline);
    }

    pub fn restore_view(&mut self) {
        (self.selected, self.winline) = self.save_view;
    }

    fn get(&self, index: usize) -> &Tree<T> {
        self.tree.get(index).unwrap()
    }

    fn get_mut(&mut self, index: usize) -> &mut Tree<T> {
        self.tree.get_mut(index).unwrap()
    }

    pub fn current(&self) -> &Tree<T> {
        self.get(self.selected)
    }

    pub fn current_mut(&mut self) -> &mut Tree<T> {
        self.get_mut(self.selected)
    }

    fn current_parent(&self) -> Option<&Tree<T>> {
        if let Some(parent_index) = self.current().parent_index {
            Some(self.get(parent_index))
        } else {
            None
        }
    }

    pub fn current_item(&self) -> &T {
        &self.current().item
    }

    pub fn row(&self) -> usize {
        self.winline
    }

    pub fn remove_current(&mut self) {
        self.tree.remove(self.selected);
        self.set_selected(self.selected.min(self.tree.len().saturating_sub(1)));
    }

    pub fn replace_current(&mut self, item: T) {
        self.current_mut().item = item
    }

    pub fn add_child(&mut self, index: usize, item: T, filter: &String) -> Result<()> {
        match self.tree.get_mut(index) {
            None => Err(anyhow::anyhow!(format!(
                "No item found at index = {}",
                index
            ))),
            Some(tree) => {
                let item_name = item.name();
                if !tree.is_opened {
                    tree.open(filter)?;
                } else {
                    tree.refresh(filter)?;
                }

                self.regenerate_index();

                let tree = self.get(index);

                // Focus the added sibling
                if let Some(tree) = tree
                    .children
                    .iter()
                    .find(|tree| tree.item.name().eq(&item_name))
                {
                    let index = tree.index;
                    self.set_selected(index)
                };
                Ok(())
            }
        }
    }
}

struct RenderedLine {
    indent: String,
    name: String,
    selected: bool,
    descendant_selected: bool,
}
struct RenderTreeParams<'a, T> {
    tree: &'a Tree<T>,
    prefix: &'a String,
    level: usize,
    selected: usize,
    filter: &'a str,
}

fn render_tree<T: TreeViewItem>(
    RenderTreeParams {
        tree,
        prefix,
        level,
        selected,
        filter,
    }: RenderTreeParams<T>,
) -> Vec<RenderedLine> {
    let indent = if level > 0 {
        let indicator = if tree.item().is_parent() {
            if tree.is_opened {
                ""
            } else {
                ""
            }
        } else {
            " "
        };
        format!("{}{} ", prefix, indicator)
    } else {
        "".to_string()
    };
    let name = tree.item.name();
    let head = RenderedLine {
        indent,
        selected: selected == tree.index,
        descendant_selected: selected != tree.index && tree.get(selected).is_some(),
        name,
    };
    let prefix = format!("{}{}", prefix, if level == 0 { "" } else { "  " });
    vec![head]
        .into_iter()
        .chain(tree.children.iter().flat_map(|elem| {
            render_tree(RenderTreeParams {
                tree: elem,
                prefix: &prefix,
                level: level + 1,
                selected,
                filter,
            })
        }))
        .collect()
}

impl<T: TreeViewItem + Clone> TreeView<T> {
    pub fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context, filter: &String) {
        let style = cx.editor.theme.get(&self.tree_symbol_style);
        let ancestor_style = cx.editor.theme.get("ui.text.focus");

        let iter = self.render_lines(area, filter).into_iter().enumerate();

        for (index, line) in iter {
            let area = Rect::new(area.x, area.y + index as u16, area.width, 1);
            let indent_len = line.indent.chars().count() as u16;
            surface.set_stringn(
                area.x,
                area.y,
                line.indent.clone(),
                indent_len as usize,
                style,
            );

            let style = if line.selected {
                style.add_modifier(Modifier::REVERSED)
            } else {
                style
            };
            let x = area.x.saturating_add(indent_len);
            surface.set_stringn(
                x,
                area.y,
                line.name.clone(),
                area.width
                    .saturating_sub(indent_len)
                    .saturating_sub(1)
                    .into(),
                if line.descendant_selected {
                    ancestor_style
                } else {
                    style
                },
            );
        }
    }

    fn render_to_string(&mut self, filter: &String) -> String {
        let area = self.previous_area;
        let lines = self.render_lines(area, filter);
        lines
            .into_iter()
            .map(|line| {
                let name = if line.selected {
                    format!("({})", line.name)
                } else if line.descendant_selected {
                    format!("[{}]", line.name)
                } else {
                    line.name
                };
                format!("{}{}", line.indent, name)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_lines(&mut self, area: Rect, filter: &String) -> Vec<RenderedLine> {
        self.previous_area = area;
        self.winline = self
            .winline
            .min(self.previous_area.height.saturating_sub(1) as usize);
        let skip = self.selected.saturating_sub(self.winline);
        let params = RenderTreeParams {
            tree: &self.tree,
            prefix: &"".to_string(),
            level: 0,
            selected: self.selected,
            filter,
        };

        let lines = render_tree(params);

        self.max_len = lines
            .iter()
            .map(|line| {
                line.indent
                    .chars()
                    .count()
                    .saturating_add(line.name.chars().count())
            })
            .max()
            .unwrap_or(0);

        let max_width = self.previous_area.width as usize;

        lines
            .into_iter()
            // Horizontal scroll
            .map(|line| {
                let skip = self.column;
                let indent_len = line.indent.chars().count();
                RenderedLine {
                    indent: if line.indent.is_empty() {
                        "".to_string()
                    } else {
                        line.indent
                            .chars()
                            .skip(skip)
                            .take(max_width)
                            .collect::<String>()
                    },
                    name: line
                        .name
                        .chars()
                        .skip(skip.saturating_sub(indent_len))
                        .take((max_width.saturating_sub(indent_len)).clamp(0, line.name.len()))
                        .collect::<String>(),
                    ..line
                }
            })
            // Vertical scroll
            .skip(skip)
            .take(area.height as usize)
            .collect()
    }

    pub fn handle_event(
        &mut self,
        event: &Event,
        cx: &mut Context,
        params: &mut T::Params,
        filter: &String,
    ) -> EventResult {
        let key_event = match event {
            Event::Key(event) => event,
            Event::Resize(..) => return EventResult::Consumed(None),
            _ => return EventResult::Ignored(None),
        };
        if let Some(mut on_next_key) = self.on_next_key.take() {
            on_next_key(cx, self, key_event);
            return EventResult::Consumed(None);
        }
        let count = std::mem::replace(&mut self.count, 0);
        match key_event {
            key!(i @ '0'..='9') => self.count = i.to_digit(10).unwrap() as usize + count * 10,
            key!('k') | key!(Up) => self.move_up(1.max(count)),
            key!('j') | key!(Down) => self.move_down(1.max(count)),
            key!('z') => {
                self.on_next_key = Some(Box::new(|_, tree, event| match event {
                    key!('z') => tree.align_view_center(),
                    key!('t') => tree.align_view_top(),
                    key!('b') => tree.align_view_bottom(),
                    _ => {}
                }));
            }
            key!('h') | key!(Left) => self.move_to_parent(),
            key!('l') | key!(Right) => match self.move_to_children(filter) {
                Ok(_) => {}
                Err(err) => cx.editor.set_error(err.to_string()),
            },
            shift!('H') => self.move_left(1),
            shift!('L') => self.move_right(1),
            key!(Enter) => self.on_enter(cx, params, self.selected, filter),
            ctrl!('d') => self.move_down_half_page(),
            ctrl!('u') => self.move_up_half_page(),
            key!('g') => {
                self.on_next_key = Some(Box::new(|_, tree, event| match event {
                    key!('g') => tree.move_to_first(),
                    key!('e') => tree.move_to_last(),
                    key!('h') => tree.move_leftmost(),
                    key!('l') => tree.move_rightmost(),
                    _ => {}
                }));
            }
            _ => return EventResult::Ignored(None),
        }

        EventResult::Consumed(None)
    }
}

/// Recalculate the index of each item of a tree.
///
/// For example:
///
/// ```
/// foo (0)
///   bar (1)
/// spam (2)
///   jar (3)
///     yo (4)
/// ```
fn index_elems<T>(parent_index: usize, elems: Vec<Tree<T>>) -> Vec<Tree<T>> {
    fn index_elems<'a, T>(
        current_index: usize,
        elems: Vec<Tree<T>>,
        parent_index: usize,
    ) -> (usize, Vec<Tree<T>>) {
        elems
            .into_iter()
            .fold((current_index, vec![]), |(current_index, trees), elem| {
                let index = current_index;
                let item = elem.item;
                let (current_index, folded) = index_elems(current_index + 1, elem.children, index);
                let tree = Tree {
                    item,
                    children: folded,
                    index,
                    is_opened: elem.is_opened,
                    parent_index: Some(parent_index),
                };
                (
                    current_index,
                    trees.into_iter().chain(vec![tree].into_iter()).collect(),
                )
            })
    }
    index_elems(parent_index + 1, elems, parent_index).1
}

#[cfg(test)]
mod test_tree_view {
    use helix_view::graphics::Rect;

    use super::{vec_to_tree, TreeView, TreeViewItem};
    use pretty_assertions::assert_eq;

    #[derive(Clone)]
    struct Item<'a> {
        name: &'a str,
    }

    fn item<'a>(name: &'a str) -> Item<'a> {
        Item { name }
    }

    impl<'a> TreeViewItem for Item<'a> {
        type Params = ();

        fn name(&self) -> String {
            self.name.to_string()
        }

        fn is_parent(&self) -> bool {
            self.name.len() > 2
        }

        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.name.cmp(other.name)
        }

        fn get_children(&self) -> anyhow::Result<Vec<Self>> {
            if self.is_parent() {
                let (left, right) = self.name.split_at(self.name.len() / 2);
                Ok(vec![item(left), item(right)])
            } else {
                Ok(vec![])
            }
        }
    }

    fn dummy_tree_view<'a>() -> TreeView<Item<'a>> {
        let root = item("who_lives_in_a_pineapple_under_the_sea");
        let mut view = TreeView::new(
            root,
            vec_to_tree(vec![
                item("gary_the_snail"),
                item("krabby_patty"),
                item("larry_the_lobster"),
                item("patrick_star"),
                item("sandy_cheeks"),
                item("spongebob_squarepants"),
                item("mrs_puff"),
                item("king_neptune"),
                item("karen"),
                item("plankton"),
            ]),
        );

        view.set_previous_area(dummy_area());

        view
    }

    fn dummy_area() -> Rect {
        Rect::new(0, 0, 50, 5)
    }

    fn render<'a>(view: &mut TreeView<Item<'a>>) -> String {
        view.render_to_string(&"".to_string())
    }

    #[test]
    fn test_init() {
        let mut view = dummy_tree_view();

        // Expect the items to be sorted
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pineapple_under_the_sea)
 gary_the_snail
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );
    }

    #[test]
    fn test_move_up_down() {
        let mut view = dummy_tree_view();
        view.move_down(1);
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 (gary_the_snail)
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );

        view.move_down(3);
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 gary_the_snail
 karen
 king_neptune
 (krabby_patty)
"
            .trim()
        );

        view.move_down(1);
        assert_eq!(
            render(&mut view),
            "
 gary_the_snail
 karen
 king_neptune
 krabby_patty
 (larry_the_lobster)
"
            .trim()
        );

        view.move_up(1);
        assert_eq!(
            render(&mut view),
            "
 gary_the_snail
 karen
 king_neptune
 (krabby_patty)
 larry_the_lobster
"
            .trim()
        );

        view.move_up(3);
        assert_eq!(
            render(&mut view),
            "
 (gary_the_snail)
 karen
 king_neptune
 krabby_patty
 larry_the_lobster
"
            .trim()
        );

        view.move_up(1);
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pineapple_under_the_sea)
 gary_the_snail
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );
    }

    #[test]
    fn test_align_view() {
        let mut view = dummy_tree_view();
        view.move_down(5);
        assert_eq!(
            render(&mut view),
            "
 gary_the_snail
 karen
 king_neptune
 krabby_patty
 (larry_the_lobster)
"
            .trim()
        );

        view.align_view_center();
        assert_eq!(
            render(&mut view),
            "
 king_neptune
 krabby_patty
 (larry_the_lobster)
 mrs_puff
 patrick_star
"
            .trim()
        );

        view.align_view_bottom();
        assert_eq!(
            render(&mut view),
            "
 gary_the_snail
 karen
 king_neptune
 krabby_patty
 (larry_the_lobster)
"
            .trim()
        );
    }

    #[test]
    fn test_move_to_first_last() {
        let mut view = dummy_tree_view();

        view.move_to_last();
        assert_eq!(
            render(&mut view),
            "
 mrs_puff
 patrick_star
 plankton
 sandy_cheeks
 (spongebob_squarepants)
"
            .trim()
        );

        view.move_to_first();
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pineapple_under_the_sea)
 gary_the_snail
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );
    }

    #[test]
    fn test_move_half() {
        let mut view = dummy_tree_view();
        view.move_down_half_page();
        assert_eq!(view.selected, 2);
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 gary_the_snail
 (karen)
 king_neptune
 krabby_patty
"
            .trim()
        );

        view.move_down_half_page();
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 gary_the_snail
 karen
 king_neptune
 (krabby_patty)
"
            .trim()
        );

        view.move_down_half_page();
        assert_eq!(
            render(&mut view),
            "
 karen
 king_neptune
 krabby_patty
 larry_the_lobster
 (mrs_puff)
"
            .trim()
        );

        view.move_up_half_page();
        assert_eq!(
            render(&mut view),
            "
 karen
 king_neptune
 (krabby_patty)
 larry_the_lobster
 mrs_puff
"
            .trim()
        );

        view.move_up_half_page();
        assert_eq!(
            render(&mut view),
            "
 (karen)
 king_neptune
 krabby_patty
 larry_the_lobster
 mrs_puff
"
            .trim()
        );

        view.move_up_half_page();
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pineapple_under_the_sea)
 gary_the_snail
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );
    }

    #[test]
    fn move_to_children_parent() {
        let filter = "".to_string();
        let mut view = dummy_tree_view();
        view.move_down(1);
        view.move_to_children(&filter).unwrap();
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 [gary_the_snail]
   (e_snail)
   gary_th
 karen
 "
            .trim()
        );

        view.move_down(1);
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 [gary_the_snail]
   e_snail
   (gary_th)
 karen
 "
            .trim()
        );

        view.move_to_parent();
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 (gary_the_snail)
   e_snail
   gary_th
 karen
 "
            .trim()
        );

        view.move_to_last();
        view.move_to_parent();
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pineapple_under_the_sea)
 gary_the_snail
   e_snail
   gary_th
 karen
 "
            .trim()
        );
    }

    #[test]
    fn test_move_left_right() {
        let mut view = dummy_tree_view();
        view.set_previous_area(dummy_area().with_width(20));

        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pinea)
 gary_the_snail
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );

        view.move_right(1);
        assert_eq!(
            render(&mut view),
            "
(ho_lives_in_a_pineap)
 gary_the_snail
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );

        view.move_right(1);
        assert_eq!(
            render(&mut view),
            "
(o_lives_in_a_pineapp)
gary_the_snail
karen
king_neptune
krabby_patty
"
            .trim()
        );

        view.move_right(1);
        assert_eq!(
            render(&mut view),
            "
(_lives_in_a_pineappl)
ary_the_snail
aren
ing_neptune
rabby_patty
"
            .trim()
        );

        view.move_left(1);
        assert_eq!(
            render(&mut view),
            "
(o_lives_in_a_pineapp)
gary_the_snail
karen
king_neptune
krabby_patty
"
            .trim()
        );

        view.move_leftmost();
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pinea)
 gary_the_snail
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );

        view.move_left(1);
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pinea)
 gary_the_snail
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );

        view.move_rightmost();
        assert_eq!(render(&mut view), "(apple_under_the_sea)\n\n\n\n");
    }

    #[test]
    fn test_move_to_parent_child() {
        let mut view = dummy_tree_view();
        let filter = "".to_string();

        view.move_to_children(&filter).unwrap();
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 (gary_the_snail)
 karen
 king_neptune
 krabby_patty
"
            .trim()
        );

        view.move_to_children(&filter).unwrap();
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 [gary_the_snail]
   (e_snail)
   gary_th
 karen
"
            .trim()
        );

        view.move_down(1);
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 [gary_the_snail]
   e_snail
   (gary_th)
 karen
"
            .trim()
        );

        view.move_to_parent();
        assert_eq!(
            render(&mut view),
            "
[who_lives_in_a_pineapple_under_the_sea]
 (gary_the_snail)
   e_snail
   gary_th
 karen
"
            .trim()
        );

        view.move_to_parent();
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pineapple_under_the_sea)
 gary_the_snail
   e_snail
   gary_th
 karen
"
            .trim()
        );

        view.move_to_parent();
        assert_eq!(
            render(&mut view),
            "
(who_lives_in_a_pineapple_under_the_sea)
 gary_the_snail
   e_snail
   gary_th
 karen
"
            .trim()
        )
    }
}

#[cfg(test)]
mod test_tree {
    use helix_core::movement::Direction;

    use super::Tree;

    #[test]
    fn test_get() {
        let result = Tree::new(
            "root",
            vec![
                Tree::new("foo", vec![Tree::new("bar", vec![])]),
                Tree::new(
                    "spam",
                    vec![Tree::new("jar", vec![Tree::new("yo", vec![])])],
                ),
            ],
        );
        assert_eq!(result.get(0).unwrap().item, "root");
        assert_eq!(result.get(1).unwrap().item, "foo");
        assert_eq!(result.get(2).unwrap().item, "bar");
        assert_eq!(result.get(3).unwrap().item, "spam");
        assert_eq!(result.get(4).unwrap().item, "jar");
        assert_eq!(result.get(5).unwrap().item, "yo");
    }

    #[test]
    fn test_iter() {
        let tree = Tree::new(
            "spam",
            vec![
                Tree::new("jar", vec![Tree::new("yo", vec![])]),
                Tree::new("foo", vec![Tree::new("bar", vec![])]),
            ],
        );

        let mut iter = tree.iter();
        assert_eq!(iter.next().map(|tree| tree.item), Some("spam"));
        assert_eq!(iter.next().map(|tree| tree.item), Some("jar"));
        assert_eq!(iter.next().map(|tree| tree.item), Some("yo"));
        assert_eq!(iter.next().map(|tree| tree.item), Some("foo"));
        assert_eq!(iter.next().map(|tree| tree.item), Some("bar"));
    }

    #[test]
    fn test_iter_double_ended() {
        let tree = Tree::new(
            "spam",
            vec![
                Tree::new("jar", vec![Tree::new("yo", vec![])]),
                Tree::new("foo", vec![Tree::new("bar", vec![])]),
            ],
        );

        let mut iter = tree.iter();
        assert_eq!(iter.next_back().map(|tree| tree.item), Some("bar"));
        assert_eq!(iter.next_back().map(|tree| tree.item), Some("foo"));
        assert_eq!(iter.next_back().map(|tree| tree.item), Some("yo"));
        assert_eq!(iter.next_back().map(|tree| tree.item), Some("jar"));
        assert_eq!(iter.next_back().map(|tree| tree.item), Some("spam"));
    }

    #[test]
    fn test_len() {
        let tree = Tree::new(
            "spam",
            vec![
                Tree::new("jar", vec![Tree::new("yo", vec![])]),
                Tree::new("foo", vec![Tree::new("bar", vec![])]),
            ],
        );

        assert_eq!(tree.len(), 5)
    }

    #[test]
    fn test_find_forward() {
        let tree = Tree::new(
            ".cargo",
            vec![
                Tree::new("jar", vec![Tree::new("Cargo.toml", vec![])]),
                Tree::new("Cargo.toml", vec![Tree::new("bar", vec![])]),
            ],
        );
        let result = tree.find(0, Direction::Forward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, Some(0));

        let result = tree.find(1, Direction::Forward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, Some(2));

        let result = tree.find(2, Direction::Forward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, Some(2));

        let result = tree.find(3, Direction::Forward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, Some(3));

        let result = tree.find(4, Direction::Forward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, None);
    }

    #[test]
    fn test_find_backward() {
        let tree = Tree::new(
            ".cargo",
            vec![
                Tree::new("jar", vec![Tree::new("Cargo.toml", vec![])]),
                Tree::new("Cargo.toml", vec![Tree::new("bar", vec![])]),
            ],
        );
        let result = tree.find(0, Direction::Backward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, None);

        let result = tree.find(1, Direction::Backward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, Some(0));

        let result = tree.find(2, Direction::Backward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, Some(0));

        let result = tree.find(3, Direction::Backward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, Some(2));

        let result = tree.find(4, Direction::Backward, |tree| {
            tree.item.to_lowercase().contains(&"cargo".to_lowercase())
        });

        assert_eq!(result, Some(3));
    }

    #[test]
    fn test_remove() {
        let mut tree = Tree::new(
            ".cargo",
            vec![
                Tree::new("spam", vec![Tree::new("Cargo.toml", vec![])]),
                Tree::new("Cargo.toml", vec![Tree::new("pam", vec![])]),
                Tree::new("hello", vec![]),
            ],
        );

        tree.remove(2);

        assert_eq!(
            tree,
            Tree::new(
                ".cargo",
                vec![
                    Tree::new("spam", vec![]),
                    Tree::new("Cargo.toml", vec![Tree::new("pam", vec![])]),
                    Tree::new("hello", vec![]),
                ],
            )
        );

        tree.remove(2);

        assert_eq!(
            tree,
            Tree::new(
                ".cargo",
                vec![Tree::new("spam", vec![]), Tree::new("hello", vec![]),],
            )
        )
    }
}
