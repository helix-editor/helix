//! Ty implementations for helix-core types.
//!
//! This module contains the `Ty` trait implementations for types defined in helix-core.
//! By having these implementations here, we avoid helix-core needing to depend on helix-config.

use std::path::PathBuf;
use crate::{Map, Ty, Value};
use anyhow::bail;

use helix_core::auto_pairs::AutoPairs;
use helix_core::diagnostic::Severity;
use helix_core::indent::IndentStyle;
use helix_core::line_ending::{LineEnding, NATIVE_LINE_ENDING};
use helix_core::syntax::config::IndentationHeuristic;

// ============================================================================
// AutoPairs
// ============================================================================

impl Ty for AutoPairs {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        match val {
            Value::Bool(false) => Ok(Self::empty()),
            Value::Bool(true) => Ok(Self::default()),
            Value::Map(map) => {
                let pairs: Result<Vec<_>, _> = map
                    .iter()
                    .map(|(open, close)| {
                        let open = open.chars().next().ok_or_else(|| {
                            anyhow::anyhow!("expected single character key, got empty string")
                        })?;
                        let Value::String(close) = close else {
                            bail!("expected string value for auto pair close character");
                        };
                        let close = close.chars().next().ok_or_else(|| {
                            anyhow::anyhow!("expected single character value, got empty string")
                        })?;
                        Ok((open, close))
                    })
                    .collect();
                Ok(AutoPairs::new(pairs?.iter()))
            }
            _ => bail!("expected boolean or map of character pairs"),
        }
    }

    fn to_value(&self) -> Value {
        if self.is_empty() {
            return Value::Bool(false);
        }
        // Check if it matches the default pairs
        let default = Self::default();
        if self.len() == default.len() && self.iter().all(|(k, v)| default.get(k) == Some(v)) {
            return Value::Bool(true);
        }
        // Custom pairs - serialize as map
        let mut map: Map<Value> = Map::default();
        // Only include opener entries (avoid duplicating close entries)
        for (ch, pair) in self.iter() {
            if ch == pair.open {
                map.insert(
                    pair.open.to_string().into_boxed_str(),
                    Value::String(pair.close.to_string()),
                );
            }
        }
        Value::Map(Box::new(map))
    }
}

// ============================================================================
// LineEnding
// ============================================================================

impl Ty for LineEnding {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        let Value::String(s) = val else {
            bail!("expected a string for line ending");
        };
        match s.to_lowercase().as_str() {
            "native" => Ok(NATIVE_LINE_ENDING),
            "lf" | "unix" => Ok(LineEnding::LF),
            "crlf" | "dos" | "windows" => Ok(LineEnding::Crlf),
            #[cfg(feature = "unicode-lines")]
            "cr" => Ok(LineEnding::CR),
            #[cfg(feature = "unicode-lines")]
            "ff" => Ok(LineEnding::FF),
            #[cfg(feature = "unicode-lines")]
            "nel" => Ok(LineEnding::Nel),
            _ => bail!("unknown line ending: {s:?}"),
        }
    }

    fn to_value(&self) -> Value {
        let s = match self {
            LineEnding::LF => "lf",
            LineEnding::Crlf => "crlf",
            #[cfg(feature = "unicode-lines")]
            LineEnding::CR => "cr",
            #[cfg(feature = "unicode-lines")]
            LineEnding::VT => "vt",
            #[cfg(feature = "unicode-lines")]
            LineEnding::FF => "ff",
            #[cfg(feature = "unicode-lines")]
            LineEnding::Nel => "nel",
            #[cfg(feature = "unicode-lines")]
            LineEnding::LS => "ls",
            #[cfg(feature = "unicode-lines")]
            LineEnding::PS => "ps",
        };
        Value::String(s.to_string())
    }
}

// ============================================================================
// IndentStyle
// ============================================================================

impl Ty for IndentStyle {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        match val {
            Value::String(s) => {
                let s = s.to_lowercase();
                if s == "tabs" || s == "tab" {
                    Ok(IndentStyle::Tabs)
                } else if let Some(n) = s.strip_prefix("spaces:") {
                    let n: u8 = n.parse()?;
                    if n == 0 || n > helix_core::indent::MAX_INDENT {
                        bail!(
                            "spaces indent must be between 1 and {}",
                            helix_core::indent::MAX_INDENT
                        );
                    }
                    Ok(IndentStyle::Spaces(n))
                } else {
                    bail!("expected 'tabs' or 'spaces:N' (got {s:?})")
                }
            }
            Value::Int(n) => {
                if n <= 0 || n > helix_core::indent::MAX_INDENT as isize {
                    bail!(
                        "spaces indent must be between 1 and {}",
                        helix_core::indent::MAX_INDENT
                    );
                }
                Ok(IndentStyle::Spaces(n as u8))
            }
            _ => bail!("expected string ('tabs' or 'spaces:N') or integer"),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            IndentStyle::Tabs => Value::String("tabs".to_string()),
            IndentStyle::Spaces(n) => Value::Int(*n as isize),
        }
    }
}

// ============================================================================
// IndentationHeuristic
// ============================================================================

impl Ty for IndentationHeuristic {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        let val: String = val.typed()?;
        match val.as_str() {
            "simple" => Ok(IndentationHeuristic::Simple),
            "tree-sitter" => Ok(IndentationHeuristic::TreeSitter),
            "hybrid" => Ok(IndentationHeuristic::Hybrid),
            _ => bail!("invalid indentation heuristic: {val}"),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            IndentationHeuristic::Simple => "simple",
            IndentationHeuristic::TreeSitter => "tree-sitter",
            IndentationHeuristic::Hybrid => "hybrid",
        }
        .into()
    }
}

// ============================================================================
// Severity
// ============================================================================

impl Ty for Severity {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        let val: String = val.typed()?;
        match val.as_str() {
            "hint" => Ok(Severity::Hint),
            "info" => Ok(Severity::Info),
            "warning" => Ok(Severity::Warning),
            "error" => Ok(Severity::Error),
            _ => bail!("expected one of 'hint', 'info', 'warning' or 'error' (got {val:?})"),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Severity::Hint => "hint".into(),
            Severity::Info => "info".into(),
            Severity::Warning => "warning".into(),
            Severity::Error => "error".into(),
        }
    }
}
impl Ty for PathBuf {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        let val: String = val.typed()?;
        Ok(PathBuf::from(&*val))
    }
    fn to_value(&self) -> Value {
        Value::String(self.to_string_lossy().into_owned())
    }
}
