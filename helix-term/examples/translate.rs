use helix_term::keymap::default;
use helix_view::document::Mode;
use helix_view::input::{KeyCode, KeyEvent};
use std::collections::HashMap;

const LANGMAP: [(&'static str, &'static str); 6] = [
    (r#"йцукенгшщзхъ"#, r#"qwertyuiop[]"#),
    (r#"ЙЦУКЕНГШЩЗХЪ"#, r#"QWERTYUIOP{}"#),
    (r#"фывапролджэё"#, r#"asdfghjkl;'\"#),
    (r#"ФЫВАПРОЛДЖЭЁ"#, r#"ASDFGHJKL:"|"#),
    (r#"]ячсмитьбю/"#, r#"`zxcvbnm,./"#),
    (r#"[ЯЧСМИТЬБЮ?"#, r#"~ZXCVBNM<>?"#),
];

fn translate<F>(ev: &KeyEvent, f: F) -> Option<KeyEvent>
where
    F: Fn(char) -> Option<char>,
{
    if let KeyCode::Char(c) = ev.code {
        Some(KeyEvent {
            code: KeyCode::Char(f(c)?),
            modifiers: ev.modifiers.clone(),
        })
    } else {
        None
    }
}

fn main() {
    let mut langmap = LANGMAP
        .iter()
        .map(|(ru, en)| ru.chars().zip(en.chars()))
        .flatten()
        .filter(|(en, ru)| en != ru)
        .map(|(ru, en)| (en, ru))
        .collect::<HashMap<_, _>>();

    langmap
        .iter()
        .filter_map(|(en, ru)| langmap.contains_key(ru).then(|| *en))
        .collect::<Vec<_>>()
        .into_iter()
        .for_each(|c| {
            langmap.remove(&c);
        });

    let keymaps = default::default();
    for mode in [Mode::Normal, Mode::Select] {
        println!("[keys.{}]", mode);
        keymaps[&mode].traverse_map(|keys, name| {
            let tr_keys = keys
                .iter()
                .filter_map(|ev| translate(ev, |c| langmap.get(&c).map(|c| *c)))
                .enumerate()
                .map(|(i, ev)| {
                    if i == 0 {
                        ev.to_string()
                    } else {
                        format!("+{}", ev)
                    }
                })
                .collect::<String>();
            if !tr_keys.is_empty() {
                println!(r#"{:?} = "{}""#, tr_keys, name);
            }
        });
    }
}
