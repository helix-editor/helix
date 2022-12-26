#[macro_export]
macro_rules! key {
    ($key:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
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
    ($key:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
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
    ($key:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
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
    ($key:ident) => {
        ::helix_view::input::KeyEvent {
            code: ::helix_view::keyboard::KeyCode::$key,
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

/// Macro for defining the root of a `Keymap` object. Example:
///
/// ```
/// # use helix_core::hashmap;
/// # use helix_term::keymap;
/// # use helix_term::keymap::Keymap;
/// let normal_mode = keymap!({ "Normal mode"
///     "i" => insert_mode,
///     "g" => { "Goto"
///         "g" => goto_file_start,
///         "e" => goto_file_end,
///     },
///     "j" | "down" => move_line_down,
/// });
/// let keymap = Keymap::new(normal_mode);
/// ```
#[macro_export]
macro_rules! keymap {
    ({ $label:literal $(sticky=$sticky:literal)? $($($key:literal)|+ => $value:tt,)+ }) => {
        // modified from the hashmap! macro
        {
            let _cap = hashmap!(@count $($($key),+),*);
            let mut _map: ::std::collections::HashMap<::helix_view::input::KeyEvent, $crate::keymap::keytrienode::KeyTrieNode> = 
                ::std::collections::HashMap::with_capacity(_cap);
            $(
                $(
                    let _key = $key.parse::<::helix_view::input::KeyEvent>().unwrap();
                    let _potential_duplicate = _map.insert(_key,keymap!(@trie $value));
                    assert!(_potential_duplicate.is_none(), "Duplicate key found: {:?}", _potential_duplicate.unwrap());
                )+
            )*
            let mut _node = $crate::keymap::keytrie::KeyTrie::new($label, _map);
            $( _node.is_sticky = $sticky; )?
            _node
        }
    };

    (@trie {$label:literal $(sticky=$sticky:literal)? $($($key:literal)|+ => $value:tt,)+ }) => {
        $crate::keymap::keytrienode::KeyTrieNode::KeyTrie(keymap!({ $label $(sticky=$sticky)? $($($key)|+ => $value,)+ }))
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
pub use keymap;
pub use shift;
