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
    (@trie $cmd:ident) => {
        $crate::keymap::KeyTrie::Leaf($crate::commands::MappableCommand::$cmd)
    };

    (@trie
        { $label:literal $(sticky=$sticky:literal)? $($($key:literal)|+ => $value:tt,)+ }
    ) => {
        keymap!({ $label $(sticky=$sticky)? $($($key)|+ => $value,)+ })
    };

    (@trie [$($cmd:ident),* $(,)?]) => {
        $crate::keymap::KeyTrie::Sequence(vec![$($crate::commands::Command::$cmd),*])
    };

    (
        { $label:literal $(sticky=$sticky:literal)? $($($key:literal)|+ => $value:tt,)+ }
    ) => {
        // modified from the hashmap! macro
        {
            let _cap = hashmap!(@count $($($key),+),*);
            let mut _map = ::std::collections::HashMap::with_capacity(_cap);
            let mut _order = ::std::vec::Vec::with_capacity(_cap);
            $(
                $(
                    let _key = $key.parse::<$crate::input::KeyEvent>().unwrap();
                    let _duplicate = _map.insert(
                        _key,
                        keymap!(@trie $value)
                    );
                    assert!(_duplicate.is_none(), "Duplicate key found: {:?}", _duplicate.unwrap());
                    _order.push(_key);
                )+
            )*
            let mut _node = $crate::keymap::KeyTrieNode::new($label, _map, _order);
            $( _node.is_sticky = $sticky; )?
            $crate::keymap::KeyTrie::Node(_node)
        }
    };
}

pub use crate::{alt, ctrl, key, shift};
pub use keymap;
