use std::ops::DerefMut;

use nucleo::pattern::{AtomKind, CaseMatching, Pattern};
use nucleo::Config;
use parking_lot::Mutex;

pub struct LazyMutex<T> {
    inner: Mutex<Option<T>>,
    init: fn() -> T,
}

impl<T> LazyMutex<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            inner: Mutex::new(None),
            init,
        }
    }

    pub fn lock(&self) -> impl DerefMut<Target = T> + '_ {
        parking_lot::MutexGuard::map(self.inner.lock(), |val| val.get_or_insert_with(self.init))
    }
}

pub static MATCHER: LazyMutex<nucleo::Matcher> = LazyMutex::new(nucleo::Matcher::default);

/// convenience function to easily fuzzy match
/// on a (relatively small list of inputs). This is not recommended for building a full tui
/// application that can match large numbers of matches as all matching is done on the current
/// thread, effectively blocking the UI
pub fn fuzzy_match<T: AsRef<str>>(
    pattern: &str,
    items: impl IntoIterator<Item = T>,
    path: bool,
) -> Vec<(T, u32)> {
    let mut matcher = MATCHER.lock();
    matcher.config = Config::DEFAULT;
    if path {
        matcher.config.set_match_paths();
    }
    let pattern = Pattern::new(pattern, CaseMatching::Smart, AtomKind::Fuzzy);
    pattern.match_list(items, &mut matcher)
}
