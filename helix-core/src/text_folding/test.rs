use crate::graphemes::next_grapheme_boundary;

use super::*;

use test_utils::new_fold_points;
use test_utils::TEXT_SAMPLE;
use test_utils::{fold_points, fold_points_filtered_by};
use test_utils::{folds_eq, folds_eq_by};

#[test]
fn fold_container_from() {
    let mut points = fold_points();
    // additional points will be removed
    points.extend(
        [("rm", 73, 77..=77)]
            .into_iter()
            .map(|(object, header_line, target_lines)| {
                new_fold_points(*TEXT_SAMPLE, object, header_line, target_lines)
            }),
    );

    let container = FoldContainer::from(*TEXT_SAMPLE, points.clone());

    let partial_eq = |sfp1: &StartFoldPoint, sfp2: &StartFoldPoint| -> bool {
        &sfp1.object == &sfp2.object && sfp1.header == sfp2.header && sfp1.target == sfp2.target
    };
    assert!(container.start_points.iter().enumerate().all(|(i, sfp)| {
        let (expected, _) = &points[i];
        if partial_eq(&sfp, expected) {
            return true;
        }
        eprintln!(
            "index = {i}\n\
            sfp = {sfp:#?}\n\
            expected = {expected:#?}"
        );
        false
    }));

    let partial_eq =
        |efp1: &EndFoldPoint, efp2: &EndFoldPoint| -> bool { efp1.target == efp2.target };
    assert!(container.end_points.iter().enumerate().all(|(i, efp)| {
        let (_, expected) = &points[efp.link];
        if partial_eq(&efp, expected) {
            return true;
        }
        eprintln!(
            "index = {i}\n\
            efp = {efp:#?}\n\
            expected = {expected:#?}"
        );
        false
    }));
}

#[test]
fn fold_container_add() {
    let mut points = fold_points();
    points.extend([]);

    let container = &mut FoldContainer::from(
        *TEXT_SAMPLE,
        points
            .iter()
            .cloned()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, points)| points)
            .collect(),
    );
    container.add(
        *TEXT_SAMPLE,
        points
            .iter()
            .cloned()
            .enumerate()
            .filter(|(i, _)| i % 2 != 0)
            .map(|(_, points)| points)
            .collect(),
    );

    let expected = &FoldContainer::from(*TEXT_SAMPLE, points);
    assert!(folds_eq(container, expected));
}

#[test]
fn fold_container_replace() {
    // replacements, replaced
    let cases = [
        (&[0, 1][..], &[][..]),
        (&[2][..], &[3, 4, 5, 6, 7, 8][..]),
        (&[9][..], &[10, 11][..]),
        (&[12][..], &[13][..]),
        (&[14][..], &[15][..]),
        (&[19][..], &[16, 17, 18][..]),
    ];

    for (case_idx, (replacements, replaced)) in cases.into_iter().enumerate() {
        let container = &mut FoldContainer::from(
            *TEXT_SAMPLE,
            fold_points_filtered_by(|(i, _)| !replacements.contains(i)),
        );
        container.replace(
            *TEXT_SAMPLE,
            fold_points_filtered_by(|(i, _)| replacements.contains(i)),
        );

        let expected = &FoldContainer::from(
            *TEXT_SAMPLE,
            fold_points_filtered_by(|(i, _)| !replaced.contains(i)),
        );

        assert!(
            folds_eq_by(
                container,
                expected,
                |sfp1, sfp2| sfp1 == sfp2,
                |efp1, efp2| efp1.link == efp2.link,
            ),
            "case index = {case_idx}"
        );
    }
}

#[test]
fn fold_container_remove_by_selection() {
    // line from, line to, removed
    let cases = [
        (0, 0, &[][..]),
        (2, 3, &[][..]),
        (5, 6, &[][..]),
        (6, 7, &[][..]),
        (8, 8, &[2][..]),
        (17, 19, &[2, 4][..]),
        (21, 34, &[2, 5, 6, 9, 10, 11][..]),
        (40, 42, &[12][..]),
        (45, 55, &[12, 13, 15][..]),
    ];

    for (case_idx, (from, to, removed)) in cases.into_iter().enumerate() {
        let selection = &Selection::single(
            TEXT_SAMPLE.line_to_char(from),
            next_grapheme_boundary(*TEXT_SAMPLE, TEXT_SAMPLE.line_to_char(to)),
        );

        let container = &mut FoldContainer::from(*TEXT_SAMPLE, fold_points());
        container.remove_by_selection(*TEXT_SAMPLE, selection);

        let expected = &FoldContainer::from(
            *TEXT_SAMPLE,
            fold_points_filtered_by(|(i, _)| !removed.contains(i)),
        );

        assert!(folds_eq(container, expected), "case index = {case_idx}");
    }
}

#[test]
fn fold_container_throw_range_out_of_folds() {
    let container = &FoldContainer::from(*TEXT_SAMPLE, fold_points());

    // line from, line to, expected (line from, line to)
    let cases = [
        ((1, 1), Range::new(0, 16)),       // (0, 0)
        ((4, 4), Range::new(34, 50)),      // (3, 3)
        ((1, 4), Range::new(0, 50)),       // (0, 3)
        ((19, 63), Range::new(67, 827)),   // (6, 62)
        ((44, 10), Range::new(576, 67)),   // (39, 6)
        ((77, 45), Range::new(1009, 558)), // (72, 39)
    ];

    for (case_idx, ((from, to), expected)) in cases.into_iter().enumerate() {
        let range = Range::new(
            TEXT_SAMPLE.line_to_char(from),
            line_end_char_index(&*TEXT_SAMPLE, to),
        );

        let result = container.throw_range_out_of_folds(*TEXT_SAMPLE, range);
        let expected = expected.with_direction(result.direction());

        assert_eq!(result, expected, "case index = {case_idx}");
    }
}

#[test]
fn fold_container_find() {
    let container = &FoldContainer::from(*TEXT_SAMPLE, fold_points());

    // object, block line range, expected
    let cases = [
        ("0", 1..=1, Some(0)),
        ("a", 1..=1, None),
        ("0", 1..=2, None),
        ("7", 28..=29, Some(7)),
        ("6", 20..=22, Some(6)),
        ("2", 8..=29, Some(2)),
        ("10", 33..=35, Some(10)),
    ];

    for (case_idx, (object, block, expected)) in cases.into_iter().enumerate() {
        let result = container.find(&FoldObject::TextObject(object), &block, |fold| {
            fold.start.line..=fold.end.line
        });
        let expected = expected.map(|idx| container.start_points[idx].fold(container));
        assert_eq!(result, expected, "case index = {case_idx}");
    }
}

#[test]
fn fold_container_start_points_in_range() {
    let container = &FoldContainer::from(*TEXT_SAMPLE, fold_points());

    // block line range, expected
    let cases = [
        (0..=0, None),
        (6..=40, Some(2..=11)),
        (10..=15, Some(3..=3)),
        (55..=70, Some(16..=19)),
        (0..=9, Some(0..=2)),
    ];

    for (case_idx, (block, expected)) in cases.into_iter().enumerate() {
        let result = container.start_points_in_range(&block, |sfp| sfp.line);
        let expected = expected.map_or(&[][..], |range| &container.start_points[range]);
        assert_eq!(result, expected, "case index = {case_idx}");
    }
}

#[test]
fn fold_container_fold_containing() {
    let container = &FoldContainer::from(*TEXT_SAMPLE, fold_points());

    // line, expected
    let cases = [
        (0, None),
        (1, Some(0)),
        (7, None),
        (11, Some(3)),
        (9, Some(2)),
        (57, None),
        (78, Some(21)),
        (12, Some(2)),
        (19, Some(2)),
    ];

    for (case_idx, (line, expected)) in cases.into_iter().enumerate() {
        let result = container.fold_containing(line, |fold| fold.start.line..=fold.end.line);
        let expected = expected.map(|idx| container.start_points[idx].fold(container));
        assert_eq!(result, expected, "case index = {case_idx}");
    }
}

#[test]
fn fold_container_superest_fold_containing() {
    let container = &FoldContainer::from(*TEXT_SAMPLE, fold_points());

    // line, expected
    let cases = [
        (0, None),
        (1, Some(0)),
        (7, None),
        (11, Some(2)),
        (9, Some(2)),
        (57, None),
        (78, Some(21)),
        (12, Some(2)),
        (19, Some(2)),
    ];

    for (case_idx, (line, expected)) in cases.into_iter().enumerate() {
        let result =
            container.superest_fold_containing(line, |fold| fold.start.line..=fold.end.line);
        let expected = expected.map(|idx| container.start_points[idx].fold(container));
        assert_eq!(result, expected, "case index = {case_idx}");
    }
}

#[test]
fn fold_annotations_folded_lines_between() {
    let container = &FoldContainer::from(*TEXT_SAMPLE, fold_points());
    let annotations = FoldAnnotations::new(Some(container));

    // line range, expected
    let cases = [
        (0..=0, 0),
        (3..=3, 0),
        (0..=3, 1),
        (0..=5, 2),
        (5..=7, 0),
        (5..=30, 22),
        (30..=31, 0),
        (30..=51, 13),
        (51..=51, 0),
        (62..=79, 5),
    ];

    for (case_idx, (line_range, expected)) in cases.into_iter().enumerate() {
        let result = annotations.folded_lines_between(&line_range);
        assert_eq!(result, expected, "case index = {case_idx}");
    }
}

#[test]
fn fold_container_update_by_transaction() {
    use crate::Rope;
    use crate::Transaction;
    use std::cell::RefCell;
    use std::iter::once;

    let init_container = &FoldContainer::from(*TEXT_SAMPLE, fold_points());
    let container = RefCell::new(FoldContainer::from(*TEXT_SAMPLE, fold_points()));

    let object_eq = |fold: Fold, object: &str| {
        matches!(
            fold.object(),
            FoldObject::TextObject(textobject) if *textobject == object
        )
    };

    let decrease_eq = |n: usize| n == init_container.len() - container.borrow().len();

    // a change, an assert function
    let cases: Vec<(_, Box<dyn Fn()>)> = vec![
        (
            // remove the first header char
            (0, 1, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[0].fold(&container);

                assert!(
                    fold.header() == 0 && object_eq(fold, "0") && decrease_eq(0),
                    "fold = {fold:#?}"
                );
            }),
        ),
        (
            // replace the text "丂 line index: " from the 0i line
            (0, 15, Some("new header".into())),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[0].fold(&container);

                assert!(object_eq(fold, "0") && decrease_eq(0), "fold = {fold:#?}");
            }),
        ),
        (
            // replace the trimmed 0i line
            (0, 16, Some("new header".into())),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[0].fold(&container);

                assert!(object_eq(fold, "1") && decrease_eq(1), "fold = {fold:#?}");
            }),
        ),
        (
            // remove the entire 0i line
            (0, 17, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[0].fold(&container);

                assert!(object_eq(fold, "1") && decrease_eq(1), "fold = {fold:#?}");
            }),
        ),
        (
            // remove the first nonwhitespace char of 11i line
            (137, 138, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[3].fold(&container);

                assert!(object_eq(fold, "4") && decrease_eq(1), "fold = {fold:#?}");
            }),
        ),
        (
            // remove the last nonwhitespace char of the 19i line
            (263, 264, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[4].fold(&container);

                assert!(object_eq(fold, "5") && decrease_eq(1), "fold = {fold:#?}");
            }),
        ),
        (
            // remove the 33i entire line
            (486, 504, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[9].fold(&container);

                assert!(object_eq(fold, "9") && decrease_eq(2), "fold = {fold:#?}");
            }),
        ),
        (
            // remove the last nonwhitespace char of the 18i line
            (263, 264, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[4].fold(&container);

                assert!(
                    object_eq(fold, "5") && fold.start.line == 19 && decrease_eq(1),
                    "fold = {fold:#?}"
                );
            }),
        ),
        (
            // remove the 9i entire line
            (117, 136, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[3].fold(&container);

                assert!(object_eq(fold, "3") && decrease_eq(0), "fold = {fold:#?}");
            }),
        ),
        (
            // replace the text "19 乪\n\t" of the 19i-20i lines
            (279, 285, Some("new text\n\t".into())),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[6].fold(&container);

                assert!(object_eq(fold, "6") && decrease_eq(0), "fold = {fold:#?}");
            }),
        ),
        (
            // replace the text "19 乪\n\t\tline" of the 19i-20i lines
            (279, 292, Some("new text\n\t\tnew text".into())),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[6].fold(&container);

                assert!(object_eq(fold, "7") && decrease_eq(1), "fold = {fold:#?}");
            }),
        ),
        (
            // remove the line ending of the 55i line and 56i-57i lines
            (737, 740, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[15].fold(&container);

                assert!(
                    object_eq(fold, "15") && decrease_eq(0) && fold.end.line == 54,
                    "fold = {fold:#?}"
                );
            }),
        ),
        (
            // remove the line ending of the 33i line
            (502, 503, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[11].fold(&container);

                assert!(
                    object_eq(fold, "11") && decrease_eq(0) && fold.end.line == 34,
                    "fold = {fold:#?}"
                )
            }),
        ),
        (
            // remove the entire 39i-40i lines
            (558, 576, None),
            Box::new(|| {
                let container = container.borrow();
                let fold = container.start_points[12].fold(&container);

                assert!(
                    object_eq(fold, "13") && decrease_eq(1) && fold.is_superest(),
                    "fold = {fold:#?}"
                )
            }),
        ),
    ];

    for (change, assert) in cases {
        let doc = &mut Rope::from(*TEXT_SAMPLE);
        // reset container
        *container.borrow_mut() = init_container.clone();

        let transaction = &Transaction::change(doc, once(change));
        transaction.apply(doc);
        // update container
        container
            .borrow_mut()
            .update_by_transaction(doc.slice(..), *TEXT_SAMPLE, transaction);

        assert();
    }
}
