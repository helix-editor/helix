use std::fs;
use std::ops;
use std::sync::LazyLock;

use helix_stdx::rope::RopeSliceExt;

use crate::{text_folding::FoldObject, RopeSlice};

use super::{EndFoldPoint, Fold, FoldContainer, StartFoldPoint};

pub(crate) static TEXT_SAMPLE: LazyLock<RopeSlice> = LazyLock::new(|| {
    const PATH: &str = "src/text_folding/test_utils/text-sample.txt";
    RopeSlice::from(fs::read_to_string(PATH).unwrap().leak() as &str)
});

// INFO: to update the text set the envaroment variable HELIX_UPDATE_FOLDED_SIMPLE_TEXT
pub(crate) static FOLDED_TEXT_SAMPLE: LazyLock<RopeSlice> = LazyLock::new(|| {
    use std::fmt::Write;

    use crate::doc_formatter::{DocumentFormatter, TextFormat};
    use crate::graphemes::Grapheme;
    use crate::text_annotations::TextAnnotations;

    const PATH: &str = "src/text_folding/test_utils/folded-text-sample";
    const VAR: &str = "HELIX_UPDATE_FOLDED_SIMPLE_TEXT";

    let container = &FoldContainer::from(*TEXT_SAMPLE, fold_points());

    let text_format = &TextFormat::default();
    let annotations = &mut TextAnnotations::default();
    annotations.add_folds(container);

    let formatter =
        DocumentFormatter::new_at_prev_checkpoint(*TEXT_SAMPLE, text_format, &annotations, 0);

    let mut folded_text = String::new();
    for g in formatter {
        match g.raw {
            Grapheme::Newline => write!(folded_text, "\n").unwrap(),
            Grapheme::Tab { width: _ } => write!(folded_text, "\t").unwrap(),
            Grapheme::Other { g } => write!(folded_text, "{g}").unwrap(),
        }
    }
    // remove EOf
    folded_text.remove(folded_text.len() - 1);

    match std::env::var(VAR) {
        Ok(_) => fs::write(PATH, &folded_text).unwrap(),
        Err(_) => assert_eq!(folded_text, fs::read_to_string(PATH).unwrap()),
    }

    RopeSlice::from(folded_text.leak() as &str)
});

pub(crate) fn new_fold_points(
    text: RopeSlice,
    object: &'static str,
    header_line: usize,
    target_lines: ops::RangeInclusive<usize>,
) -> (StartFoldPoint, EndFoldPoint) {
    let object = FoldObject::TextObject(object);
    let header = text.line_to_char(header_line)
        + text.line(header_line).first_non_whitespace_char().unwrap();
    let target = {
        let (from, to) = (*target_lines.start(), *target_lines.end());
        let start = text.line_to_char(from) + text.line(from).first_non_whitespace_char().unwrap();
        let end = text.line_to_char(to) + text.line(to).last_non_whitespace_char().unwrap();
        start..=end
    };
    Fold::new_points(text, object, header, &target)
}

pub(crate) fn fold_points() -> Vec<(StartFoldPoint, EndFoldPoint)> {
    // object, header line, target lines
    [
        ("0", 0, 1..=1),
        ("1", 3, 4..=4),
        ("2", 6, 8..=29),
        ("3", 8, 10..=11),
        ("4", 15, 16..=18),
        ("5", 14, 19..=25), // block: 20..=25
        ("6", 19, 20..=22),
        ("7", 27, 28..=29),
        ("8", 28, 29..=29),
        ("9", 31, 32..=36),
        ("10", 32, 33..=35),
        ("11", 33, 34..=35),
        ("12", 39, 41..=45),
        ("13", 41, 43..=45),
        ("14", 46, 48..=50),
        ("15", 46, 52..=55),
        ("16", 58, 59..=59),
        ("17", 60, 61..=61),
        ("18", 62, 63..=63),
        ("19", 58, 66..=67),
        ("20", 74, 76..=76),
        ("21", 72, 78..=78),
    ]
    .into_iter()
    .map(|(object, header_line, target_lines)| {
        new_fold_points(*TEXT_SAMPLE, object, header_line, target_lines)
    })
    .collect()
}

pub(crate) fn fold_points_filtered_by(
    f: impl Fn(&(usize, (StartFoldPoint, EndFoldPoint))) -> bool,
) -> Vec<(StartFoldPoint, EndFoldPoint)> {
    fold_points()
        .into_iter()
        .enumerate()
        .filter(f)
        .map(|(_, points)| points)
        .collect()
}

pub(crate) fn folds_eq(container1: &FoldContainer, container2: &FoldContainer) -> bool {
    folds_eq_by(
        container1,
        container2,
        |sfp1, sfp2| sfp1 == sfp2,
        |efp1, efp2| efp1 == efp2,
    )
}

pub(crate) fn folds_eq_by(
    container1: &FoldContainer,
    container2: &FoldContainer,
    sfp_eq: impl Fn(&StartFoldPoint, &StartFoldPoint) -> bool,
    efp_eq: impl Fn(&EndFoldPoint, &EndFoldPoint) -> bool,
) -> bool {
    if container1.len() != container2.len() {
        eprintln!(
            "left has lenght = {}\n\
            right has lenght = {}",
            container1.len(),
            container2.len(),
        );
        return false;
    }

    container1
        .start_points
        .iter()
        .zip(&container2.start_points)
        .enumerate()
        .all(|(i, (sfp1, sfp2))| {
            if sfp_eq(sfp1, sfp2) {
                return true;
            }

            eprintln!(
                "index = {i}\n\
                left sfp = {sfp1:#?}\n\
                right sfp = {sfp2:#?}"
            );
            false
        })
        && container1
            .end_points
            .iter()
            .zip(&container2.end_points)
            .enumerate()
            .all(|(i, (efp1, efp2))| {
                if efp_eq(efp1, efp2) {
                    return true;
                }

                eprintln!(
                    "index = {i}\n\
                    left efp = {efp1:#?}\n\
                    right efp = {efp2:#?}"
                );
                false
            })
}
