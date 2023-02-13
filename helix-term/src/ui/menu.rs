use std::{
    borrow::Cow,
    cmp::{self, Ordering},
    mem::swap,
    path::PathBuf,
    sync::Arc,
};

use crate::{
    compositor::{Callback, Component, Compositor, Context, Event, EventResult},
    ctrl, job, key, shift,
};
use futures_util::{future::BoxFuture, stream::FuturesUnordered, Future, FutureExt, StreamExt};

use helix_core::movement::Direction;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Notify,
};
use tui::{buffer::Buffer as Surface, widgets::Table};

pub use tui::widgets::{Cell, Row};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;

use helix_view::{graphics::Rect, Editor};
use tui::layout::Constraint;

pub trait Item: Send + 'static {
    /// Additional editor state that is used for label calculation.
    type Data: Send;

    fn format(&self, data: &Self::Data) -> Row;

    fn sort_text(&self, data: &Self::Data) -> Cow<str> {
        let label: String = self.format(data).cell_text().collect();
        label.into()
    }

    fn filter_text(&self, data: &Self::Data) -> Cow<str> {
        let label: String = self.format(data).cell_text().collect();
        label.into()
    }
}

impl Item for PathBuf {
    /// Root prefix to strip.
    type Data = PathBuf;

    fn format(&self, root_path: &Self::Data) -> Row {
        self.strip_prefix(root_path)
            .unwrap_or(self)
            .to_string_lossy()
            .into()
    }
}

pub type MenuCallback<T> = Box<dyn Fn(&mut Editor, Option<&T>, MenuEvent)>;

type AsyncData<T> = BoxFuture<'static, anyhow::Result<Vec<T>>>;
type AsyncRefetchWithQuery<T> = Box<dyn Fn(&str, &mut Editor) -> AsyncData<T> + Send>;

pub enum ItemSource<T: Item> {
    AsyncData(Option<AsyncData<T>>, <T as Item>::Data),
    // TODO maybe "generalize" this by using conditional functions
    // uses the current pattern/query to refetch new data
    AsyncRefetchOnIdleTimeoutWithPattern(AsyncRefetchWithQuery<T>, <T as Item>::Data),
    Data(Vec<T>, <T as Item>::Data),
}

impl<T: Item> ItemSource<T> {
    pub fn editor_data(&self) -> &<T as Item>::Data {
        match self {
            ItemSource::Data(_, editor_data) => editor_data,
            ItemSource::AsyncData(_, editor_data) => editor_data,
            ItemSource::AsyncRefetchOnIdleTimeoutWithPattern(_, editor_data) => editor_data,
        }
    }
    pub fn from_async_data(
        future: BoxFuture<'static, anyhow::Result<Vec<T>>>,
        editor_data: <T as Item>::Data,
    ) -> Self {
        Self::AsyncData(Some(future), editor_data)
    }

    pub fn from_data(data: Vec<T>, editor_data: <T as Item>::Data) -> Self {
        Self::Data(data, editor_data)
    }

    pub fn from_async_refetch_on_idle_timeout_with_pattern(
        fetch: AsyncRefetchWithQuery<T>,
        editor_data: <T as Item>::Data,
    ) -> Self {
        Self::AsyncRefetchOnIdleTimeoutWithPattern(fetch, editor_data)
    }
}

#[derive(PartialEq, Eq, Debug)]
struct Match {
    option_index: usize,
    score: i64,
    option_source: usize,
    len: usize,
}

impl Match {
    fn key(&self) -> impl Ord {
        (
            cmp::Reverse(self.score),
            self.len,
            self.option_source,
            self.option_index,
        )
    }
}

enum ItemSourceMessage<T> {
    Items {
        item_source_idx: usize,
        items: anyhow::Result<Vec<T>>,
    },
    NoFurtherItems,
}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Match {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

pub struct OptionsManager<T: Item> {
    options: Vec<Vec<T>>,
    options_receiver: UnboundedReceiver<ItemSourceMessage<T>>,
    options_sender: UnboundedSender<ItemSourceMessage<T>>,
    matches: Vec<Match>,
    matcher: Box<Matcher>,
    cursor: Option<usize>,
    item_sources: Vec<ItemSource<T>>,
    previous_pattern: (String, FuzzyQuery),
    last_pattern_on_idle_timeout: String,
    cursor_always_selects: bool,
    awaiting_async_options: bool,
    has_refetch_item_sources: bool,
}

// TODO Could be extended to a general error handling callback (e.g. errors while fetching)
pub type NoItemsAvailableCallback = Box<dyn FnOnce(&mut Editor) + Send + 'static>;

impl<T: Item> OptionsManager<T> {
    pub fn create_from_items(items: Vec<T>, editor_data: <T as Item>::Data) -> Self {
        let item_source = ItemSource::Data(vec![], editor_data);
        Self::create_with_item_sources(vec![item_source], [(0, items)], false)
    }

    fn create_with_item_sources<I>(
        item_sources: Vec<ItemSource<T>>,
        items: I,
        has_refetch_item_sources: bool,
    ) -> Self
    where
        I: IntoIterator<Item = (usize, Vec<T>)>,
    {
        // vec![vec![]; item_sources.len()] requires T: Clone
        let options = (0..item_sources.len()).map(|_| vec![]).collect();

        let (options_sender, options_receiver) = unbounded_channel();
        let mut options_manager = Self {
            item_sources,
            matches: vec![],
            matcher: Box::new(Matcher::default().ignore_case()),
            cursor: None,
            options,
            options_receiver,
            options_sender,
            previous_pattern: (String::new(), FuzzyQuery::default()),
            last_pattern_on_idle_timeout: String::new(),
            cursor_always_selects: false,
            awaiting_async_options: false,
            has_refetch_item_sources,
        };
        for (item_source_idx, items) in items {
            options_manager.options[item_source_idx] = items;
        }
        options_manager.force_score();
        options_manager
    }

    fn create_options_manager_async<C>(
        mut requests: FuturesUnordered<
            impl Future<Output = (usize, anyhow::Result<Vec<T>>)> + Send + 'static,
        >,
        item_sources: Vec<ItemSource<T>>,
        has_refetch_item_sources: bool,
        create_options_container: C,
        no_items_available: Option<NoItemsAvailableCallback>,
    ) -> BoxFuture<'static, anyhow::Result<job::Callback>>
    where
        C: FnOnce(&mut Editor, &mut Compositor, OptionsManager<T>) + Send + 'static,
    {
        async move {
            let request = requests.next().await;
            let call =
                job::Callback::EditorCompositorJobs(Box::new(move |editor, compositor, jobs| {
                    let (item_source_idx, items) = match request {
                        Some(r) => r,
                        None => {
                            return if let Some(no_items_available) = no_items_available {
                                no_items_available(editor)
                            }
                        }
                    };
                    let items = items.unwrap_or_default(); // TODO show error somewhere instead of swalloing it here?
                    if items.is_empty() {
                        // items are empty, try the next item source
                        jobs.callback(Self::create_options_manager_async(
                            requests,
                            item_sources,
                            has_refetch_item_sources,
                            create_options_container,
                            no_items_available,
                        ));
                    } else {
                        let mut option_manager = Self::create_with_item_sources(
                            item_sources,
                            [(item_source_idx, items)],
                            has_refetch_item_sources,
                        );
                        let options_sender = option_manager.options_sender.clone();
                        if !requests.is_empty() {
                            option_manager.awaiting_async_options = true;
                            jobs.spawn(Self::extend_options_manager_async(
                                requests,
                                options_sender,
                                editor.redraw_handle.0.clone(),
                            ));
                        }
                        // callback that adds ui like menu with the options_manager as argument
                        create_options_container(editor, compositor, option_manager);
                    }
                }));
            Ok(call)
        }
        .boxed()
    }

    fn extend_options_manager_async(
        mut requests: FuturesUnordered<
            impl Future<Output = (usize, anyhow::Result<Vec<T>>)> + Send + 'static,
        >,
        options_sender: UnboundedSender<ItemSourceMessage<T>>,
        redraw_notify: Arc<Notify>,
    ) -> BoxFuture<'static, anyhow::Result<()>> {
        async move {
            while let Some((item_source_idx, items)) = requests.next().await {
                // ignore error, as it just indicates that the options manager is gone (i.e. closed), so just discard this future
                if options_sender
                    .send(ItemSourceMessage::Items {
                        item_source_idx,
                        items,
                    })
                    .is_err()
                {
                    return Ok(());
                };
                redraw_notify.notify_one();
            }
            let _ = options_sender.send(ItemSourceMessage::NoFurtherItems);
            Ok(())
        }
        .boxed()
    }

    pub fn create_from_item_sources<F>(
        mut item_sources: Vec<ItemSource<T>>,
        editor: &mut Editor,
        jobs: &job::Jobs,
        create_options_container: F,
        no_items_available: Option<NoItemsAvailableCallback>, // It's a dynamic dispatch to avoid explicit typing with 'None' for the callbacks
    ) where
        F: FnOnce(&mut Editor, &mut Compositor, OptionsManager<T>) + Send + 'static,
    {
        let async_requests: FuturesUnordered<_> = item_sources
            .iter_mut()
            .enumerate()
            .filter_map(|(idx, item_source)| match item_source {
                ItemSource::AsyncData(data, _) => data
                    .take()
                    .map(|data| async move { (idx, data.await) }.boxed()),
                ItemSource::AsyncRefetchOnIdleTimeoutWithPattern(fetch, _) => {
                    let future = fetch("", editor);
                    Some(async move { (idx, future.await) }.boxed())
                }
                _ => None,
            })
            .collect();
        let sync_items: Vec<_> = item_sources
            .iter_mut()
            .enumerate()
            .filter_map(|(idx, item_source)| match item_source {
                ItemSource::Data(data, _) if !data.is_empty() => {
                    let mut new_data = vec![];
                    swap(data, &mut new_data);
                    Some((idx, new_data))
                }
                _ => None,
            })
            .collect();
        let has_refetch_item_sources = item_sources.iter().any(|item_source| {
            matches!(
                item_source,
                ItemSource::AsyncRefetchOnIdleTimeoutWithPattern(_, _)
            )
        });

        // no items available
        if async_requests.is_empty() && sync_items.is_empty() {
            if let Some(no_items_available) = no_items_available {
                no_items_available(editor);
            }
            return;
        }

        if !sync_items.is_empty() {
            // TODO this could be done in sync, but it needs the compositor in scope
            jobs.callback(async move {
                Ok(job::Callback::EditorCompositorJobs(Box::new(
                    move |editor, compositor, jobs| {
                        let mut option_manager = Self::create_with_item_sources(
                            item_sources,
                            sync_items,
                            has_refetch_item_sources,
                        );
                        let option_sender = option_manager.options_sender.clone();
                        if !async_requests.is_empty() {
                            option_manager.awaiting_async_options = true;
                            jobs.spawn(Self::extend_options_manager_async(
                                async_requests,
                                option_sender,
                                editor.redraw_handle.0.clone(),
                            ))
                        }
                        create_options_container(editor, compositor, option_manager);
                    },
                )))
            });
        } else {
            jobs.callback(Self::create_options_manager_async(
                async_requests,
                item_sources,
                has_refetch_item_sources,
                create_options_container,
                no_items_available,
            ));
        }
    }

    pub fn refetch_on_idle_timeout(&mut self, editor: &mut Editor, jobs: &job::Jobs) -> bool {
        if !self.has_refetch_item_sources
            || (self.last_pattern_on_idle_timeout == self.previous_pattern.0.clone())
        {
            return false;
        }

        let requests: FuturesUnordered<_> = self
            .item_sources
            .iter()
            .enumerate()
            .filter_map(|(idx, item_source)| match item_source {
                ItemSource::AsyncRefetchOnIdleTimeoutWithPattern(fetch, _) => {
                    let future = fetch(&self.previous_pattern.0, editor);
                    Some(async move { (idx, future.await) }.boxed())
                }
                _ => None,
            })
            .collect();

        if !requests.is_empty() {
            self.last_pattern_on_idle_timeout = self.previous_pattern.0.clone();
            self.awaiting_async_options = true;
            jobs.spawn(Self::extend_options_manager_async(
                requests,
                self.options_sender.clone(),
                editor.redraw_handle.0.clone(),
            ));
            return true;
        }
        false
    }

    pub fn poll_for_new_options(&mut self) -> bool {
        if !self.awaiting_async_options {
            return false;
        }
        let mut new_options_added = false;
        // TODO handle errors somehow?
        while let Ok(message) = self.options_receiver.try_recv() {
            match message {
                ItemSourceMessage::Items {
                    item_source_idx,
                    items: Ok(items),
                } => {
                    if items.is_empty() && self.options[item_source_idx].is_empty() {
                        continue;
                    }
                    new_options_added = true;
                    // TODO this could be extended by getting the matched option and try to find it in the new options
                    let cursor_on_old_option = matches!(self.cursor.and_then(|cursor| self.matches.get(cursor)),
                                                        Some(Match { option_source, ..}) if *option_source == item_source_idx);
                    if cursor_on_old_option {
                        self.cursor = if self.cursor_always_selects {
                            Some(0)
                        } else {
                            None
                        };
                    }
                    self.options[item_source_idx] = items;
                }
                ItemSourceMessage::NoFurtherItems => self.awaiting_async_options = false,
                _ => (), // TODO handle error somehow?
            }
        }
        if new_options_added {
            self.force_score();
        }
        new_options_added
    }

    pub fn options(&self) -> impl Iterator<Item = (&T, &T::Data)> {
        self.options
            .iter()
            .enumerate()
            .flat_map(move |(idx, options)| {
                options
                    .iter()
                    .map(move |o| (o, self.item_sources[idx].editor_data()))
            })
    }

    pub fn options_len(&self) -> usize {
        self.options.iter().map(Vec::len).sum()
    }

    pub fn matches(&self) -> impl Iterator<Item = (&T, &T::Data)> {
        self.matches.iter().map(
            |Match {
                 option_index,
                 option_source,
                 ..
             }| {
                (
                    &self.options[*option_source][*option_index],
                    self.item_sources[*option_source].editor_data(),
                )
            },
        )
    }

    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    // TODO should probably be an enum
    pub fn set_cursor_selection_mode(&mut self, cursor_always_selects: bool) {
        self.cursor_always_selects = cursor_always_selects;
        if cursor_always_selects && self.cursor.is_none() && !self.matches.is_empty() {
            self.cursor = Some(0);
        }
    }

    // if pattern is None, use the previously used last pattern
    pub fn score(&mut self, pattern: Option<&str>, reset_cursor: bool, force_recalculation: bool) {
        if reset_cursor && self.cursor.is_some() {
            self.cursor = if self.cursor_always_selects {
                Some(0)
            } else {
                None
            };
        }

        let pattern = match pattern {
            Some(pattern) if pattern == self.previous_pattern.0 && !force_recalculation => return,
            None if !force_recalculation => return,
            None => &self.previous_pattern.0,
            Some(pattern) => pattern,
        };
        let prev_selected_option = if !reset_cursor {
            self.cursor.and_then(|c| {
                self.matches.get(c).map(
                    |Match {
                         option_source,
                         option_index,
                         ..
                     }| (*option_source, *option_index),
                )
            })
        } else {
            None
        };

        let (query, is_refined) = self
            .previous_pattern
            .1
            .refine(pattern, &self.previous_pattern.0);

        if pattern.is_empty() {
            // Fast path for no pattern.
            self.matches.clear();
            self.matches
                .extend(self.item_sources.iter().enumerate().flat_map(
                    |(option_source, item_source)| {
                        self.options[option_source].iter().enumerate().map(
                            move |(option_index, option)| {
                                let text = option.filter_text(item_source.editor_data());
                                Match {
                                    option_index,
                                    option_source,
                                    score: 0,
                                    len: text.chars().count(),
                                }
                            },
                        )
                    },
                ));
        } else if is_refined && !force_recalculation {
            // optimization: if the pattern is a more specific version of the previous one
            // then we can score the filtered set.
            self.matches.retain_mut(|omatch| {
                let option = &self.options[omatch.option_source][omatch.option_index];
                let text = option.sort_text(self.item_sources[omatch.option_source].editor_data());

                match query.fuzzy_match(&text, &self.matcher) {
                    Some(s) => {
                        // Update the score
                        omatch.score = s;
                        true
                    }
                    None => false,
                }
            });

            self.matches.sort();
        } else {
            self.matches.clear();
            let matcher = &self.matcher;
            let query = &query;
            self.matches
                .extend(self.item_sources.iter().enumerate().flat_map(
                    |(option_source, item_source)| {
                        self.options[option_source].iter().enumerate().filter_map(
                            move |(option_index, option)| {
                                let text = option.filter_text(item_source.editor_data());
                                query.fuzzy_match(&text, matcher).map(|score| Match {
                                    option_index,
                                    option_source,
                                    score,
                                    len: text.chars().count(),
                                })
                            },
                        )
                    },
                ));

            self.matches.sort();
        }

        // reset cursor position or recover position based on previous matched option
        if !reset_cursor {
            self.cursor = self
                .matches
                .iter()
                .enumerate()
                .find_map(|(index, m)| {
                    if Some((m.option_source, m.option_index)) == prev_selected_option {
                        Some(index)
                    } else {
                        None
                    }
                })
                .or(if self.cursor_always_selects {
                    Some(0)
                } else {
                    None
                });
        };
        if self.previous_pattern.0 != pattern {
            self.previous_pattern.0 = pattern.to_owned();
        }
        self.previous_pattern.1 = query;
    }

    pub fn force_score(&mut self) {
        self.score(None, false, true)
    }

    pub fn clear(&mut self) {
        self.matches.clear();

        // reset cursor position
        self.cursor = None;
    }

    /// Move the cursor by a number of lines, either down (`Forward`) or up (`Backward`)
    pub fn move_cursor_by(&mut self, amount: usize, direction: Direction) {
        let len = self.matches.len();

        if len == 0 {
            // No results, can't move.
            return;
        }

        if amount != 0 {
            self.cursor = Some(match (direction, self.cursor) {
                (Direction::Forward, Some(cursor)) => cursor.saturating_add(amount) % len,
                (Direction::Backward, Some(cursor)) => {
                    cursor.saturating_add(len).saturating_sub(amount) % len
                }
                (Direction::Forward, None) => amount - 1,
                (Direction::Backward, None) => len.saturating_sub(amount),
            });
        }
    }

    /// Move the cursor to the first entry
    pub fn to_start(&mut self) {
        self.cursor = Some(0);
    }

    /// Move the cursor to the last entry
    pub fn to_end(&mut self) {
        self.cursor = Some(self.matches.len().saturating_sub(1));
    }

    pub fn selection(&self) -> Option<&T> {
        self.cursor.and_then(|cursor| {
            self.matches.get(cursor).map(
                |Match {
                     option_index,
                     option_source,
                     ..
                 }| &self.options[*option_source][*option_index],
            )
        })
    }

    pub fn selection_mut(&mut self) -> Option<&mut T> {
        self.cursor.and_then(|cursor| {
            self.matches.get(cursor).map(
                |Match {
                     option_index,
                     option_source,
                     ..
                 }| &mut self.options[*option_source][*option_index],
            )
        })
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn matches_len(&self) -> usize {
        self.matches.len()
    }

    pub fn matcher(&self) -> &Matcher {
        &self.matcher
    }
}

impl<T: Item + PartialEq> OptionsManager<T> {
    fn replace_option(&mut self, old_option: T, new_option: T) {
        for options in &mut self.options {
            for option in options {
                if old_option == *option {
                    *option = new_option;
                    return;
                }
            }
        }
    }
}

pub struct Menu<T: Item> {
    options_manager: OptionsManager<T>,
    widths: Vec<Constraint>,

    callback_fn: MenuCallback<T>,

    scroll: usize,
    size: (u16, u16),
    viewport: (u16, u16),
    recalculate: bool,
}

use super::{fuzzy_match::FuzzyQuery, PromptEvent as MenuEvent};

impl<T: Item> Menu<T> {
    const LEFT_PADDING: usize = 1;

    pub fn new(
        options_manager: OptionsManager<T>,
        callback_fn: impl Fn(&mut Editor, Option<&T>, MenuEvent) + 'static,
    ) -> Self {
        Self {
            options_manager,
            widths: Vec::new(),
            callback_fn: Box::new(callback_fn),
            scroll: 0,
            size: (0, 0),
            viewport: (0, 0),
            recalculate: true,
        }
    }

    pub fn score(&mut self, pattern: &str) {
        // TODO reset cursor?
        self.options_manager.score(Some(pattern), false, false);
        self.scroll = 0;
        self.recalculate = true;
    }

    pub fn clear(&mut self) {
        self.options_manager.clear();
        self.scroll = 0;
    }

    pub fn move_up(&mut self) {
        self.options_manager.move_cursor_by(1, Direction::Backward);
        self.adjust_scroll();
    }

    pub fn move_down(&mut self) {
        self.options_manager.move_cursor_by(1, Direction::Forward);
        self.adjust_scroll();
    }

    fn recalculate_size(&mut self, viewport: (u16, u16)) {
        let n = self
            .options_manager
            .options()
            .next()
            .map(|(option, editor_data)| option.format(editor_data).cells.len())
            .unwrap_or_default();
        let max_lens =
            self.options_manager
                .options()
                .fold(vec![0; n], |mut acc, (option, editor_data)| {
                    let row = option.format(editor_data);
                    // maintain max for each column
                    for (acc, cell) in acc.iter_mut().zip(row.cells.iter()) {
                        let width = cell.content.width();
                        if width > *acc {
                            *acc = width;
                        }
                    }

                    acc
                });

        let height = self.len().min(10).min(viewport.1 as usize);
        // do all the matches fit on a single screen?
        let fits = self.len() <= height;

        let mut len = max_lens.iter().sum::<usize>() + n;

        if !fits {
            len += 1; // +1: reserve some space for scrollbar
        }

        len += Self::LEFT_PADDING;
        let width = len.min(viewport.0 as usize);

        self.widths = max_lens
            .into_iter()
            .map(|len| Constraint::Length(len as u16))
            .collect();

        self.size = (width as u16, height as u16);

        // adjust scroll offsets if size changed
        self.adjust_scroll();
        self.recalculate = false;
    }

    fn adjust_scroll(&mut self) {
        let win_height = self.size.1 as usize;
        if let Some(cursor) = self.options_manager.cursor() {
            let mut scroll = self.scroll;
            if cursor > (win_height + scroll).saturating_sub(1) {
                // scroll down
                scroll += cursor - (win_height + scroll).saturating_sub(1)
            } else if cursor < scroll {
                // scroll up
                scroll = cursor
            }
            self.scroll = scroll;
        }
    }

    pub fn selection(&self) -> Option<&T> {
        self.options_manager.selection()
    }

    pub fn selection_mut(&mut self) -> Option<&mut T> {
        self.options_manager.selection_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.options_manager.is_empty()
    }

    pub fn len(&self) -> usize {
        self.options_manager.matches_len()
    }
}

impl<T: Item> Component for Menu<T> {
    fn handle_event(&mut self, event: &Event, cx: &mut Context) -> EventResult {
        let event = match event {
            Event::Key(event) => *event,
            Event::IdleTimeout => {
                self.options_manager
                    .refetch_on_idle_timeout(cx.editor, cx.jobs);
                return EventResult::Consumed(None);
            }
            _ => return EventResult::Ignored(None),
        };

        let close_fn: Option<Callback> = Some(Box::new(|compositor: &mut Compositor, _| {
            // remove the layer
            compositor.pop();
        }));

        match event {
            // esc or ctrl-c aborts the completion and closes the menu
            key!(Esc) | ctrl!('c') => {
                (self.callback_fn)(cx.editor, self.selection(), MenuEvent::Abort);
                return EventResult::Consumed(close_fn);
            }
            // arrow up/ctrl-p/shift-tab prev completion choice (including updating the doc)
            shift!(Tab) | key!(Up) | ctrl!('p') => {
                self.move_up();
                (self.callback_fn)(cx.editor, self.selection(), MenuEvent::Update);
                return EventResult::Consumed(None);
            }
            key!(Tab) | key!(Down) | ctrl!('n') => {
                // arrow down/ctrl-n/tab advances completion choice (including updating the doc)
                self.move_down();
                (self.callback_fn)(cx.editor, self.selection(), MenuEvent::Update);
                return EventResult::Consumed(None);
            }
            key!(Enter) => {
                if let Some(selection) = self.selection() {
                    (self.callback_fn)(cx.editor, Some(selection), MenuEvent::Validate);
                    return EventResult::Consumed(close_fn);
                } else {
                    return EventResult::Ignored(close_fn);
                }
            }
            // KeyEvent {
            //     code: KeyCode::Char(c),
            //     modifiers: KeyModifiers::NONE,
            // } => {
            //     self.insert_char(c);
            //     (self.callback_fn)(cx.editor, &self.line, MenuEvent::Update);
            // }

            // / -> edit_filter?
            //
            // enter confirms the match and closes the menu
            // typing filters the menu
            // if we run out of options the menu closes itself
            _ => (),
        }
        // for some events, we want to process them but send ignore, specifically all input except
        // tab/enter/ctrl-k or whatever will confirm the selection/ ctrl-n/ctrl-p for scroll.
        // EventResult::Consumed(None)
        EventResult::Ignored(None)
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        self.recalculate |= self.options_manager.poll_for_new_options();
        if viewport != self.viewport || self.recalculate {
            self.recalculate_size(viewport);
        }

        Some(self.size)
    }

    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        self.options_manager.poll_for_new_options();
        let theme = &cx.editor.theme;
        let style = theme
            .try_get("ui.menu")
            .unwrap_or_else(|| theme.get("ui.text"));
        let selected = theme.get("ui.menu.selected");
        surface.clear_with(area, style);

        let scroll = self.scroll;

        let options: Vec<_> = self.options_manager.matches().collect();

        let len = options.len();

        let win_height = area.height as usize;

        const fn div_ceil(a: usize, b: usize) -> usize {
            (a + b - 1) / b
        }

        let rows = options
            .iter()
            .map(|(option, editor_data)| option.format(editor_data));
        let table = Table::new(rows)
            .style(style)
            .highlight_style(selected)
            .column_spacing(1)
            .widths(&self.widths);

        use tui::widgets::TableState;

        table.render_table(
            area.clip_left(Self::LEFT_PADDING as u16).clip_right(1),
            surface,
            &mut TableState {
                offset: scroll,
                selected: self.options_manager.cursor(),
            },
        );

        if let Some(cursor) = self.options_manager.cursor() {
            let offset_from_top = cursor - scroll;
            let left = &mut surface[(area.left(), area.y + offset_from_top as u16)];
            left.set_style(selected);
            let right = &mut surface[(
                area.right().saturating_sub(1),
                area.y + offset_from_top as u16,
            )];
            right.set_style(selected);
        }

        let fits = len <= win_height;

        let scroll_style = theme.get("ui.menu.scroll");
        if !fits {
            let scroll_height = div_ceil(win_height.pow(2), len).min(win_height);
            let scroll_line = (win_height - scroll_height) * scroll
                / std::cmp::max(1, len.saturating_sub(win_height));

            let mut cell;
            for i in 0..win_height {
                cell = &mut surface[(area.right() - 1, area.top() + i as u16)];

                cell.set_symbol("▐"); // right half block

                if scroll_line <= i && i < scroll_line + scroll_height {
                    // Draw scroll thumb
                    cell.set_fg(scroll_style.fg.unwrap_or(helix_view::theme::Color::Reset));
                } else {
                    // Draw scroll track
                    cell.set_fg(scroll_style.bg.unwrap_or(helix_view::theme::Color::Reset));
                }
            }
        }
    }
}

impl<T: Item + PartialEq> Menu<T> {
    pub fn replace_option(&mut self, old_option: T, new_option: T) {
        self.options_manager.replace_option(old_option, new_option);
    }
}
