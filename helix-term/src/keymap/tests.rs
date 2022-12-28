#[macro_use]
#[cfg(test)]
mod tests {
    use helix_core::hashmap;
    use helix_view::{document::Mode, input::KeyEvent};
    use crate::{config::Config, commands::MappableCommand, keymap::*};
    use arc_swap::access::Constant;

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
    fn merge_partial_keys() {
        let config = Config {
            keys: hashmap! {
                Mode::Normal => Keymap::new(
                    keymap!({ "Normal mode"
                        "i" => normal_mode,
                        "无" => insert_mode,
                        "z" => jump_backward,
                        "g" => { "Merge into goto mode"
                            "$" => goto_line_end,
                            "g" => delete_char_forward,
                        },
                    })
                )
            },
            ..Default::default()
        };
        let mut merged_config = merge_keys(config.clone());
        assert_ne!(config, merged_config);

        let mut keymap = Keymaps::new(Box::new(Constant(merged_config.keys.clone())));
        assert_eq!(
            keymap.get(Mode::Normal, key!('i')),
            KeymapResult::Matched(MappableCommand::normal_mode),
            "New mappable command should ovveride default."
        );
        assert_eq!(
            keymap.get(Mode::Normal, key!('无')),
            KeymapResult::Matched(MappableCommand::insert_mode),
            "New mappable command should be present in merged keymap."
        );
        // Assumes that z is a node in the default keymap
        assert_eq!(
            keymap.get(Mode::Normal, key!('z')),
            KeymapResult::Matched(MappableCommand::jump_backward),
            "New Mappable command should replace default sub keytrie."
        );

        let keymap = merged_config.keys.get_mut(&Mode::Normal).unwrap();
        // Assumes that `g` is a node in default keymap
        assert_eq!(
            keymap.root_node.traverse(&[key!('g'), key!('$')]).unwrap(),
            KeyTrieNode::MappableCommand(MappableCommand::goto_line_end),
            "Mappable command should be present in merged keytrie."
        );
        // Assumes that `gg` is in default keymap
        assert_eq!(
            keymap.root_node.traverse(&[key!('g'), key!('g')]).unwrap(),
            KeyTrieNode::MappableCommand(MappableCommand::delete_char_forward),
            "Mappable command should replace default in merged keytrie."
        );
        // Assumes that `ge` is in default keymap
        assert_eq!(
            keymap.root_node.traverse(&[key!('g'), key!('e')]).unwrap(),
            KeyTrieNode::MappableCommand(MappableCommand::goto_last_line),
            "Mappable commands from default keytrie should still be present in merged merged keytrie unless overridden."
        );

        assert!(merged_config.keys.get(&Mode::Normal).unwrap().len() > 1);
        assert!(merged_config.keys.get(&Mode::Insert).unwrap().len() > 0);
    }

    #[test]
    fn merges_with_default_keymap_config() {
        let config = Config {
            keys: hashmap! {
                Mode::Normal => Keymap::new(
                    keymap!({ "Normal mode"
                        "space" => { ""
                            "s" => { ""
                                "v" => vsplit,
                                "c" => hsplit,
                            },
                        },
                    })
                )
            },
            ..Default::default()
        };
        let mut merged_config = Keymaps::merge_with_default(config.clone());
        assert_ne!(config, merged_config);
        let keymap_normal = merged_config.keys.get_mut(&Mode::Normal).unwrap();
        assert_eq!(
            keymap_normal
                .root_node
                .traverse(&[key!(' '), key!('s'), key!('v')])
                .unwrap(),
            KeyTrieNode::MappableCommand(MappableCommand::vsplit),
            "Mappable command should be present in merged keytrie."
        );
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