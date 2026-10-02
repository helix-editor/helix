use super::*;

use helix_term::{
    commands,
    compositor::{self, Compositor, Event},
    job::Jobs,
    ui,
};
use helix_view::{editor::Severity, graphics::Rect, input::parse_macro};

#[tokio::test(flavor = "multi_thread")]
async fn invalid_regex_prompt_explains_error() -> anyhow::Result<()> {
    // Receive the prompt's deferred popup callback in this component test.
    // Jobs uses the first queue created in this test's runtime.
    let mut jobs = Jobs::new();
    let mut app = AppBuilder::new().build()?;
    let area = Rect::new(0, 0, 80, 24);
    let mut compositor = Compositor::new(area);
    let mut cx = commands::Context {
        register: None,
        count: None,
        editor: &mut app.editor,
        callback: Vec::new(),
        on_next_key_callback: None,
        jobs: &mut jobs,
    };
    ui::regex_prompt(
        &mut cx,
        "/".into(),
        Some('/'),
        |_, _| Vec::new(),
        |_, _, _| panic!("invalid regex must not reach the search callback"),
    );
    for callback in cx.callback {
        callback(
            &mut compositor,
            &mut compositor::Context {
                editor: &mut app.editor,
                jobs: &mut jobs,
                scroll: None,
            },
        );
    }
    for key in parse_macro("+=<ret>")? {
        compositor.handle_event(
            &Event::Key(key),
            &mut compositor::Context {
                editor: &mut app.editor,
                jobs: &mut jobs,
                scroll: None,
            },
        );
    }
    let callback = tokio::time::timeout(std::time::Duration::from_secs(1), jobs.callbacks.recv())
        .await?
        .expect("the invalid-regex popup should be queued");
    jobs.handle_callback(&mut app.editor, &mut compositor, Ok(Some(callback)));

    let mut surface = tui::buffer::Buffer::empty(area);
    compositor.render(
        area,
        &mut surface,
        &mut compositor::Context {
            editor: &mut app.editor,
            jobs: &mut jobs,
            scroll: None,
        },
    );
    let rendered: String = surface
        .content
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect();
    assert!(rendered.contains("+="), "{rendered}");
    assert!(rendered.contains('^'), "{rendered}");
    assert!(
        rendered.contains("repetition operator missing expression"),
        "{rendered}"
    );
    assert!(!rendered.contains("error parsing pattern 0"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn global_search_invalid_regex_clears_on_empty_query() -> anyhow::Result<()> {
    let mut config = test_config();
    // Wait past global search's debounce before asserting on its status.
    config.editor.idle_timeout = std::time::Duration::from_millis(400);
    test_key_sequences(
        &mut AppBuilder::new().with_config(config).build()?,
        vec![
            (
                Some(" /+="),
                Some(&|app| {
                    assert_eq!(
                        app.editor.get_status(),
                        Some((&"Invalid regular expression".into(), &Severity::Error))
                    );
                }),
            ),
            (
                Some("<C-a><C-k>"),
                Some(&|app| assert!(app.editor.get_status().is_none())),
            ),
        ],
        false,
    )
    .await
}
