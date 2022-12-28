#[macro_use]
#[cfg(test)]
mod tests {
    use helix_core::hashmap;
    use helix_view::{document::Mode, input::KeyEvent};
    use crate::{
        keymap::macros::*,
        keymap::keymaps::Keymaps,
        keymap::Keymap,
    };
    use std::collections::HashMap;

    #[test]
    #[should_panic]
    fn duplicate_keys_should_panic() {
        keymap!({ "Normal mode"
            "i" => normal_mode,
            "i" => goto_definition,
        });
    }

    #[test]
    fn check_duplicate_keys_in_default_keymap() {
        // will panic on duplicate keys, assumes that `Keymaps` uses keymap! macro
        Keymaps::default();
    }

    #[test]
    fn aliased_modes_are_same_in_default_keymap() {
        let keymaps = Keymaps::default().keymaps;
        let root = keymaps.load().get(&Mode::Normal).unwrap().root_node.clone();
        assert_eq!(
            root.traverse(&[key!(' '), key!('w')]).unwrap(),
            root.traverse(&["C-w".parse::<KeyEvent>().unwrap()]).unwrap(),
            "Mismatch for window mode on `Space-w` and `Ctrl-w`."
        );
        assert_eq!(
            root.traverse(&[key!('z')]).unwrap(),
            root.traverse(&[key!('Z')]).unwrap(),
            "Mismatch for view mode on `z` and `Z`."
        );
    }

    #[test]
    fn command_list() {
        let normal_mode = keymap!({ "Normal mode"
            "i" => insert_mode,
            "g" => { "Goto"
                "g" => goto_file_start,
                "e" => goto_file_end,
            },
            "j" | "k" => move_line_down,
        });
        let keymap = Keymap::new(normal_mode);
        let mut command_list = keymap.command_list();

        // sort keybindings in order to have consistent tests
        // HashMaps can be compared but we can still get different ordering of bindings
        // for commands that have multiple bindings assigned
        for v in command_list.values_mut() {
            v.sort()
        }

        assert_eq!(
            command_list,
            HashMap::from([
                (    
                    "insert_mode".to_string(),
                    vec![key!('i').to_string()]
                ),
                (
                    "goto_file_start".to_string(),
                    vec![format!("{}>{}", key!('g'), key!('g'))]
                ),
                (
                    "goto_file_end".to_string(),
                    vec![format!("{}>{}", key!('g'), key!('e'))]
                ),
                (
                    "move_line_down".to_string(),
                    vec![key!('j').to_string(), key!('k').to_string()]
                )
            ]),
            "Mismatch"
        )
    }
}