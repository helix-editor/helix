use crate::{
    commands::MappableCommand,
    key,
    keymap::{
        keytrie::KeyTrie,
        keytrienode::{CommandSequence, KeyTrieNode},
        macros::keytrie,
        Keymap,
    },
};
use arc_swap::ArcSwap;
use helix_core::hashmap;
use helix_view::{document::Mode, input::KeyEvent};
use std::{collections::HashMap, sync::Arc};

#[test]
#[should_panic]
fn duplicate_keys_should_panic() {
    keytrie!({ "Normal mode"
        "i" => normal_mode,
        "i" => goto_definition,
    });
}

#[test]
fn check_duplicate_keys_in_default_keymap() {
    // will panic on duplicate keys, assumes that `Keymap` uses keymap! macro
    Keymap::default();
}

#[test]
fn aliased_modes_are_same_in_default_keymap() {
    let normal_mode_keytrie_root = Keymap::default().get_keytrie(&Mode::Normal);
    assert_eq!(
        normal_mode_keytrie_root
            .traverse(&[key!(' '), key!('w')])
            .unwrap(),
        normal_mode_keytrie_root
            .traverse(&["C-w".parse::<KeyEvent>().unwrap()])
            .unwrap(),
        "Mismatch for window mode on `Space-w` and `Ctrl-w`."
    );
    assert_eq!(
        normal_mode_keytrie_root.traverse(&[key!('z')]).unwrap(),
        normal_mode_keytrie_root.traverse(&[key!('Z')]).unwrap(),
        "Mismatch for view mode on `z` and `Z`."
    );
}

#[test]
fn command_list() {
    let normal_mode = keytrie!({ "Normal mode"
        "i" => insert_mode,
        "g" => { "Goto"
            "g" => goto_file_start,
            "e" => goto_file_end,
        },
        "j" | "k" => move_line_down,
    });

    let keymap = Keymap::new(Box::new(ArcSwap::new(Arc::new(
        hashmap!(Mode::Normal => normal_mode),
    ))));
    let mut command_list = keymap.command_list(&Mode::Normal);

    // sort keybindings in order to have consistent tests
    // HashMaps can be compared but we can still get different ordering of bindings
    // for commands that have multiple bindings assigned
    for v in command_list.values_mut() {
        v.sort()
    }

    assert_eq!(
        command_list,
        HashMap::from([
            ("insert_mode".to_string(), vec![key!('i').to_string()]),
            (
                "goto_file_start".to_string(),
                vec![format!("{}→{}", key!('g'), key!('g'))]
            ),
            (
                "goto_file_end".to_string(),
                vec![format!("{}→{}", key!('g'), key!('e'))]
            ),
            (
                "move_line_down".to_string(),
                vec![key!('j').to_string(), key!('k').to_string()]
            )
        ]),
        "Mismatch"
    )
}

#[test]
fn escaped_keymap() {
    let parsed_keytrie: KeyTrie = toml::from_str(
        r#"
"+" = [
    "select_all",
    ":pipe sed -E 's/\\s+$//g'",
]
    "#,
    )
    .unwrap();

    let command_sequence = KeyTrieNode::CommandSequence(CommandSequence::descriptionless(vec![
        MappableCommand::select_all,
        MappableCommand::Typable {
            name: "pipe".to_string(),
            args: vec![
                "sed".to_string(),
                "-E".to_string(),
                "'s/\\s+$//g'".to_string(),
            ],
            description: "".to_string(),
        },
    ]));

    assert_eq!(parsed_keytrie.get_children()[0], command_sequence);
}
