#[macro_export]
macro_rules! key {
    ($key_event:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key_event,
            modifiers: ::helix_view::keyboard::KeyModifiers::NONE,
        }
    };
    ($($ch:tt)*) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::NONE,
        }
    };
}

#[macro_export]
macro_rules! shift {
    ($key_event:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key_event,
            modifiers: ::helix_view::keyboard::KeyModifiers::SHIFT,
        }
    };
    ($($ch:tt)*) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::SHIFT,
        }
    };
}

#[macro_export]
macro_rules! ctrl {
    ($key_event:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key_event,
            modifiers: ::helix_view::keyboard::KeyModifiers::CONTROL,
        }
    };
    ($($ch:tt)*) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::CONTROL,
        }
    };
}

#[macro_export]
macro_rules! alt {
    ($key_event:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key_event,
            modifiers: ::helix_view::keyboard::KeyModifiers::ALT,
        }
    };
    ($($ch:tt)*) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::Char($($ch)*),
            modifiers: ::helix_view::keyboard::KeyModifiers::ALT,
        }
    };
}

/// Macro for defining the root of a `KeyTrie` object. Example:
///
/// ```
/// # use helix_core::hashmap;
/// # use helix_term::keymap::{keytrie::KeyTrie, macros::keytrie};
/// let normal_mode = keytrie!({ "Normal mode"
///     "i" => insert_mode,
///     "g" => { "Goto"
///         "g" => goto_file_start,
///         "e" => goto_file_end,
///     },
///     "j" | "down" => move_line_down,
/// });
/// let keymap = normal_mode;
/// ```
#[macro_export]
macro_rules! keytrie {
    // Sub key_trie
    ({ $label:literal $(sticky=$sticky:literal)? $($($key_event:literal)|+ => $value:tt,)+ }) => {
        {
            let _cap = hashmap!(@count $($($key_event),+),*);
            let mut _children: Vec<$crate::keymap::keytrienode::KeyTrieNode> = ::std::vec::Vec::new();
            let mut _child_order: ::std::collections::HashMap<::helix_view::input::KeyEvent, usize> = ::std::collections::HashMap::with_capacity(_cap);
            $(
                $(
                    let _key_event = $key_event.parse::<::helix_view::input::KeyEvent>().unwrap();
                    let _potential_duplicate = _child_order.insert(_key_event, _children.len());
                    assert!(_potential_duplicate.is_none(), "Duplicate key found: {:?}", _potential_duplicate.unwrap());
                    _children.push(keytrie!(@trie $value));
                )+
            )*

            let mut _node = $crate::keymap::keytrie::KeyTrie::new($label, _child_order, _children);
            $( _node.is_sticky = $sticky; )?
            _node
        }
    };

    (@trie {$label:literal $(sticky=$sticky:literal)? $($($key_event:literal)|+ => $value:tt,)+ }) => {
        $crate::keymap::keytrienode::KeyTrieNode::KeyTrie(keytrie!({ $label $(sticky=$sticky)? $($($key_event)|+ => $value,)+ }))
    };

    (@trie $cmd:ident) => {
        $crate::keymap::keytrienode::KeyTrieNode::MappableCommand($crate::commands::MappableCommand::$cmd)
    };

    (@trie [$($cmd:ident),* $(,)?]) => {
        $crate::keymap::keytrienode::KeyTrieNode::CommandSequence(vec![$($crate::commands::Command::$cmd),*])
    };
}

pub use alt;
pub use ctrl;
pub use key;
pub use keytrie;
pub use shift;
