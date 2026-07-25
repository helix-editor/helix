//! Tree widget for hierarchical data.
//!
//! Ported from `tui-tree-widget` 0.24.0 and adapted to Helix's local TUI
//! types and widget conventions.

use crate::{
    buffer::Buffer,
    text::Text,
    widgets::{Block, Widget},
};
use helix_core::unicode::width::UnicodeWidthStr;
use helix_view::graphics::{Rect, Style};
use std::collections::{hash_set, HashSet};

/// One item inside a [`Tree`].
///
/// The identifier is used by [`TreeState`] to keep track of selected and open
/// nodes. Identifiers only need to be unique among siblings.
#[derive(Debug, Clone)]
pub struct TreeItem<'text, Identifier> {
    identifier: Identifier,
    text: Text<'text>,
    children: Vec<Self>,
}

impl<'text, Identifier> TreeItem<'text, Identifier>
where
    Identifier: Clone + PartialEq + Eq + std::hash::Hash,
{
    /// Create a new `TreeItem` without children.
    pub fn new_leaf<T>(identifier: Identifier, text: T) -> Self
    where
        T: Into<Text<'text>>,
    {
        Self {
            identifier,
            text: text.into(),
            children: Vec::new(),
        }
    }

    /// Create a new `TreeItem` with children.
    ///
    /// Returns an error when any child identifiers are duplicated.
    pub fn new<T>(identifier: Identifier, text: T, children: Vec<Self>) -> std::io::Result<Self>
    where
        T: Into<Text<'text>>,
    {
        let identifiers = children
            .iter()
            .map(|item| &item.identifier)
            .collect::<HashSet<_>>();
        if identifiers.len() != children.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "The children contain duplicate identifiers",
            ));
        }

        Ok(Self {
            identifier,
            text: text.into(),
            children,
        })
    }

    pub const fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    pub fn text(&self) -> &Text<'text> {
        &self.text
    }

    pub fn children(&self) -> &[Self] {
        &self.children
    }

    pub fn child(&self, index: usize) -> Option<&Self> {
        self.children.get(index)
    }

    /// Get a mutable reference to a child by index.
    ///
    /// Changing identifiers after rendering can make existing [`TreeState`]
    /// selections point at stale nodes.
    pub fn child_mut(&mut self, index: usize) -> Option<&mut Self> {
        self.children.get_mut(index)
    }

    pub fn height(&self) -> usize {
        self.text.height()
    }

    /// Add a child.
    ///
    /// Returns an error when the child's identifier already exists among this
    /// node's children.
    pub fn add_child(&mut self, child: Self) -> std::io::Result<()> {
        let existing = self
            .children
            .iter()
            .map(|item| &item.identifier)
            .collect::<HashSet<_>>();
        if existing.contains(&child.identifier) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "identifier already exists in the children",
            ));
        }

        self.children.push(child);
        Ok(())
    }
}

impl TreeItem<'static, &'static str> {
    #[cfg(test)]
    fn example() -> Vec<Self> {
        vec![
            Self::new_leaf("a", "Alfa"),
            Self::new(
                "b",
                "Bravo",
                vec![
                    Self::new_leaf("c", "Charlie"),
                    Self::new(
                        "d",
                        "Delta",
                        vec![Self::new_leaf("e", "Echo"), Self::new_leaf("f", "Foxtrot")],
                    )
                    .expect("all item identifiers are unique"),
                    Self::new_leaf("g", "Golf"),
                ],
            )
            .expect("all item identifiers are unique"),
            Self::new_leaf("h", "Hotel"),
        ]
    }
}

/// A flattened visible tree item.
pub struct Flattened<'text, Identifier> {
    pub identifier: Vec<Identifier>,
    pub item: &'text TreeItem<'text, Identifier>,
}

impl<Identifier> Flattened<'_, Identifier> {
    /// Zero-based depth. Depth 0 means top-level with no indentation.
    pub fn depth(&self) -> usize {
        self.identifier.len() - 1
    }
}

fn flatten<'text, Identifier>(
    open_identifiers: &HashSet<Vec<Identifier>>,
    items: &'text [TreeItem<'text, Identifier>],
    current: &[Identifier],
) -> Vec<Flattened<'text, Identifier>>
where
    Identifier: Clone + PartialEq + Eq + std::hash::Hash,
{
    let mut result = Vec::new();
    for item in items {
        let mut child_identifier = current.to_vec();
        child_identifier.push(item.identifier.clone());

        let child_result = open_identifiers
            .contains(&child_identifier)
            .then(|| flatten(open_identifiers, &item.children, &child_identifier));

        result.push(Flattened {
            identifier: child_identifier,
            item,
        });

        if let Some(mut child_result) = child_result {
            result.append(&mut child_result);
        }
    }
    result
}

/// State for selected, open, and scrolled tree nodes.
#[derive(Debug)]
pub struct TreeState<Identifier> {
    offset: usize,
    opened: HashSet<Vec<Identifier>>,
    selected: Vec<Identifier>,
    ensure_selected_in_view_on_next_render: bool,

    last_area: Rect,
    last_biggest_index: usize,
    last_identifiers: Vec<Vec<Identifier>>,
    last_rendered_identifiers: Vec<(u16, Vec<Identifier>)>,
}

impl<Identifier> Default for TreeState<Identifier> {
    fn default() -> Self {
        Self {
            offset: 0,
            opened: HashSet::new(),
            selected: Vec::new(),
            ensure_selected_in_view_on_next_render: false,

            last_area: Rect::default(),
            last_biggest_index: 0,
            last_identifiers: Vec::new(),
            last_rendered_identifiers: Vec::new(),
        }
    }
}

impl<Identifier> TreeState<Identifier>
where
    Identifier: Clone + PartialEq + Eq + std::hash::Hash,
{
    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub const fn opened(&self) -> &HashSet<Vec<Identifier>> {
        &self.opened
    }

    pub fn opened_mut(&mut self) -> &mut HashSet<Vec<Identifier>> {
        &mut self.opened
    }

    pub fn opened_iter(&self) -> hash_set::Iter<'_, Vec<Identifier>> {
        self.opened.iter()
    }

    pub fn selected(&self) -> &[Identifier] {
        &self.selected
    }

    /// Get all currently visible tree items.
    pub fn flatten<'text>(
        &self,
        items: &'text [TreeItem<'text, Identifier>],
    ) -> Vec<Flattened<'text, Identifier>> {
        flatten(&self.opened, items, &[])
    }

    /// Select the given identifier. Pass an empty vector to clear selection.
    ///
    /// Returns `true` when the selection changed.
    pub fn select(&mut self, identifier: Vec<Identifier>) -> bool {
        self.ensure_selected_in_view_on_next_render = true;
        let changed = self.selected != identifier;
        self.selected = identifier;
        changed
    }

    /// Open a tree node.
    ///
    /// Returns `true` when the node was previously closed.
    pub fn open(&mut self, identifier: Vec<Identifier>) -> bool {
        if identifier.is_empty() {
            false
        } else {
            self.opened.insert(identifier)
        }
    }

    /// Close a tree node.
    ///
    /// Returns `true` when the node was previously open.
    pub fn close(&mut self, identifier: &[Identifier]) -> bool {
        self.opened.remove(identifier)
    }

    /// Toggle a tree node open or closed.
    pub fn toggle(&mut self, identifier: Vec<Identifier>) -> bool {
        if identifier.is_empty() {
            false
        } else if self.opened.contains(&identifier) {
            self.close(&identifier)
        } else {
            self.open(identifier)
        }
    }

    /// Toggle the selected node open or closed.
    pub fn toggle_selected(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }

        self.ensure_selected_in_view_on_next_render = true;

        let was_open = self.opened.remove(&self.selected);
        if was_open {
            return true;
        }

        self.open(self.selected.clone())
    }

    /// Close all open nodes.
    pub fn close_all(&mut self) -> bool {
        if self.opened.is_empty() {
            false
        } else {
            self.opened.clear();
            true
        }
    }

    pub fn select_first(&mut self) -> bool {
        let identifier = self.last_identifiers.first().cloned().unwrap_or_default();
        self.select(identifier)
    }

    pub fn select_last(&mut self) -> bool {
        let identifier = self.last_identifiers.last().cloned().unwrap_or_default();
        self.select(identifier)
    }

    pub fn select_relative<F>(&mut self, change_function: F) -> bool
    where
        F: FnOnce(Option<usize>) -> usize,
    {
        let current_index = self
            .last_identifiers
            .iter()
            .position(|identifier| identifier == &self.selected);
        let new_index = change_function(current_index).min(self.last_biggest_index);
        let new_identifier = self
            .last_identifiers
            .get(new_index)
            .cloned()
            .unwrap_or_default();
        self.select(new_identifier)
    }

    /// Get the identifier rendered at the given coordinates during the last render.
    pub fn rendered_at(&self, x: u16, y: u16) -> Option<&[Identifier]> {
        if !rect_contains(self.last_area, x, y) {
            return None;
        }

        self.last_rendered_identifiers
            .iter()
            .rev()
            .find(|(rendered_y, _)| y >= *rendered_y)
            .map(|(_, identifier)| identifier.as_ref())
    }

    /// Select what was rendered at the given coordinates. If it is already
    /// selected, toggle it.
    pub fn click_at(&mut self, x: u16, y: u16) -> bool {
        if let Some(identifier) = self.rendered_at(x, y) {
            if identifier == self.selected {
                self.toggle_selected()
            } else {
                self.select(identifier.to_vec())
            }
        } else {
            false
        }
    }

    pub const fn scroll_selected_into_view(&mut self) {
        self.ensure_selected_in_view_on_next_render = true;
    }

    pub const fn scroll_up(&mut self, lines: usize) -> bool {
        let before = self.offset;
        self.offset = self.offset.saturating_sub(lines);
        before != self.offset
    }

    pub fn scroll_down(&mut self, lines: usize) -> bool {
        let before = self.offset;
        self.offset = self
            .offset
            .saturating_add(lines)
            .min(self.last_biggest_index);
        before != self.offset
    }

    pub fn key_up(&mut self) -> bool {
        self.select_relative(|current| {
            current.map_or(usize::MAX, |current| current.saturating_sub(1))
        })
    }

    pub fn key_down(&mut self) -> bool {
        self.select_relative(|current| current.map_or(0, |current| current.saturating_add(1)))
    }

    pub fn key_left(&mut self) -> bool {
        self.ensure_selected_in_view_on_next_render = true;
        let mut changed = self.opened.remove(&self.selected);
        if !changed {
            changed = self.selected.pop().is_some();
        }
        changed
    }

    pub fn key_right(&mut self) -> bool {
        if self.selected.is_empty() {
            false
        } else {
            self.ensure_selected_in_view_on_next_render = true;
            self.open(self.selected.clone())
        }
    }
}

/// A renderable tree.
#[derive(Debug, Clone)]
pub struct Tree<'a, Identifier> {
    items: &'a [TreeItem<'a, Identifier>],

    block: Option<Block<'a>>,
    style: Style,
    highlight_style: Style,
    highlight_symbol: &'a str,

    node_closed_symbol: &'a str,
    node_open_symbol: &'a str,
    node_no_children_symbol: &'a str,
}

impl<'a, Identifier> Tree<'a, Identifier>
where
    Identifier: Clone + PartialEq + Eq + std::hash::Hash,
{
    /// Create a new `Tree`.
    ///
    /// Returns an error when top-level identifiers are duplicated.
    pub fn new(items: &'a [TreeItem<'a, Identifier>]) -> std::io::Result<Self> {
        let identifiers = items
            .iter()
            .map(|item| &item.identifier)
            .collect::<HashSet<_>>();
        if identifiers.len() != items.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "The items contain duplicate identifiers",
            ));
        }

        Ok(Self {
            items,
            block: None,
            style: Style::default(),
            highlight_style: Style::default(),
            highlight_symbol: "",
            node_closed_symbol: "\u{25b6} ",
            node_open_symbol: "\u{25bc} ",
            node_no_children_symbol: "  ",
        })
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub const fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub const fn highlight_symbol(mut self, highlight_symbol: &'a str) -> Self {
        self.highlight_symbol = highlight_symbol;
        self
    }

    pub const fn node_closed_symbol(mut self, symbol: &'a str) -> Self {
        self.node_closed_symbol = symbol;
        self
    }

    pub const fn node_open_symbol(mut self, symbol: &'a str) -> Self {
        self.node_open_symbol = symbol;
        self
    }

    pub const fn node_no_children_symbol(mut self, symbol: &'a str) -> Self {
        self.node_no_children_symbol = symbol;
        self
    }

    pub fn render_tree(
        mut self,
        full_area: Rect,
        buf: &mut Buffer,
        state: &mut TreeState<Identifier>,
    ) {
        buf.set_style(full_area, self.style);

        let area = match self.block.take() {
            Some(block) => {
                let inner_area = block.inner(full_area);
                block.render(full_area, buf);
                inner_area
            }
            None => full_area,
        };

        state.last_area = area;
        state.last_rendered_identifiers.clear();
        if area.width < 1 || area.height < 1 {
            return;
        }

        let visible = state.flatten(self.items);
        state.last_biggest_index = visible.len().saturating_sub(1);
        if visible.is_empty() {
            return;
        }
        let available_height = area.height as usize;

        let ensure_index_in_view =
            if state.ensure_selected_in_view_on_next_render && !state.selected.is_empty() {
                visible
                    .iter()
                    .position(|flattened| flattened.identifier == state.selected)
            } else {
                None
            };

        let mut start = state.offset.min(state.last_biggest_index);
        if let Some(ensure_index_in_view) = ensure_index_in_view {
            start = start.min(ensure_index_in_view);
        }

        let mut end = start;
        let mut height = 0;
        for item_height in visible
            .iter()
            .skip(start)
            .map(|flattened| flattened.item.height())
        {
            if height + item_height > available_height {
                break;
            }
            height += item_height;
            end += 1;
        }

        if let Some(ensure_index_in_view) = ensure_index_in_view {
            while ensure_index_in_view >= end {
                height += visible[end].item.height();
                end += 1;
                while height > available_height {
                    height = height.saturating_sub(visible[start].item.height());
                    start += 1;
                }
            }
        }

        state.offset = start;
        state.ensure_selected_in_view_on_next_render = false;

        let blank_symbol = " ".repeat(self.highlight_symbol.width());

        let mut current_height = 0;
        let has_selection = !state.selected.is_empty();
        for flattened in visible.iter().skip(state.offset).take(end - start) {
            let Flattened { identifier, item } = flattened;

            let x = area.x;
            let y = area.y + current_height;
            let height = item.height() as u16;
            current_height += height;

            let row_area = Rect {
                x,
                y,
                width: area.width,
                height,
            };

            let is_selected = state.selected == *identifier;
            let after_highlight_symbol_x = if has_selection {
                let symbol = if is_selected {
                    self.highlight_symbol
                } else {
                    &blank_symbol
                };
                let (x, _) = buf.set_stringn(x, y, symbol, row_area.width as usize, self.style);
                x
            } else {
                x
            };

            let after_depth_x = {
                let indent_width = flattened.depth() * 2;
                let (after_indent_x, _) = buf.set_stringn(
                    after_highlight_symbol_x,
                    y,
                    &" ".repeat(indent_width),
                    indent_width,
                    self.style,
                );
                let symbol = if item.children.is_empty() {
                    self.node_no_children_symbol
                } else if state.opened.contains(identifier) {
                    self.node_open_symbol
                } else {
                    self.node_closed_symbol
                };
                let max_width = row_area.width.saturating_sub(after_indent_x - x);
                let (x, _) =
                    buf.set_stringn(after_indent_x, y, symbol, max_width as usize, self.style);
                x
            };

            let text_area = Rect {
                x: after_depth_x,
                width: row_area.width.saturating_sub(after_depth_x - x),
                ..row_area
            };
            render_text(buf, item.text(), text_area);

            if is_selected {
                buf.set_style(row_area, self.highlight_style);
            }

            state
                .last_rendered_identifiers
                .push((row_area.y, identifier.clone()));
        }
        state.last_identifiers = visible
            .into_iter()
            .map(|flattened| flattened.identifier)
            .collect();
    }
}

impl<Identifier> Widget for Tree<'_, Identifier>
where
    Identifier: Clone + PartialEq + Eq + std::hash::Hash,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = TreeState::default();
        self.render_tree(area, buf, &mut state);
    }
}

fn rect_contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x && x < area.right() && y >= area.y && y < area.bottom()
}

fn render_text(buf: &mut Buffer, text: &Text, area: Rect) {
    for (i, spans) in text.lines.iter().enumerate() {
        if i as u16 >= area.height {
            break;
        }
        buf.set_spans(area.x, area.y + i as u16, spans, area.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(width: u16, height: u16, state: &mut TreeState<&'static str>) -> Buffer {
        let items = TreeItem::example();
        let tree = Tree::new(&items).unwrap();
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        tree.render_tree(area, &mut buffer, state);
        buffer
    }

    #[test]
    #[should_panic = "duplicate identifiers"]
    fn tree_new_errors_with_duplicate_identifiers() {
        let item = TreeItem::new_leaf("same", "text");
        let another = item.clone();
        let _: Tree<_> = Tree::new(&[item, another]).unwrap();
    }

    #[test]
    #[should_panic = "duplicate identifiers"]
    fn tree_item_new_errors_with_duplicate_identifiers() {
        let item = TreeItem::new_leaf("same", "text");
        let another = item.clone();
        TreeItem::new("root", "Root", vec![item, another]).unwrap();
    }

    #[test]
    #[should_panic = "identifier already exists"]
    fn tree_item_add_child_errors_with_duplicate_identifiers() {
        let item = TreeItem::new_leaf("same", "text");
        let another = item.clone();
        let mut root = TreeItem::new("root", "Root", vec![item]).unwrap();
        root.add_child(another).unwrap();
    }

    #[test]
    fn flatten_nothing_open_is_top_level() {
        let open = HashSet::new();
        let actual = flatten(&open, &TreeItem::example(), &[])
            .into_iter()
            .map(|flattened| flattened.identifier.into_iter().next_back().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(actual, ["a", "b", "h"]);
    }

    #[test]
    fn flatten_all_open() {
        let mut open = HashSet::new();
        open.insert(vec!["b"]);
        open.insert(vec!["b", "d"]);
        let actual = flatten(&open, &TreeItem::example(), &[])
            .into_iter()
            .map(|flattened| flattened.identifier.into_iter().next_back().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(actual, ["a", "b", "c", "d", "e", "f", "g", "h"]);
    }

    #[test]
    fn does_not_panic() {
        _ = render(0, 0, &mut TreeState::default());
        _ = render(10, 0, &mut TreeState::default());
        _ = render(0, 10, &mut TreeState::default());
        _ = render(10, 10, &mut TreeState::default());
    }

    #[test]
    fn nothing_open() {
        let buffer = render(10, 4, &mut TreeState::default());
        #[rustfmt::skip]
        let expected = Buffer::with_lines(vec![
            "  Alfa    ",
            "▶ Bravo   ",
            "  Hotel   ",
            "          ",
        ]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn depth_one() {
        let mut state = TreeState::default();
        state.open(vec!["b"]);
        let buffer = render(13, 7, &mut state);
        #[rustfmt::skip]
        let expected = Buffer::with_lines(vec![
            "  Alfa       ",
            "▼ Bravo      ",
            "    Charlie  ",
            "  ▶ Delta    ",
            "    Golf     ",
            "  Hotel      ",
            "             ",
        ]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn depth_two() {
        let mut state = TreeState::default();
        state.open(vec!["b"]);
        state.open(vec!["b", "d"]);
        let buffer = render(15, 9, &mut state);
        #[rustfmt::skip]
        let expected = Buffer::with_lines(vec![
            "  Alfa         ",
            "▼ Bravo        ",
            "    Charlie    ",
            "  ▼ Delta      ",
            "      Echo     ",
            "      Foxtrot  ",
            "    Golf       ",
            "  Hotel        ",
            "               ",
        ]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn key_navigation_uses_last_rendered_items() {
        let mut state = TreeState::default();
        _ = render(10, 4, &mut state);

        assert!(state.key_down());
        assert_eq!(state.selected(), &["a"]);
        assert!(state.key_down());
        assert_eq!(state.selected(), &["b"]);
        assert!(state.key_right());
        _ = render(10, 4, &mut state);
        assert!(state.key_down());
        assert_eq!(state.selected(), &["b", "c"]);
    }

    #[test]
    fn click_at_selects_rendered_item() {
        let mut state = TreeState::default();
        _ = render(10, 4, &mut state);

        assert!(state.click_at(0, 1));
        assert_eq!(state.selected(), &["b"]);
        assert!(state.click_at(0, 1));
        assert!(state.opened().contains(&vec!["b"]));
    }
}
