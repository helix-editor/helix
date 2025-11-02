use super::*;

use std::cell::Cell;

use helix_core::doc_formatter::{DocumentFormatter, TextFormat};
use helix_core::graphemes::prev_grapheme_boundary;
use helix_core::{coords_at_pos, Position, Range};
use helix_core::{Rope, RopeSlice};

use helix_view::current_ref;

const RUST_CODE: &str = "tests/test/commands/text_folding/rust-code.rs";
const PYTHON_CODE: &str = "tests/test/commands/text_folding/python-code.py";

const FOLDED_RUST_CODE: &str = "tests/test/commands/text_folding/folded-rust-code";
const FOLDED_PYTHON_CODE: &str = "tests/test/commands/text_folding/folded-python-code";

fn fold_text(app: &Application) -> Rope {
    use helix_core::graphemes::Grapheme;
    use std::fmt::Write;

    let (view, doc) = current_ref!(&app.editor);
    let text = doc.text().slice(..);
    let text_format = &TextFormat::default();
    let annotations = &view.text_annotations(doc, None);

    let formatter = DocumentFormatter::new_at_prev_checkpoint(text, text_format, annotations, 0);

    let mut folded_text = String::new();
    for g in formatter {
        match g.raw {
            Grapheme::Newline => write!(folded_text, "\n").unwrap(),
            other => write!(folded_text, "{other}").unwrap(),
        }
    }
    // remove EOF
    folded_text.remove(folded_text.len() - 1);

    Rope::from(folded_text)
}

// NOTE: positions are one-based indexing
// Returns (from, to)
fn positions_from_range(text: RopeSlice, range: Range) -> (Position, Position) {
    let Position { row, col } = coords_at_pos(text, range.from());
    let from = Position {
        row: row + 1,
        col: col + 1,
    };

    let Position { row, col } = coords_at_pos(text, prev_grapheme_boundary(text, range.to()));
    let to = Position {
        row: row + 1,
        col: col + 1,
    };

    (from, to)
}

// NOTE: position is one-based indexing
fn position_from_char(text: RopeSlice, char: usize) -> Position {
    let Position { row, col } = coords_at_pos(text, char);
    Position {
        row: row + 1,
        col: col + 1,
    }
}

// INFO: to update the folded text, set the environment variable HELIX_UPDATE_FOLDED_RUST_CODE
#[tokio::test(flavor = "multi_thread")]
async fn fold_rust_code() -> anyhow::Result<()> {
    use std::fs::File;

    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    test_key_sequence(
        app,
        Some(":fold --all --document<ret>"),
        Some(&|app| {
            let folded_rust_code = fold_text(app);
            match std::env::var("HELIX_UPDATE_FOLDED_RUST_CODE") {
                Ok(_) => {
                    folded_rust_code
                        .write_to(File::create(FOLDED_RUST_CODE).unwrap())
                        .unwrap();
                }
                Err(_) => {
                    let expected =
                        Rope::from_reader(File::open(FOLDED_RUST_CODE).unwrap()).unwrap();
                    assert_eq!(folded_rust_code, expected);
                }
            }
        }),
        false,
    )
    .await
}

// INFO: to update the folded text, set the environment variable HELIX_UPDATE_FOLDED_PYTHON_CODE
#[tokio::test(flavor = "multi_thread")]
async fn fold_python_code() -> anyhow::Result<()> {
    use std::fs::File;

    let app = &mut AppBuilder::new()
        .with_file(PYTHON_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    test_key_sequence(
        app,
        Some(":fold --all --document<ret>"),
        Some(&|app| {
            let folded_python_code = fold_text(app);
            match std::env::var("HELIX_UPDATE_FOLDED_PYTHON_CODE") {
                Ok(_) => {
                    folded_python_code
                        .write_to(File::create(FOLDED_PYTHON_CODE).unwrap())
                        .unwrap();
                }
                Err(_) => {
                    let expected =
                        Rope::from_reader(File::open(FOLDED_PYTHON_CODE).unwrap()).unwrap();
                    assert_eq!(folded_python_code, expected);
                }
            }
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_class() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: positions are one-based indexing
    // ((from, to), additional folds number)
    type TestResult = ((Position, Position), isize);

    let prev_folds_number = Cell::new(0);
    let result = |app: &Application| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);
        let text = doc.text().slice(..);

        let (from, to) = {
            let range = doc.selection(view.id).primary();
            positions_from_range(text, range)
        };

        let additional_folds_number = {
            let folds_number = doc
                .fold_container(view.id)
                .map_or(0, |container| container.len());
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        ((from, to), additional_folds_number)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(
                    "g10g\
                    g5\
                    |zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(10, 5), Position::new(10, 5)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g48g\
                    g2|\
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(48, 2), Position::new(48, 2)), 0);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g46g\
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(44, 1), Position::new(44, 13)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g105g\
                    xx\
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(104, 17), Position::new(104, 36)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g93g\
                    v\
                    g100g\
                    :fold class<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(90, 9), Position::new(98, 20)), 2);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g51g\
                    g6|\
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(51, 6), Position::new(51, 6)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_function() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: positions are one-based indexing
    // ((from, to), additional folds number)
    type TestResult = ((Position, Position), isize);

    let prev_folds_number = Cell::new(0);
    let result = |app: &Application| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);
        let text = doc.text().slice(..);

        let (from, to) = {
            let range = doc.selection(view.id).primary();
            positions_from_range(text, range)
        };

        let additional_folds_number = {
            let folds_number = doc
                .fold_container(view.id)
                .map_or(0, |container| container.len());
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        ((from, to), additional_folds_number)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(
                    "g77g\
                    v\
                    g83g\
                    :fold function<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(77, 1), Position::new(80, 20)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g105g\
                    g21|\
                    :fold function<ret>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(99, 13), Position::new(99, 19)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g64g\
                    v\
                    g77g\
                    :fold function<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(60, 5), Position::new(75, 22)), 2);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g50g\
                    mat\
                    :fold function<ret>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(50, 1), Position::new(115, 1)), 0);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "gg\
                    :fold -a -d class comment<ret>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(1, 1), Position::new(1, 1)), 0);
                    assert_eq!(result(app), expected);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_comment() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: positions are one-based indexing
    // ((from, to), additional folds number)
    type TestResult = ((Position, Position), isize);

    let prev_folds_number = Cell::new(0);
    let result = |app: &Application| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);
        let text = doc.text().slice(..);

        let (from, to) = {
            let range = doc.selection(view.id).primary();
            positions_from_range(text, range)
        };

        let additional_folds_number = {
            let folds_number = doc
                .fold_container(view.id)
                .map_or(0, |container| container.len());
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        ((from, to), additional_folds_number)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(
                    "gg\
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(1, 1), Position::new(1, 1)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g8g\
                    g27|\
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(7, 5), Position::new(7, 27)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g12g\
                    g27|
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(12, 27), Position::new(12, 27)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g15g\
                    g7|
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(15, 7), Position::new(15, 7)), 0);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g18g\
                    g16|\
                    v\
                    g30g\
                    g9|\
                    :fold comment<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(18, 16), Position::new(29, 28)), 2);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g32g\
                    v\
                    g37g
                    :fold comment<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(32, 1), Position::new(36, 17)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g53g\
                    g5|
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(53, 5), Position::new(53, 5)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g102g\
                    g40|\
                    zf",
                ),
                Some(&|app| {
                    let expected = ((Position::new(100, 17), Position::new(100, 41)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn fold_selection() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: positions are one-based indexing
    // ((from, to), additional folds number)
    type TestResult = ((Position, Position), isize);

    let prev_folds_number = Cell::new(0);
    let result = |app: &Application| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);
        let text = doc.text().slice(..);

        let (from, to) = {
            let range = doc.selection(view.id).primary();
            positions_from_range(text, range)
        };

        let additional_folds_number = {
            let folds_number = doc
                .fold_container(view.id)
                .map_or(0, |container| container.len());
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        ((from, to), additional_folds_number)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(
                    "g2g\
                    v\
                    g10g\
                    :fold -s<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(2, 1), Position::new(2, 22)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g15g\
                    v\
                    g25g\
                    :fold -s<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(15, 1), Position::new(15, 29)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g11g\
                    v\
                    g14g\
                    :fold -s<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(11, 1), Position::new(11, 8)), 1);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g2g\
                    v\
                    2j<esc>",
                ),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let text = doc.text().slice(..);

                    let range = doc.selection(view.id).primary();
                    let (start, end) = range.line_range(text);

                    let expected = (1, 14);
                    assert_eq!((start, end), expected, "select fold headers");
                }),
            ),
            (
                Some(":fold -s<ret>"),
                Some(&|app| {
                    let expected = ((Position::new(2, 1), Position::new(2, 22)), -2);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "v\
                    j<esc>",
                ),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let text = doc.text().slice(..);

                    let range = doc.selection(view.id).primary();
                    let (start, end) = range.line_range(text);

                    let expected = (1, 25);
                    assert_eq!((start, end), expected, "select fold");
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn fold() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: positions are one-based indexing
    // ((from, to), additional folds number)
    type TestResult = ((Position, Position), isize);

    let prev_folds_number = Cell::new(0);
    let result = |app: &Application| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);
        let text = doc.text().slice(..);

        let (from, to) = {
            let range = doc.selection(view.id).primary();
            positions_from_range(text, range)
        };

        let additional_folds_number = {
            let folds_number = doc
                .fold_container(view.id)
                .map_or(0, |container| container.len());
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        ((from, to), additional_folds_number)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(
                    "g5g\
                    gl\
                    mam\
                    <A-;>\
                    v\
                    gg\
                    :fold -a<ret><esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(1, 1), Position::new(34, 1)), 7);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g37g\
                    v\
                    g46g\
                    zf<esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(36, 1), Position::new(44, 13)), 3);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g64g\
                    v\
                    g80g\
                    zf<esc>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(60, 5), Position::new(75, 22)), 4);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g50g\
                    mat\
                    :fold comment<ret>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(50, 1), Position::new(115, 1)), 6);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some("zf"),
                Some(&|app| {
                    let expected = ((Position::new(50, 1), Position::new(58, 13)), 6);
                    assert_eq!(result(app), expected);
                }),
            ),
            (
                Some(
                    "g5g\
                    g22|\
                    mam\
                    :fold -s<ret>",
                ),
                Some(&|app| {
                    let expected = ((Position::new(5, 22), Position::new(5, 23)), -5);
                    assert_eq!(result(app), expected);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn format() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    let prev_folds_number = Cell::new(0);
    test_key_sequences(
        app,
        vec![
            (
                Some(":fold -a -d<ret>"),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some(":format<ret>"),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    assert_eq!(
                        container.len(),
                        prev_folds_number.get(),
                        "All folds must be retained."
                    );
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn unfold_class() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: header is one-based indexing (row, col)
    // (additional folds number, unexpected fold headers)
    type TestResult = (isize, Vec<Position>);

    let prev_folds_number = Cell::new(0);
    // NOTE: header is one-based indexing (row, col)
    let result = |app: &Application, headers: &[(usize, usize)]| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);

        let text = doc.text().slice(..);
        let container = doc
            .fold_container(view.id)
            .expect("Container must be initialized.");

        let additional_folds_number = {
            let folds_number = container.len();
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        let unexpected_fold_headers = container
            .start_points()
            .iter()
            .map(|sfp| position_from_char(text, sfp.header))
            .filter(|&header| {
                let Position { row, col } = header;
                headers.contains(&(row, col))
            })
            .collect();

        (additional_folds_number, unexpected_fold_headers)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(":fold -a -d<ret>"),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some(
                    "g15g\
                    g6|\
                    zF",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) = result(app, &[]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, 0);
                }),
            ),
            (
                Some(
                    "g15g\
                    g5|
                    zF",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(10, 5)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g18g\
                    g5|
                    :unfold class<ret>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(17, 17)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g35g\
                    v\
                    g49g\
                    :unfold class<ret><esc>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(44, 1)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g52g\
                    g5|
                    zF",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(50, 1)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g89g\
                    v\
                    g110g\
                    :unfold -r class<ret><esc>",
                ),
                Some(&|app| {
                    // NOTE: when navigating to line 89, the function `g` is unfolded
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(75, 5), (90, 9), (98, 9), (104, 17)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -4);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn unfold_function() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: header is one-based indexing (row, col)
    // (additional folds number, unexpected fold headers)
    type TestResult = (isize, Vec<Position>);

    let prev_folds_number = Cell::new(0);
    // NOTE: header is one-based indexing (row, col)
    let result = |app: &Application, headers: &[(usize, usize)]| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);

        let text = doc.text().slice(..);
        let container = doc
            .fold_container(view.id)
            .expect("Container must be initialized.");

        let additional_folds_number = {
            let folds_number = container.len();
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        let unexpected_fold_headers = container
            .start_points()
            .iter()
            .map(|sfp| position_from_char(text, sfp.header))
            .filter(|&header| {
                let Position { row, col } = header;
                headers.contains(&(row, col))
            })
            .collect();

        (additional_folds_number, unexpected_fold_headers)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(":fold -a -d class<ret>"),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some(
                    "g71g\
                    g6|\
                    zF",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) = result(app, &[]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, 0);
                }),
            ),
            (
                Some(
                    "g71g\
                    g5|\
                    zF",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(60, 5)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g50g\
                    mat\
                    :unfold -r function<ret>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(75, 5), (80, 9), (99, 13)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -3);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn unfold_comment() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: header is one-based indexing (row, col)
    // (additional folds number, unexpected fold headers)
    type TestResult = (isize, Vec<Position>);

    let prev_folds_number = Cell::new(0);
    // NOTE: header is one-based indexing (row, col)
    let result = |app: &Application, headers: &[(usize, usize)]| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);

        let text = doc.text().slice(..);
        let container = doc
            .fold_container(view.id)
            .expect("Container must be initialized.");

        let additional_folds_number = {
            let folds_number = container.len();
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        let unexpected_fold_headers = container
            .start_points()
            .iter()
            .map(|sfp| position_from_char(text, sfp.header))
            .filter(|&header| {
                let Position { row, col } = header;
                headers.contains(&(row, col))
            })
            .collect();

        (additional_folds_number, unexpected_fold_headers)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(
                    ":fold -a -d<ret>\
                    g50g\
                    mat\
                    :unfold -r class<ret>\
                    gg",
                ),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some("zF"),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) = result(app, &[(1, 1)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g5g\
                    g22|\
                    mam\
                    :unfold comment<ret>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(7, 5), (18, 5)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -2);
                }),
            ),
            (
                Some(
                    "g5g\
                    g22|\
                    mam\
                    :unfold -r comment<ret>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(12, 13), (29, 9)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -2);
                }),
            ),
            (
                Some(
                    "g17g\
                    v\
                    g40g\
                    :unfold comment<ret><esc>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(36, 1), (40, 1)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -2);
                }),
            ),
            (
                Some(
                    "g61g\
                    zF",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(61, 1)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g71g\
                    g7|\
                    zF",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(71, 7)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g75g\
                    g5|\
                    maf\
                    :unfold -r comment<ret>",
                ),
                Some(&|app| {
                    // NOTE: when navigating to line 75, the function `g` is unfolded
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(75, 5), (76, 9), (90, 23), (100, 17), (111, 9)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -5);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn unfold_selection() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: header is one-based indexing (row, col)
    // (additional folds number, unexpected fold headers)
    type TestResult = (isize, Vec<Position>);

    let prev_folds_number = Cell::new(0);
    // NOTE: header is one-based indexing (row, col)
    let result = |app: &Application, headers: &[(usize, usize)]| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);

        let text = doc.text().slice(..);
        let container = doc
            .fold_container(view.id)
            .expect("Container must be initialized.");

        let additional_folds_number = {
            let folds_number = container.len();
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        let unexpected_fold_headers = container
            .start_points()
            .iter()
            .map(|sfp| position_from_char(text, sfp.header))
            .filter(|&header| {
                let Position { row, col } = header;
                headers.contains(&(row, col))
            })
            .collect();

        (additional_folds_number, unexpected_fold_headers)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(
                    "g2g\
                    v\
                    g5g\
                    :fold -s<ret><esc>\
                    g7g\
                    v\
                    g10g\
                    :fold -s<ret><esc>\
                    g12g\
                    v\
                    g15g\
                    :fold -s<ret><esc>\
                    gg",
                ),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some(
                    "g2g\
                    :unfold -s<ret>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) = result(app, &[(2, 1)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -1);
                }),
            ),
            (
                Some(
                    "g7g\
                    v\
                    g12g\
                    :unfold -s<ret><esc>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(7, 1), (12, 1)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -2);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn unfold() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // NOTE: header is one-based indexing (row, col)
    // (additional folds number, unexpected fold headers)
    type TestResult = (isize, Vec<Position>);

    let prev_folds_number = Cell::new(0);
    // NOTE: header is one-based indexing (row, col)
    let result = |app: &Application, headers: &[(usize, usize)]| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);

        let text = doc.text().slice(..);
        let container = doc
            .fold_container(view.id)
            .expect("Container must be initialized.");

        let additional_folds_number = {
            let folds_number = container.len();
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        let unexpected_fold_headers = container
            .start_points()
            .iter()
            .map(|sfp| position_from_char(text, sfp.header))
            .filter(|&header| {
                let Position { row, col } = header;
                headers.contains(&(row, col))
            })
            .collect();

        (additional_folds_number, unexpected_fold_headers)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(":fold -a -d<ret>"),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some(
                    "g5g\
                    g22|\
                    mam\
                    <A-;>\
                    v\
                    gg\
                    :unfold -a<ret><esc>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) =
                        result(app, &[(1, 1), (7, 5), (10, 5), (17, 17), (18, 5)]);

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -5);
                }),
            ),
            (
                Some(
                    "g50g\
                    v\
                    g116g\
                    :unfold -a -r comment<ret><esc>",
                ),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) = result(
                        app,
                        &[
                            (50, 1),
                            (50, 5),
                            (75, 5),
                            (80, 9),
                            (90, 9),
                            (98, 9),
                            (99, 13),
                            (104, 17),
                        ],
                    );

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -8);
                }),
            ),
            (
                Some(":unfold -a -d -r<ret>"),
                Some(&|app| {
                    let (additional_folds_number, unexpected_fold_headers) = result(
                        app,
                        &[
                            (12, 13),
                            (29, 9),
                            (36, 1),
                            (40, 1),
                            (44, 1),
                            (53, 5),
                            (56, 5),
                            (61, 1),
                            (71, 7),
                            (76, 9),
                            (90, 23),
                            (100, 17),
                            (111, 9),
                        ],
                    );

                    assert!(
                        unexpected_fold_headers.is_empty(),
                        "{unexpected_fold_headers:#?}"
                    );
                    assert_eq!(additional_folds_number, -13);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn open() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    // (text of the new line, additional folds number)
    type TestResult<'a> = (String, isize);

    let prev_folds_number = Cell::new(0);
    // NOTE: new_line is one-based indexing
    let result = |app: &Application, new_line: usize| -> TestResult {
        let (view, doc) = current_ref!(&app.editor);
        let text = doc.text().slice(..);
        let container = doc
            .fold_container(view.id)
            .expect("Container must be initialized.");

        let additional_folds_number = {
            let folds_number = container.len();
            folds_number as isize - prev_folds_number.replace(folds_number) as isize
        };

        let new_line = text.line(new_line - 1).as_str().unwrap();

        (new_line.into(), additional_folds_number)
    };

    test_key_sequences(
        app,
        vec![
            (
                Some(":fold -a -d class<ret>"),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some(
                    "g53g\
                    o\
                    new text<esc>",
                ),
                Some(&|app| {
                    let expected = ("    new text\n".into(), 0);
                    assert_eq!(result(app, 55), expected);
                }),
            ),
            (Some("xd"), None),
            (
                Some(
                    "g55g\
                    O\
                    new text<esc>",
                ),
                Some(&|app| {
                    let expected = ("    new text\n".into(), 0);
                    assert_eq!(result(app, 55), expected);
                }),
            ),
            (Some("xd"), None),
            (
                Some(
                    "g63g\
                    o\
                    new text<esc>",
                ),
                Some(&|app| {
                    let expected = ("        new text\n".into(), -1);
                    assert_eq!(result(app, 71), expected);
                }),
            ),
            (
                Some("xdzf"),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some(
                    "g71g\
                    O\
                    new text<esc>",
                ),
                Some(&|app| {
                    let expected = ("        new text\n".into(), -1);
                    assert_eq!(result(app, 71), expected);
                }),
            ),
            (
                Some(
                    "gg\
                    o\
                    new text<esc>",
                ),
                Some(&|app| {
                    let expected = ("new text\n".into(), 0);
                    assert_eq!(result(app, 4), expected);
                }),
            ),
            (
                Some(
                    "xd\
                    gg\
                    zF",
                ),
                Some(&|app| {
                    let (view, doc) = current_ref!(&app.editor);
                    let container = doc
                        .fold_container(view.id)
                        .expect("Container must be initialized.");

                    prev_folds_number.set(container.len());
                }),
            ),
            (
                Some(
                    "g3g\
                    o\
                    new text<esc>",
                ),
                Some(&|app| {
                    let expected = ("//! new text\n".into(), 0);
                    assert_eq!(result(app, 4), expected);
                }),
            ),
        ],
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn default_folding() -> anyhow::Result<()> {
    use helix_view::editor::LspConfig;

    let config = Config {
        editor: helix_view::editor::Config {
            fold_textobjects: vec!["class".into(), "function".into()],
            lsp: LspConfig {
                // suppress lsp error
                enable: false,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };

    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .with_config(config)
        .build()
        .unwrap();

    test_key_sequence(
        app,
        None,
        Some(&|app| {
            let (view, doc) = current_ref!(&app.editor);
            let container = doc
                .fold_container(view.id)
                .expect("Container must be initialized.");

            let folds_number = container.len();
            assert_eq!(folds_number, 11);
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn toggle_fold() -> anyhow::Result<()> {
    let app = &mut AppBuilder::new()
        .with_file(RUST_CODE, None)
        .with_lang_loader(helpers::test_syntax_loader(None))
        .build()
        .unwrap();

    let folds_number = |app: &Application| {
        let (view, doc) = current_ref!(&app.editor);
        doc.fold_container(view.id)
            .map_or(0, |container| container.len())
    };

    test_key_sequences(
        app,
        vec![
            (
                Some("z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 1)),
            ),
            (
                Some("z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 0)),
            ),
            (
                Some("g7gz<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 0)),
            ),
            (
                Some("g5|z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 1)),
            ),
            (
                Some("z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 0)),
            ),
            (
                Some("g10gg5|z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 1)),
            ),
            (
                Some("z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 0)),
            ),
            (
                Some("g12gg13|z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 1)),
            ),
            (
                Some("z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 0)),
            ),
            (
                Some("g17gg17|z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 1)),
            ),
            (
                Some("z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 0)),
            ),
            (
                Some("g93gg13|z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 1)),
            ),
            (
                Some("z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 0)),
            ),
            (
                Some("g105gg21|z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 1)),
            ),
            (
                Some("z<A-f>"),
                Some(&|app| assert_eq!(folds_number(app), 0)),
            ),
        ],
        false,
    )
    .await
}
