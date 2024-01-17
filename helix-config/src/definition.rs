use std::time::Duration;

use crate::*;

mod language;
mod lsp;
mod ui;

pub use lsp::init_language_server_config;

options! {
    use ui::*;
    use lsp::*;
    use language::*;

    struct WrapConfig {
        /// Soft wrap lines that exceed viewport width.
        enable: bool = false,
        /// Maximum free space left at the end of the line.
        /// Automatically limited to a quarter of the viewport.
        max_wrap: u16 = 20,
        /// Maximum indentation to carry over when soft wrapping a line.
        /// Automatically limited to a quarter of the viewport.
        max_indent_retain: u16 = 40,
        /// Text inserted before soft wrapped lines, highlighted with `ui.virtual.wrap`.
        wrap_indicator: String = "↪",
        /// Soft wrap at `text-width` instead of using the full viewport size.
        wrap_at_text_width: bool = false,
        /// Maximum line length. Used for the `:reflow` command and
        /// soft-wrapping if `soft-wrap.wrap-at-text-width` is set
        text_width: usize = 80,
    }

    struct MouseConfig {
        /// Enable mouse mode
        #[read = copy]
        mouse: bool = true,
        /// Number of lines to scroll per scroll wheel step.
        #[read = copy]
        scroll_lines: usize = 3,
        ///  Middle click paste support
        #[read = copy]
        middle_click_paste: bool = true,
    }
    struct SmartTabConfig {
        /// If set to true, then when the cursor is in a position with
        /// non-whitespace to its left, instead of inserting a tab, it will run
        /// `move_parent_node_end`. If there is only whitespace to the left,
        /// then it inserts a tab as normal. With the default bindings, to
        /// explicitly insert a tab character, press Shift-tab.
        #[name = "smart-tab.enable"]
        #[read = copy]
        enable: bool = true,
        /// Normally, when a menu is on screen, such as when auto complete
        /// is triggered, the tab key is bound to cycling through the items.
        /// This means when menus are on screen, one cannot use the tab key
        /// to trigger the `smart-tab` command. If this option is set to true,
        /// the `smart-tab` command always takes precedence, which means one
        /// cannot use the tab key to cycle through menu items. One of the other
        /// bindings must be used instead, such as arrow keys or `C-n`/`C-p`.
        #[name = "smart-tab.supersede-menu"]
        #[read = copy]
        supersede_menu: bool = false,
    }

    struct SearchConfig {
        /// Enable smart case regex searching (case-insensitive unless pattern
        /// contains upper case characters)
        #[name = "search.smart-case"]
        #[read = copy]
        smart_case: bool = true,
        /// Whether the search should wrap after depleting the matches
        #[name = "search.wrap-round"]
        #[read = copy]
        wrap_round: bool = true,
    }

    struct MiscConfig {
        /// Number of lines of padding around the edge of the screen when scrolling.
        #[read = copy]
        scrolloff: usize = 5,
        /// Shell to use when running external commands
        #[read = deref]
        shell: List<String> = if cfg!(windows) {
             &["cmd", "/C"]
        } else {
            &["sh", "-c"]
        },
        /// Enable automatic saving on the focus moving away from Helix.
        /// Requires [focus event support](https://github.com/helix-editor/
        /// helix/wiki/Terminal-Support) from your terminal
        #[read = copy]
        auto_save: bool = false,
        /// Whether to automatically insert a trailing line-ending on write
        /// if missing
        #[read = copy]
        insert_final_newline: bool = true,
        /// Time in milliseconds since last keypress before idle timers trigger.
        /// Used for autocompletion, set to 0 for instant
        #[read = copy]
        idle_timeout: Duration = Duration::from_millis(250),
    }
}

impl Ty for Duration {
    fn from_value(val: Value) -> anyhow::Result<Self> {
        let val: usize = val.typed()?;
        Ok(Duration::from_millis(val as _))
    }
    fn to_value(&self) -> Value {
        Value::Int(self.as_millis().try_into().unwrap())
    }
}
