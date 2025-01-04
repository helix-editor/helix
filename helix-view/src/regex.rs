use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    cmp::{Eq, PartialEq},
    ops::Deref,
};

/// Wrapper type for regex::Regex that only exists so we can implement Eq on it, as that's needed
/// to put it in editor::Config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EqRegex {
    #[serde(with = "serde_regex")]
    inner: Regex,
}

impl From<Regex> for EqRegex {
    fn from(value: Regex) -> Self {
        EqRegex { inner: value }
    }
}

impl Deref for EqRegex {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl PartialEq for EqRegex {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for EqRegex {}
