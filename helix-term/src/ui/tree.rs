use std::{cmp::Ordering, path::PathBuf};

use anyhow::Result;
use helix_view::theme::Modifier;

use crate::{
    compositor::{Context, EventResult},
    ctrl, key, shift,
};
use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::{
    graphics::Rect,
    input::{Event, KeyEvent},
};
use tui::{buffer::Buffer as Surface, text::Spans};

pub trait TreeItem: Sized {
    type Params;

    // fn text(&self, cx: &mut Context, selected: bool, params: &mut Self::Params) -> Spans;
    fn text_string(&self) -> String;
    fn is_child(&self, other: &Self) -> bool;
    fn is_parent(&self) -> bool;
    fn cmp(&self, other: &Self) -> Ordering;

    fn filter(&self, s: &str) -> bool {
        self.text_string()
            .to_lowercase()
            .contains(&s.to_lowercase())
    }

    fn get_children(&self) -> Result<Vec<Self>> {
        Ok(vec![])
    }
}

fn tree_item_cmp<T: TreeItem>(item1: &T, item2: &T) -> Ordering {
    if item1.is_child(item2) {
        return Ordering::Greater;
    }
    if item2.is_child(item1) {
        return Ordering::Less;
    }

    T::cmp(item1, item2)
}

fn vec_to_tree<T: TreeItem>(mut items: Vec<T>) -> Vec<Tree<T>> {
    items.sort_by(tree_item_cmp);
    index_elems(
        0,
        items
            .into_iter()
            .map(|item| Tree::new(item, vec![]))
            .collect(),
    )
}

// return total elems's count contain self
fn get_elems_recursion<T: TreeItem>(t: &mut Tree<T>, depth: usize) -> Result<usize> {
    let mut childs = t.item.get_children()?;
    childs.sort_by(tree_item_cmp);
    let mut elems = Vec::with_capacity(childs.len());
    // let level = t.level + 1;
    let level = todo!();

    let mut total = 1;
    for child in childs {
        let mut elem = Tree::new(child, level);
        let count = if depth > 0 {
            get_elems_recursion(&mut elem, depth - 1)?
        } else {
            1
        };
        elems.push(elem);
        total += count;
    }
    t.children = elems;
    Ok(total)
}

fn expand_elems<T: TreeItem>(dist: &mut Vec<Tree<T>>, mut t: Tree<T>) {
    let childs = std::mem::take(&mut t.children);
    dist.push(t);
    for child in childs {
        expand_elems(dist, child)
    }
}

pub enum TreeOp<T> {
    Noop,
    Restore,
    InsertChild(Vec<T>),
    GetChildsAndInsert,
    ReplaceTree(Vec<T>),
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
            self.tree.find_by_index(index)
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
            self.tree.find_by_index(index as usize)
        }
    }
}

impl<'a, T> ExactSizeIterator for TreeIter<'a, T> {}

impl<T> Tree<T> {
    fn iter(&self) -> TreeIter<T> {
        TreeIter {
            tree: self,
            current_index_forward: 0,
            current_index_reverse: (self.len() - 1) as isize,
        }
    }
    pub fn new(item: T, children: Vec<Tree<T>>) -> Self {
        Self {
            item,
            index: 0,
            parent_index: None,
            children: index_elems(1, children),
            is_opened: false,
        }
    }

    pub fn item(&self) -> &T {
        &self.item
    }

    fn find_by_index(&self, index: usize) -> Option<&Tree<T>> {
        if self.index == index {
            Some(self)
        } else {
            self.children
                .iter()
                .find_map(|elem| elem.find_by_index(index))
        }
    }

    fn find_by_index_mut(&mut self, index: usize) -> Option<&mut Tree<T>> {
        if self.index == index {
            Some(self)
        } else {
            self.children
                .iter_mut()
                .find_map(|elem| elem.find_by_index_mut(index))
        }
    }

    fn len(&self) -> usize {
        (1 as usize).saturating_add(self.children.iter().map(|elem| elem.len()).sum())
    }

    fn regenerate_index(&mut self) {
        let items = std::mem::take(&mut self.children);
        self.children = index_elems(1, items);
    }
}

pub struct TreeView<T: TreeItem> {
    tree: Tree<T>,
    recycle: Option<(String, Vec<Tree<T>>)>,
    /// Selected item idex
    selected: usize,

    /// (selected, row)
    save_view: (usize, usize),

    /// View row
    winline: usize,

    area_height: usize,
    col: usize,
    max_len: usize,
    count: usize,
    tree_symbol_style: String,
    #[allow(clippy::type_complexity)]
    pre_render: Option<Box<dyn Fn(&mut Self, Rect) + 'static>>,
    #[allow(clippy::type_complexity)]
    on_opened_fn:
        Option<Box<dyn FnMut(&mut T, &mut Context, &mut T::Params) -> TreeOp<T> + 'static>>,
    #[allow(clippy::type_complexity)]
    on_folded_fn: Option<Box<dyn FnMut(&mut T, &mut Context, &mut T::Params) + 'static>>,
    #[allow(clippy::type_complexity)]
    on_next_key: Option<Box<dyn FnMut(&mut Context, &mut Self, KeyEvent)>>,
}

impl<T: TreeItem> TreeView<T> {
    pub fn new(root: T, items: Vec<Tree<T>>) -> Self {
        Self {
            tree: Tree::new(root, items),
            recycle: None,
            selected: 0,
            save_view: (0, 0),
            winline: 0,
            col: 0,
            max_len: 0,
            count: 0,
            area_height: 0,
            tree_symbol_style: "ui.text".into(),
            pre_render: None,
            on_opened_fn: None,
            on_folded_fn: None,
            on_next_key: None,
        }
    }

    pub fn replace_with_new_items(&mut self, items: Vec<T>) {
        todo!()
        // let old = std::mem::replace(self, Self::new(vec_to_tree(items)));
        // self.on_opened_fn = old.on_opened_fn;
        // self.on_folded_fn = old.on_folded_fn;
        // self.tree_symbol_style = old.tree_symbol_style;
    }

    pub fn build_tree(root: T, items: Vec<T>) -> Self {
        Self::new(root, vec_to_tree(items))
    }

    pub fn with_enter_fn<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut T, &mut Context, &mut T::Params) -> TreeOp<T> + 'static,
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

    fn find<F>(&self, start: usize, reverse: bool, f: F) -> Option<usize>
    where
        F: FnMut(&Tree<T>) -> bool,
    {
        let iter = self.tree.iter();
        if reverse {
            iter.take(start).rposition(f)
        } else {
            iter.skip(start).position(f).map(|index| index + start)
        }
    }

    /// Reveal item in the tree based on the given `segments`.
    ///
    /// The name of the root should be excluded.
    ///
    /// Example `segments`:
    /// ```
    /// vec!["helix-term", "src", "ui", "tree.rs"]
    /// ```
    pub fn reveal_item(&mut self, segments: Vec<&str>) -> Result<(), String> {
        // Expand the tree
        segments.iter().fold(
            Ok(&mut self.tree),
            |current_tree, segment| match current_tree {
                Err(err) => Err(err),
                Ok(current_tree) => {
                    match current_tree
                        .children
                        .iter_mut()
                        .find(|tree| tree.item.text_string().eq(segment))
                    {
                        Some(tree) => {
                            if !tree.is_opened {
                                tree.children = vec_to_tree(
                                    tree.item.get_children().map_err(|err| err.to_string())?,
                                );
                                if !tree.children.is_empty() {
                                    tree.is_opened = true;
                                }
                            }
                            Ok(tree)
                        }
                        None => Err(format!(
                            "Unable to find path: '{}'. current_segment = {}",
                            segments.join("/"),
                            segment
                        )),
                    }
                }
            },
        )?;

        // Locate the item
        self.regenerate_index();
        self.selected = segments
            .iter()
            .fold(&self.tree, |tree, segment| {
                tree.children
                    .iter()
                    .find(|tree| tree.item.text_string().eq(segment))
                    .expect("Should be unreachable")
            })
            .index;

        self.align_view_center();
        Ok(())
    }

    fn align_view_center(&mut self) {
        self.winline = self.area_height / 2
    }

    fn align_view_top(&mut self) {
        self.winline = 0
    }

    fn align_view_bottom(&mut self) {
        self.winline = self.area_height
    }

    fn regenerate_index(&mut self) {
        self.tree.regenerate_index();
    }

    fn go_to_parent(&mut self) {
        if let Some(parent) = self.current_parent() {
            self.selected = parent.index
        }
    }

    fn go_to_children(&mut self, cx: &mut Context) {
        let current = self.current_mut();
        if current.is_opened {
            self.selected += 1;
            return;
        }
        let items = match current.item.get_children() {
            Ok(items) => items,
            Err(e) => return cx.editor.set_error(format!("{e}")),
        };
        if items.is_empty() {
            return;
        }
        current.is_opened = true;
        current.children = vec_to_tree(items);
        self.selected += 1;
        self.regenerate_index()
    }
}

impl<T: TreeItem> TreeView<T> {
    pub fn on_enter(&mut self, cx: &mut Context, params: &mut T::Params, selected_index: usize) {
        // if let Some(next_level) = self.next_item().map(|elem| elem.level) {
        //     let current = self.find_by_index(selected_index);
        //     let current_level = current.level;
        //     if next_level > current_level {
        //         // if let Some(mut on_folded_fn) = self.on_folded_fn.take() {
        //         //     on_folded_fn(&mut current.item, cx, params);
        //         //     self.on_folded_fn = Some(on_folded_fn);
        //         // }
        //         self.fold_current_child();
        //         return;
        //     }
        // }
        //
        let mut selected_item = self.find_by_index_mut(selected_index);
        if selected_item.is_opened {
            selected_item.is_opened = false;
            selected_item.children = vec![];
            self.regenerate_index();
            return;
        }

        if let Some(mut on_open_fn) = self.on_opened_fn.take() {
            let mut f = || {
                let current = &mut self.find_by_index_mut(selected_index);
                match on_open_fn(&mut current.item, cx, params) {
                    TreeOp::Restore => {
                        panic!();
                        // let inserts = std::mem::take(&mut current.folded);
                        // let _: Vec<_> = self
                        //     .items
                        //     .splice(selected_index + 1..selected_index + 1, inserts)
                        //     .collect();
                        return;
                    }
                    TreeOp::InsertChild(items) => {
                        items;
                    }
                    TreeOp::GetChildsAndInsert => {
                        let items = match current.item.get_children() {
                            Ok(items) => items,
                            Err(e) => return cx.editor.set_error(format!("{e}")),
                        };
                        current.is_opened = true;
                        current.children = vec_to_tree(items);
                    }
                    TreeOp::ReplaceTree(items) => {
                        return self.replace_with_new_items(items);
                    }
                    TreeOp::Noop => {}
                };

                // current.folded = vec![];
                // let inserts = vec_to_tree(items, current.level + 1);
                // let _: Vec<_> = self
                //     .items
                //     .splice(selected_index + 1..selected_index + 1, inserts)
                //     .collect();
            };
            f();
            self.regenerate_index();
            self.on_opened_fn = Some(on_open_fn)
        } else {
            panic!();
            self.find_by_index_mut(selected_index).children = vec![];
            // let current = &mut self.items[selected_index];
            // let inserts = std::mem::take(&mut current.folded);
            // let _: Vec<_> = self
            //     .items
            //     .splice(selected_index + 1..selected_index + 1, inserts)
            //     .collect();
        }
    }

    pub fn fold_current_child(&mut self) {
        if let Some(parent) = self.current_parent_mut() {
            parent.is_opened = false;
            parent.children = vec![];
            self.selected = parent.index;
            self.regenerate_index()
        }
    }

    pub fn search_next(&mut self, cx: &mut Context, s: &str, params: &mut T::Params) {
        let skip = std::cmp::max(2, self.save_view.0 + 1);
        self.selected = self
            .find(skip, false, |e| e.item.filter(s))
            .unwrap_or(self.save_view.0);

        self.winline = (self.save_view.1 + self.selected).saturating_sub(self.save_view.0);
    }

    pub fn search_previous(&mut self, cx: &mut Context, s: &str, params: &mut T::Params) {
        let take = self.save_view.0;
        self.selected = self
            .find(take, true, |e| e.item.filter(s))
            .unwrap_or(self.save_view.0);

        self.winline = (self.save_view.1 + self.selected).saturating_sub(self.save_view.0);
    }

    pub fn move_down(&mut self, rows: usize) {
        let len = self.tree.len();
        if len > 0 {
            self.selected = std::cmp::min(self.selected + rows, len.saturating_sub(1));
            self.winline = std::cmp::min(self.selected, self.winline + rows);
        }
    }

    pub fn move_up(&mut self, rows: usize) {
        let len = self.tree.len();
        if len > 0 {
            self.selected = std::cmp::max(0, self.selected.saturating_sub(rows));
            self.winline = std::cmp::min(self.selected, self.winline.saturating_sub(rows));
        }
    }

    pub fn move_left(&mut self, cols: usize) {
        self.col = self.col.saturating_sub(cols);
    }

    pub fn move_right(&mut self, cols: usize) {
        self.pre_render = Some(Box::new(move |tree: &mut Self, area: Rect| {
            let max_scroll = tree.max_len.saturating_sub(area.width as usize);
            tree.col = max_scroll.min(tree.col + cols);
        }));
    }

    pub fn move_down_half_page(&mut self) {
        self.pre_render = Some(Box::new(|tree: &mut Self, area: Rect| {
            tree.move_down((area.height / 2) as usize);
        }));
    }

    pub fn move_up_half_page(&mut self) {
        self.pre_render = Some(Box::new(|tree: &mut Self, area: Rect| {
            tree.move_up((area.height / 2) as usize);
        }));
    }

    pub fn move_down_page(&mut self) {
        self.pre_render = Some(Box::new(|tree: &mut Self, area: Rect| {
            tree.move_down((area.height) as usize);
        }));
    }

    pub fn move_up_page(&mut self) {
        self.pre_render = Some(Box::new(|tree: &mut Self, area: Rect| {
            tree.move_up((area.height) as usize);
        }));
    }

    pub fn save_view(&mut self) {
        self.save_view = (self.selected, self.winline);
    }

    pub fn restore_view(&mut self) {
        (self.selected, self.winline) = self.save_view;
    }

    fn find_by_index(&self, index: usize) -> &Tree<T> {
        self.tree
            .iter()
            .find_map(|item| item.find_by_index(index))
            .unwrap()
    }

    fn find_by_index_mut(&mut self, index: usize) -> &mut Tree<T> {
        self.tree.find_by_index_mut(index).unwrap()
    }

    pub fn current(&self) -> &Tree<T> {
        self.find_by_index(self.selected)
    }

    pub fn current_mut(&mut self) -> &mut Tree<T> {
        self.find_by_index_mut(self.selected)
    }

    fn current_parent(&self) -> Option<&Tree<T>> {
        if let Some(parent_index) = self.current().parent_index {
            Some(self.find_by_index(parent_index))
        } else {
            None
        }
    }

    fn current_parent_mut(&mut self) -> Option<&mut Tree<T>> {
        if let Some(parent_index) = self.current().parent_index {
            Some(self.find_by_index_mut(parent_index))
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

    pub fn remove_current(&mut self) -> T {
        todo!()
        // let elem = self.tree.remove(self.selected);
        // self.selected = self.selected.saturating_sub(1);
        // elem.item
    }

    pub fn replace_current(&mut self, item: T) {
        self.current_mut().item = item
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected
    }

    pub fn insert_current_level(&mut self, item: T) {
        let current = self.current_mut();
        current.children.push(Tree::new(item, vec![]));
        current
            .children
            .sort_by(|a, b| tree_item_cmp(&a.item, &b.item));
        self.regenerate_index()
    }
}

impl<T: TreeItem> TreeView<T> {
    pub fn render(
        &mut self,
        area: Rect,
        surface: &mut Surface,
        cx: &mut Context,
        params: &mut T::Params,
    ) {
        if let Some(pre_render) = self.pre_render.take() {
            pre_render(self, area);
        }

        self.max_len = 0;
        self.area_height = area.height.saturating_sub(1) as usize;
        self.winline = std::cmp::min(self.winline, self.area_height);
        let style = cx.editor.theme.get(&self.tree_symbol_style);
        let last_item_index = self.tree.len().saturating_sub(1);
        let skip = self.selected.saturating_sub(self.winline);

        let params = RenderElemParams {
            tree: &self.tree,
            prefix: &"".to_string(),
            is_last: true,
            level: 0,
            selected: self.selected,
        };

        let rendered = render_tree(params);

        let iter = rendered
            .iter()
            .skip(skip)
            .take(area.height as usize)
            .enumerate();

        struct Indent(String);
        struct Node {
            name: String,
            selected: bool,
        }

        struct RenderElemParams<'a, T> {
            tree: &'a Tree<T>,
            prefix: &'a String,
            is_last: bool,
            level: u16,
            selected: usize,
        }

        fn render_tree<T: TreeItem>(
            RenderElemParams {
                tree,
                prefix,
                is_last,
                level,
                selected,
            }: RenderElemParams<T>,
        ) -> Vec<(Indent, Node)> {
            let indent = if level > 0 {
                let bar = if is_last { "└" } else { "├" };
                let branch = if tree.is_opened { "┬" } else { "─" };
                format!("{}{}{}", prefix, bar, branch)
            } else {
                "".to_string()
            };
            let folded_length = tree.children.len();
            let head = (
                Indent(indent),
                Node {
                    selected: selected == tree.index,
                    name: format!(
                        "{}{}",
                        tree.item.text_string(),
                        if tree.item.is_parent() {
                            format!("{}", std::path::MAIN_SEPARATOR)
                        } else {
                            "".to_string()
                        }
                    ),
                },
            );
            let prefix = format!("{}{}", prefix, if is_last { " " } else { "│" });
            vec![head]
                .into_iter()
                .chain(
                    tree.children
                        .iter()
                        .enumerate()
                        .flat_map(|(local_index, elem)| {
                            let is_last = local_index == folded_length - 1;
                            render_tree(RenderElemParams {
                                tree: elem,
                                prefix: &prefix,
                                is_last,
                                level: level + 1,
                                selected,
                            })
                        }),
                )
                .collect()
        }

        for (index, (indent, node)) in iter {
            let area = Rect::new(area.x, area.y + index as u16, area.width, 1);
            let indent_len = indent.0.chars().count() as u16;
            surface.set_stringn(area.x, area.y, indent.0.clone(), indent_len as usize, style);

            let style = if node.selected {
                style.add_modifier(Modifier::REVERSED)
            } else {
                style
            };
            surface.set_stringn(
                area.x.saturating_add(indent_len).saturating_add(1),
                area.y,
                node.name.clone(),
                area.width
                    .saturating_sub(indent_len)
                    .saturating_sub(1)
                    .into(),
                style,
            );
        }
        // let mut text = elem.item.text(cx, skip + index == self.selected, params);
        // for (index, elem) in iter {
        //     let row = index as u16;
        //     let mut area = Rect::new(area.x, area.y + row, area.width, 1);
        //     let indent = if elem.level > 0 {
        //         if index + skip != last_item_index {
        //             format!("{}├─", "│ ".repeat(elem.level - 1))
        //         } else {
        //             format!("└─{}", "┴─".repeat(elem.level - 1))
        //         }
        //     } else {
        //         "".to_string()
        //     };

        //     let indent_len = indent.chars().count();
        //     if indent_len > self.col {
        //         let indent: String = indent.chars().skip(self.col).collect();
        //         if !indent.is_empty() {
        //             surface.set_stringn(area.x, area.y, &indent, area.width as usize, style);
        //             area = area.clip_left(indent.width() as u16);
        //         }
        //     };
        //     let mut start_index = self.col.saturating_sub(indent_len);
        //     let mut text = elem.item.text(cx, skip + index == self.selected, params);
        //     self.max_len = self.max_len.max(text.width() + indent.len());
        //     for span in text.0.iter_mut() {
        //         if area.width == 0 {
        //             return;
        //         }
        //         if start_index == 0 {
        //             surface.set_span(area.x, area.y, span, area.width);
        //             area = area.clip_left(span.width() as u16);
        //         } else {
        //             let span_width = span.width();
        //             if start_index > span_width {
        //                 start_index -= span_width;
        //             } else {
        //                 let content: String = span
        //                     .content
        //                     .chars()
        //                     .filter(|c| {
        //                         if start_index > 0 {
        //                             start_index = start_index.saturating_sub(c.to_string().width());
        //                             false
        //                         } else {
        //                             true
        //                         }
        //                     })
        //                     .collect();
        //                 surface.set_string_truncated(
        //                     area.x,
        //                     area.y,
        //                     &content,
        //                     area.width as usize,
        //                     |_| span.style,
        //                     false,
        //                     false,
        //                 );
        //                 start_index = 0
        //             }
        //         }
        //     }
        // }
    }

    pub fn handle_event(
        &mut self,
        event: Event,
        cx: &mut Context,
        params: &mut T::Params,
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
        match key_event.into() {
            key!(i @ '0'..='9') => self.count = i.to_digit(10).unwrap() as usize + count * 10,
            key!('k') | shift!(Tab) | key!(Up) | ctrl!('k') => self.move_up(1.max(count)),
            key!('j') | key!(Tab) | key!(Down) | ctrl!('j') => self.move_down(1.max(count)),
            key!('z') => {
                self.on_next_key = Some(Box::new(|_, tree, event| match event.into() {
                    key!('f') => tree.fold_current_child(),
                    key!('z') => tree.align_view_center(),
                    key!('t') => tree.align_view_top(),
                    key!('b') => tree.align_view_bottom(),
                    _ => {}
                }));
            }
            key!('h') => self.go_to_parent(),
            key!('l') => self.go_to_children(cx),
            key!(Enter) => self.on_enter(cx, params, self.selected),
            ctrl!('d') => self.move_down_half_page(),
            ctrl!('u') => self.move_up_half_page(),
            key!('g') => {
                self.on_next_key = Some(Box::new(|_, tree, event| match event.into() {
                    key!('g') => tree.move_up(usize::MAX / 2),
                    key!('e') => tree.move_down(usize::MAX / 2),
                    _ => {}
                }));
            }
            _ => return EventResult::Ignored(None),
        }

        EventResult::Consumed(None)
    }
}

impl<T: TreeItem + Clone> TreeView<T> {
    pub fn filter(&mut self, s: &str, cx: &mut Context, params: &mut T::Params) {
        todo!()
        // fn filter_recursion<T>(
        //     elems: &Vec<Tree<T>>,
        //     mut index: usize,
        //     s: &str,
        //     cx: &mut Context,
        //     params: &mut T::Params,
        // ) -> (Vec<Tree<T>>, usize)
        // where
        //     T: TreeItem + Clone,
        // {
        //     let mut retain = vec![];
        //     let elem = &elems[index];
        //     loop {
        //         let child = match elems.get(index + 1) {
        //             Some(child) if child.item.is_child(&elem.item) => child,
        //             _ => break,
        //         };
        //         index += 1;
        //         let next = elems.get(index + 1);
        //         if next.map_or(false, |n| n.item.is_child(&child.item)) {
        //             let (sub_retain, current_index) = filter_recursion(elems, index, s, cx, params);
        //             retain.extend(sub_retain);
        //             index = current_index;
        //         } else if child.item.filter(s) {
        //             retain.push(child.clone());
        //         }
        //     }
        //     if !retain.is_empty() || elem.item.filter(s) {
        //         retain.insert(0, elem.clone());
        //     }
        //     (retain, index)
        // }

        // if s.is_empty() {
        //     if let Some((_, recycle)) = self.recycle.take() {
        //         // self.tree = recycle;
        //         self.restore_view();
        //         return;
        //     }
        // }

        // let mut retain = vec![];
        // let mut index = 0;
        // let items = match &self.recycle {
        //     Some((pre, _)) if pre == s => return,
        //     Some((pre, recycle)) if pre.contains(s) => recycle,
        //     _ => &self.tree,
        // };
        // while let Some(elem) = items.get(index) {
        //     let next = items.get(index + 1);
        //     if next.map_or(false, |n| n.item.is_child(&elem.item)) {
        //         let (sub_items, current_index) = filter_recursion(items, index, s, cx, params);
        //         index = current_index;
        //         retain.extend(sub_items);
        //     } else if elem.item.filter(s) {
        //         retain.push(elem.clone())
        //     }
        //     index += 1;
        // }

        // if retain.is_empty() {
        //     if let Some((_, recycle)) = self.recycle.take() {
        //         self.tree = recycle;
        //         self.restore_view();
        //     }
        //     return;
        // }

        // let recycle = std::mem::replace(&mut self.tree, retain);
        // if let Some(r) = self.recycle.as_mut() {
        //     r.0 = s.into()
        // } else {
        //     self.recycle = Some((s.into(), recycle));
        //     self.save_view();
        // }

        // self.selected = self.find(0, false, |elem| elem.item.filter(s)).unwrap_or(0);
        // self.winline = self.selected;
    }

    pub fn clean_recycle(&mut self) {
        self.recycle = None;
    }

    pub fn restore_recycle(&mut self) {
        todo!();
        // if let Some((_, recycle)) = self.recycle.take() {
        //     self.tree = recycle;
        // }
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
fn index_elems<T>(start_index: usize, elems: Vec<Tree<T>>) -> Vec<Tree<T>> {
    fn index_elems<'a, T>(
        current_index: usize,
        elems: Vec<Tree<T>>,
        parent_index: Option<usize>,
    ) -> (usize, Vec<Tree<T>>) {
        elems
            .into_iter()
            .fold((current_index, vec![]), |(current_index, trees), elem| {
                let index = current_index;
                let item = elem.item;
                let (current_index, folded) =
                    index_elems(current_index + 1, elem.children, Some(index));
                let tree = Tree {
                    item,
                    children: folded,
                    index,
                    is_opened: elem.is_opened,
                    parent_index,
                };
                (
                    current_index,
                    trees.into_iter().chain(vec![tree].into_iter()).collect(),
                )
            })
    }
    index_elems(start_index, elems, None).1
}

#[cfg(test)]
mod test_tree {
    use super::{index_elems, Tree};

    #[test]
    fn test_indexs_elems_1() {
        let result = index_elems(
            0,
            vec![
                Tree::new("foo", vec![Tree::new("bar", vec![])]),
                Tree::new(
                    "spam",
                    vec![Tree::new("jar", vec![Tree::new("yo", vec![])])],
                ),
            ],
        );
        assert_eq!(
            result,
            vec![
                Tree {
                    item: "foo",
                    is_opened: false,
                    index: 0,
                    parent_index: None,
                    children: vec![Tree {
                        item: "bar",
                        is_opened: false,
                        index: 1,
                        parent_index: Some(0),
                        children: vec![]
                    }]
                },
                Tree {
                    item: "spam",
                    is_opened: false,
                    index: 2,
                    parent_index: None,
                    children: vec![Tree {
                        item: "jar",
                        is_opened: false,
                        index: 3,
                        parent_index: Some(2),
                        children: vec![Tree {
                            item: "yo",
                            is_opened: false,
                            index: 4,
                            children: vec![],
                            parent_index: Some(3)
                        }]
                    }]
                }
            ]
        )
    }

    #[test]
    fn test_iter_1() {
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
    fn test_len_1() {
        let tree = Tree::new(
            "spam",
            vec![
                Tree::new("jar", vec![Tree::new("yo", vec![])]),
                Tree::new("foo", vec![Tree::new("bar", vec![])]),
            ],
        );

        assert_eq!(tree.len(), 5)
    }
}
