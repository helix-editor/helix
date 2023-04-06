use helix_term::application::Application;
use helix_term::ui::prompt::Prompt;

use super::*;

fn get_prompt(app: &Application) -> &Prompt {
    let type_name = std::any::type_name::<Prompt>();

    app.get_compositor_layers()
        .iter()
        .find(|component| component.type_name() == type_name)
        .and_then(|component| component.as_any().downcast_ref::<Prompt>())
        .expect("expected prompt component")
}

#[tokio::test(flavor = "multi_thread")]
async fn history_completion() -> anyhow::Result<()> {
    test_key_sequence(
        &mut AppBuilder::new().build()?,
        Some(":asdf<ret>:theme d<C-n><tab>"),
        Some(&|app| {
            assert!(!app.editor.is_err());

            // Before this PR:
            // assert_eq!("asdf", get_prompt(app).line());

            assert_eq!("theme darcula", get_prompt(app).line());
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn history_control_previous() -> anyhow::Result<()> {
    test_key_sequences(
        &mut AppBuilder::new().build()?,
        vec![
            (
                Some(":1<ret>:2<ret>:3<ret>:"),
                Some(&|app| {
                    assert_eq!("", get_prompt(app).line(), "[1,2,3] (no input)");
                }),
            ),
            (
                Some("<esc>:<C-p>"),
                Some(&|app| {
                    assert_eq!("3", get_prompt(app).line(), "[1,2,3] ^");
                }),
            ),
            (
                Some("<esc>:<C-p><C-p>"),
                Some(&|app| {
                    assert_eq!("2", get_prompt(app).line(), "[1,2,3] ^^");
                }),
            ),
            (
                Some("<esc>:<C-p><C-p><C-p>"),
                Some(&|app| {
                    assert_eq!("1", get_prompt(app).line(), "[1,2,3] ^^^");
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn history_control_next() -> anyhow::Result<()> {
    test_key_sequences(
        &mut AppBuilder::new().build()?,
        vec![
            (
                Some(":1<ret>:2<ret>:3<ret>:<C-n>"),
                Some(&|app| {
                    assert_eq!("", get_prompt(app).line(), "[1,2,3] v (no-op)");
                }),
            ),
            (
                Some("<esc>:<C-p><C-n>"),
                Some(&|app| {
                    assert_eq!("", get_prompt(app).line(), "[1,2,3] ^v (reset)");
                }),
            ),
            (
                Some("<esc>:<C-p><C-p><C-n>"),
                Some(&|app| {
                    assert_eq!("3", get_prompt(app).line(), "[1,2,3] ^^v");
                }),
            ),
            (
                Some("<esc>:<C-p><C-p><C-p><C-n>"),
                Some(&|app| {
                    assert_eq!("2", get_prompt(app).line(), "[1,2,3] ^^^v");
                }),
            ),
            (
                Some("<esc>:<C-p><C-p><C-p><C-p><C-n>"),
                Some(&|app| {
                    assert_eq!("2", get_prompt(app).line(), "[1,2,3] ^^^^v (one extra ^)");
                }),
            ),
            (
                Some("<esc>:<C-p><C-p><C-p><C-n><C-n>"),
                Some(&|app| {
                    assert_eq!("3", get_prompt(app).line(), "[1,2,3] ^^^vv");
                }),
            ),
            (
                Some("<esc>:<C-p><C-p><C-p><C-n><C-n><C-n>"),
                Some(&|app| {
                    assert_eq!("", get_prompt(app).line(), "[1,2,3] ^^^vvv (reset)");
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn history_arrow_up() -> anyhow::Result<()> {
    test_key_sequences(
        &mut AppBuilder::new().build()?,
        vec![
            (
                Some(":1<ret>:2<ret>:3<ret>:"),
                Some(&|app| {
                    assert_eq!("", get_prompt(app).line(), "[1,2,3] (no input)");
                }),
            ),
            (
                Some("<esc>:<up>"),
                Some(&|app| {
                    assert_eq!("3", get_prompt(app).line(), "[1,2,3] ^");
                }),
            ),
            (
                Some("<esc>:<up><up>"),
                Some(&|app| {
                    assert_eq!("2", get_prompt(app).line(), "[1,2,3] ^^");
                }),
            ),
            (
                Some("<esc>:<up><up><up>"),
                Some(&|app| {
                    assert_eq!("1", get_prompt(app).line(), "[1,2,3] ^^^");
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn history_arrow_down() -> anyhow::Result<()> {
    test_key_sequences(
        &mut AppBuilder::new().build()?,
        vec![
            (
                Some(":1<ret>:2<ret>:3<ret>:<down>"),
                Some(&|app| {
                    assert_eq!("", get_prompt(app).line(), "[1,2,3] v (no-op)");
                }),
            ),
            (
                Some("<esc>:<up><down>"),
                Some(&|app| {
                    assert_eq!("", get_prompt(app).line(), "[1,2,3] ^v (reset)");
                }),
            ),
            (
                Some("<esc>:<up><up><down>"),
                Some(&|app| {
                    assert_eq!("3", get_prompt(app).line(), "[1,2,3] ^^v");
                }),
            ),
            (
                Some("<esc>:<up><up><up><down>"),
                Some(&|app| {
                    assert_eq!("2", get_prompt(app).line(), "[1,2,3] ^^^v");
                }),
            ),
            (
                Some("<esc>:<up><up><up><up><down>"),
                Some(&|app| {
                    assert_eq!("2", get_prompt(app).line(), "[1,2,3] ^^^^v (one extra ^)");
                }),
            ),
            (
                Some("<esc>:<up><up><up><down><down>"),
                Some(&|app| {
                    assert_eq!("3", get_prompt(app).line(), "[1,2,3] ^^^vv");
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn history_partial_search() -> anyhow::Result<()> {
    test_key_sequences(
        &mut AppBuilder::new().build()?,
        vec![
            (
                Some(":10<ret>:20<ret>:100<ret>:200<ret>:1"),
                Some(&|app| {
                    assert_eq!("1", get_prompt(app).line(), "[10,20,100,200] (no search)");
                }),
            ),
            (
                Some("<esc>:1<up>"),
                Some(&|app| {
                    assert_eq!("100", get_prompt(app).line(), "[10,20,100,200] ^");
                }),
            ),
            (
                Some("<esc>:1<up><up>"),
                Some(&|app| {
                    assert_eq!("10", get_prompt(app).line(), "[10,20,100,200] ^^");
                }),
            ),
            (
                Some("<esc>:1<up><up><up>"),
                Some(&|app| {
                    assert_eq!(
                        "10",
                        get_prompt(app).line(),
                        "[10,20,100,200] ^^^ (one extra ^)"
                    );
                }),
            ),
            (
                Some("<esc>:1<up><up><up><down>"),
                Some(&|app| {
                    assert_eq!("100", get_prompt(app).line(), "[10,20,100,200] ^^^v");
                }),
            ),
            (
                Some("<esc>:1<up><up><up><down><down>"),
                Some(&|app| {
                    assert_eq!(
                        "1",
                        get_prompt(app).line(),
                        "[10,20,100,200] ^^^vv (back to search term)"
                    );
                }),
            ),
            (
                Some("<esc>:1<up><up><up><down><down><down>"),
                Some(&|app| {
                    assert_eq!("", get_prompt(app).line(), "[10,20,100,200] ^^^vvv (reset)");
                }),
            ),
        ],
        false,
    )
    .await
}
