use std::cmp::Ordering;
use std::iter::Peekable;

use anyhow::Result;

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

    fn text(&self, cx: &mut Context, selected: bool, params: &mut Self::Params) -> Spans;
    fn is_child(&self, other: &Self) -> bool;
    fn cmp(&self, other: &Self) -> Ordering;

    fn filter(&self, cx: &mut Context, s: &str, params: &mut Self::Params) -> bool {
        self.text(cx, false, params)
            .0
            .into_iter()
            .map(|s| s.content)
            .collect::<Vec<_>>()
            .concat()
            .contains(s)
    }

    fn get_childs(&self) -> Result<Vec<Self>> {
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

fn vec_to_tree<T: TreeItem>(mut items: Vec<T>, level: usize) -> Vec<Elem<T>> {
    fn get_childs<T, Iter>(iter: &mut Peekable<Iter>, elem: &mut Elem<T>)
    where
        T: TreeItem,
        Iter: Iterator<Item = T>,
    {
        let level = elem.level + 1;
        loop {
            if !iter.peek().map_or(false, |next| next.is_child(&elem.item)) {
                break;
            }
            let mut child = Elem::new(iter.next().unwrap(), level);
            if iter.peek().map_or(false, |nc| nc.is_child(&child.item)) {
                get_childs(iter, &mut child);
            }
            elem.folded.push(child);
        }
    }

    items.sort_by(tree_item_cmp);
    let mut elems = Vec::with_capacity(items.len());
    let mut iter = items.into_iter().peekable();
    while let Some(item) = iter.next() {
        let mut elem = Elem::new(item, level);
        if iter.peek().map_or(false, |next| next.is_child(&elem.item)) {
            get_childs(&mut iter, &mut elem);
        }
        expand_elems(&mut elems, elem);
    }
    elems
}

// return total elems's count contain self
fn get_elems_recursion<T: TreeItem>(t: &mut Elem<T>, depth: usize) -> Result<usize> {
    let mut childs = t.item.get_childs()?;
    childs.sort_by(tree_item_cmp);
    let mut elems = Vec::with_capacity(childs.len());
    let level = t.level + 1;
    let mut total = 1;
    for child in childs {
        let mut elem = Elem::new(child, level);
        let count = if depth > 0 {
            get_elems_recursion(&mut elem, depth - 1)?
        } else {
            1
        };
        elems.push(elem);
        total += count;
    }
    t.folded = elems;
    Ok(total)
}

fn expand_elems<T: TreeItem>(dist: &mut Vec<Elem<T>>, mut t: Elem<T>) {
    let childs = std::mem::take(&mut t.folded);
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

pub struct Elem<T> {
    item: T,
    level: usize,
    folded: Vec<Self>,
}

impl<T: Clone> Clone for Elem<T> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            level: self.level,
            folded: self.folded.clone(),
        }
    }
}

impl<T> Elem<T> {
    pub fn new(item: T, level: usize) -> Self {
        Self {
            item,
            level,
            folded: vec![],
        }
    }

    pub fn item(&self) -> &T {
        &self.item
    }
}

pub struct Tree<T: TreeItem> {
    items: Vec<Elem<T>>,
    recycle: Option<(String, Vec<Elem<T>>)>,
    selected: usize,           // select item index
    save_view: (usize, usize), // (selected, row)
    winline: usize,            // view row
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

impl<T: TreeItem> Tree<T> {
    pub fn new(items: Vec<Elem<T>>) -> Self {
        Self {
            items,
            recycle: None,
            selected: 0,
            save_view: (0, 0),
            winline: 0,
            col: 0,
            max_len: 0,
            count: 0,
            tree_symbol_style: "ui.text".into(),
            pre_render: None,
            on_opened_fn: None,
            on_folded_fn: None,
            on_next_key: None,
        }
    }

    pub fn replace_with_new_items(&mut self, items: Vec<T>) {
        let old = std::mem::replace(self, Self::new(vec_to_tree(items, 0)));
        self.on_opened_fn = old.on_opened_fn;
        self.on_folded_fn = old.on_folded_fn;
        self.tree_symbol_style = old.tree_symbol_style;
    }

    pub fn build_tree(items: Vec<T>) -> Self {
        Self::new(vec_to_tree(items, 0))
    }

    pub fn build_from_root(t: T, depth: usize) -> Result<Self> {
        let mut elem = Elem::new(t, 0);
        let count = get_elems_recursion(&mut elem, depth)?;
        let mut elems = Vec::with_capacity(count);
        expand_elems(&mut elems, elem);
        Ok(Self::new(elems))
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

    fn next_item(&self) -> Option<&Elem<T>> {
        self.items.get(self.selected + 1)
    }

    fn next_not_descendant_pos(&self, index: usize) -> usize {
        let item = &self.items[index];
        self.find(index + 1, false, |n| n.level <= item.level)
            .unwrap_or(self.items.len())
    }

    fn find_parent(&self, index: usize) -> Option<usize> {
        let item = &self.items[index];
        self.find(index, true, |p| p.level < item.level)
    }

    // rev start: start - 1
    fn find<F>(&self, start: usize, rev: bool, f: F) -> Option<usize>
    where
        F: FnMut(&Elem<T>) -> bool,
    {
        let iter = self.items.iter();
        if rev {
            iter.take(start).rposition(f)
        } else {
            iter.skip(start).position(f).map(|p| p + start)
        }
    }
}

impl<T: TreeItem> Tree<T> {
    pub fn on_enter(&mut self, cx: &mut Context, params: &mut T::Params) {
        if self.items.is_empty() {
            return;
        }
        if let Some(next_level) = self.next_item().map(|elem| elem.level) {
            let current = &mut self.items[self.selected];
            let current_level = current.level;
            if next_level > current_level {
                if let Some(mut on_folded_fn) = self.on_folded_fn.take() {
                    on_folded_fn(&mut current.item, cx, params);
                    self.on_folded_fn = Some(on_folded_fn);
                }
                self.fold_current_child();
                return;
            }
        }

        if let Some(mut on_open_fn) = self.on_opened_fn.take() {
            let mut f = || {
                let current = &mut self.items[self.selected];
                let items = match on_open_fn(&mut current.item, cx, params) {
                    TreeOp::Restore => {
                        let inserts = std::mem::take(&mut current.folded);
                        let _: Vec<_> = self
                            .items
                            .splice(self.selected + 1..self.selected + 1, inserts)
                            .collect();
                        return;
                    }
                    TreeOp::InsertChild(items) => items,
                    TreeOp::GetChildsAndInsert => match current.item.get_childs() {
                        Ok(items) => items,
                        Err(e) => return cx.editor.set_error(format!("{e}")),
                    },
                    TreeOp::ReplaceTree(items) => return self.replace_with_new_items(items),
                    TreeOp::Noop => return,
                };
                current.folded = vec![];
                let inserts = vec_to_tree(items, current.level + 1);
                let _: Vec<_> = self
                    .items
                    .splice(self.selected + 1..self.selected + 1, inserts)
                    .collect();
            };
            f();
            self.on_opened_fn = Some(on_open_fn)
        } else {
            let current = &mut self.items[self.selected];
            let inserts = std::mem::take(&mut current.folded);
            let _: Vec<_> = self
                .items
                .splice(self.selected + 1..self.selected + 1, inserts)
                .collect();
        }
    }

    pub fn fold_current_level(&mut self) {
        let start = match self.find_parent(self.selected) {
            Some(start) => start,
            None => return,
        };
        self.selected = start;
        self.fold_current_child();
    }

    pub fn fold_current_child(&mut self) {
        if self.selected + 1 >= self.items.len() {
            return;
        }
        let pos = self.next_not_descendant_pos(self.selected);
        if self.selected < pos {
            self.items[self.selected].folded = self.items.drain(self.selected + 1..pos).collect();
        }
    }

    pub fn search_next(&mut self, cx: &mut Context, s: &str, params: &mut T::Params) {
        let skip = std::cmp::max(2, self.save_view.0 + 1);
        self.selected = self
            .find(skip, false, |e| e.item.filter(cx, s, params))
            .unwrap_or(self.save_view.0);

        self.winline = (self.save_view.1 + self.selected).saturating_sub(self.save_view.0);
    }

    pub fn search_pre(&mut self, cx: &mut Context, s: &str, params: &mut T::Params) {
        let take = self.save_view.0;
        self.selected = self
            .find(take, true, |e| e.item.filter(cx, s, params))
            .unwrap_or(self.save_view.0);

        self.winline = (self.save_view.1 + self.selected).saturating_sub(self.save_view.0);
    }

    pub fn move_down(&mut self, rows: usize) {
        let len = self.items.len();
        if len > 0 {
            self.selected = std::cmp::min(self.selected + rows, len.saturating_sub(1));
            self.winline = std::cmp::min(self.selected, self.winline + rows);
        }
    }

    pub fn move_up(&mut self, rows: usize) {
        let len = self.items.len();
        if len > 0 {
            self.selected = self.selected.saturating_sub(rows);
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

    pub fn current(&self) -> &Elem<T> {
        &self.items[self.selected]
    }

    pub fn current_item(&self) -> &T {
        &self.items[self.selected].item
    }

    pub fn row(&self) -> usize {
        self.winline
    }

    pub fn remove_current(&mut self) -> T {
        let elem = self.items.remove(self.selected);
        self.selected = self.selected.saturating_sub(1);
        elem.item
    }

    pub fn replace_current(&mut self, item: T) {
        self.items[self.selected].item = item;
    }

    pub fn insert_current_level(&mut self, item: T) {
        let current = self.current();
        let level = current.level;
        let pos = match current.item.cmp(&item) {
            Ordering::Less => self
                .find(self.selected + 1, false, |e| {
                    e.level < level || (e.level == level && e.item.cmp(&item) != Ordering::Less)
                })
                .unwrap_or(self.items.len()),

            Ordering::Greater => {
                match self.find(self.selected, true, |elem| {
                    elem.level < level
                        || (elem.level == level && elem.item.cmp(&item) != Ordering::Greater)
                }) {
                    Some(p) if self.items[p].level == level => self.next_not_descendant_pos(p),
                    Some(p) => p + 1,
                    None => 0,
                }
            }
            Ordering::Equal => self.selected + 1,
        };
        self.items.insert(pos, Elem::new(item, level));
    }
}

impl<T: TreeItem> Tree<T> {
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
        self.winline = std::cmp::min(self.winline, area.height.saturating_sub(1) as usize);
        let style = cx.editor.theme.get(&self.tree_symbol_style);
        let last_item_index = self.items.len().saturating_sub(1);
        let skip = self.selected.saturating_sub(self.winline);
        let iter = self
            .items
            .iter()
            .skip(skip)
            .take(area.height as usize)
            .enumerate();
        for (index, elem) in iter {
            let row = index as u16;
            let mut area = Rect::new(area.x, area.y + row, area.width, 1);
            let indent = if elem.level > 0 {
                if index + skip != last_item_index {
                    format!("{}├─", "│ ".repeat(elem.level - 1))
                } else {
                    format!("└─{}", "┴─".repeat(elem.level - 1))
                }
            } else {
                "".to_string()
            };

            let indent_len = indent.chars().count();
            if indent_len > self.col {
                let indent: String = indent.chars().skip(self.col).collect();
                if !indent.is_empty() {
                    surface.set_stringn(area.x, area.y, &indent, area.width as usize, style);
                    area = area.clip_left(indent.width() as u16);
                }
            };
            let mut start_index = self.col.saturating_sub(indent_len);
            let mut text = elem.item.text(cx, skip + index == self.selected, params);
            self.max_len = self.max_len.max(text.width() + indent.len());
            for span in text.0.iter_mut() {
                if area.width == 0 {
                    return;
                }
                if start_index == 0 {
                    surface.set_span(area.x, area.y, span, area.width);
                    area = area.clip_left(span.width() as u16);
                } else {
                    let span_width = span.width();
                    if start_index > span_width {
                        start_index -= span_width;
                    } else {
                        let content: String = span
                            .content
                            .chars()
                            .filter(|c| {
                                if start_index > 0 {
                                    start_index = start_index.saturating_sub(c.to_string().width());
                                    false
                                } else {
                                    true
                                }
                            })
                            .collect();
                        surface.set_string_truncated(
                            area.x,
                            area.y,
                            &content,
                            area.width as usize,
                            |_| span.style,
                            false,
                            false,
                        );
                        start_index = 0
                    }
                }
            }
        }
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
            key!('z') => self.fold_current_level(),
            key!('h') => self.move_left(1.max(count)),
            key!('l') => self.move_right(1.max(count)),
            shift!('G') => self.move_down(usize::MAX / 2),
            key!(Enter) => self.on_enter(cx, params),
            ctrl!('d') => self.move_down_half_page(),
            ctrl!('u') => self.move_up_half_page(),
            shift!('D') => self.move_down_page(),
            shift!('U') => self.move_up_page(),
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

impl<T: TreeItem + Clone> Tree<T> {
    pub fn filter(&mut self, s: &str, cx: &mut Context, params: &mut T::Params) {
        fn filter_recursion<T>(
            elems: &Vec<Elem<T>>,
            mut index: usize,
            s: &str,
            cx: &mut Context,
            params: &mut T::Params,
        ) -> (Vec<Elem<T>>, usize)
        where
            T: TreeItem + Clone,
        {
            let mut retain = vec![];
            let elem = &elems[index];
            loop {
                let child = match elems.get(index + 1) {
                    Some(child) if child.item.is_child(&elem.item) => child,
                    _ => break,
                };
                index += 1;
                let next = elems.get(index + 1);
                if next.map_or(false, |n| n.item.is_child(&child.item)) {
                    let (sub_retain, current_index) = filter_recursion(elems, index, s, cx, params);
                    retain.extend(sub_retain);
                    index = current_index;
                } else if child.item.filter(cx, s, params) {
                    retain.push(child.clone());
                }
            }
            if !retain.is_empty() || elem.item.filter(cx, s, params) {
                retain.insert(0, elem.clone());
            }
            (retain, index)
        }

        if s.is_empty() {
            if let Some((_, recycle)) = self.recycle.take() {
                self.items = recycle;
                self.restore_view();
                return;
            }
        }

        let mut retain = vec![];
        let mut index = 0;
        let items = match &self.recycle {
            Some((pre, _)) if pre == s => return,
            Some((pre, recycle)) if pre.contains(s) => recycle,
            _ => &self.items,
        };
        while let Some(elem) = items.get(index) {
            let next = items.get(index + 1);
            if next.map_or(false, |n| n.item.is_child(&elem.item)) {
                let (sub_items, current_index) = filter_recursion(items, index, s, cx, params);
                index = current_index;
                retain.extend(sub_items);
            } else if elem.item.filter(cx, s, params) {
                retain.push(elem.clone())
            }
            index += 1;
        }

        if retain.is_empty() {
            if let Some((_, recycle)) = self.recycle.take() {
                self.items = recycle;
                self.restore_view();
            }
            return;
        }

        let recycle = std::mem::replace(&mut self.items, retain);
        if let Some(r) = self.recycle.as_mut() {
            r.0 = s.into()
        } else {
            self.recycle = Some((s.into(), recycle));
            self.save_view();
        }

        self.selected = self
            .find(0, false, |elem| elem.item.filter(cx, s, params))
            .unwrap_or(0);
        self.winline = self.selected;
    }

    pub fn clean_recycle(&mut self) {
        self.recycle = None;
    }

    pub fn restore_recycle(&mut self) {
        if let Some((_, recycle)) = self.recycle.take() {
            self.items = recycle;
        }
    }
}
