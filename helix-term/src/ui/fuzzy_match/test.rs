use crate::ui::fuzzy_match::FuzzyQuery;
use crate::ui::fuzzy_match::Matcher;

fn run_test<'a>(query: &str, items: &'a [&'a str]) -> Vec<String> {
    let query = FuzzyQuery::new(query);
    let matcher = Matcher::default();
    items
        .iter()
        .filter_map(|item| {
            let (_, indicies) = query.fuzzy_indicies(item, &matcher)?;
            let matched_string = indicies
                .iter()
                .map(|&pos| item.chars().nth(pos).unwrap())
                .collect();
            Some(matched_string)
        })
        .collect()
}

#[test]
fn match_single_value() {
    let matches = run_test("foo", &["foobar", "foo", "bar"]);
    assert_eq!(matches, &["foo", "foo"])
}

#[test]
fn match_multiple_values() {
    let matches = run_test(
        "foo bar",
        &["foo bar", "foo   bar", "bar foo", "bar", "foo"],
    );
    assert_eq!(matches, &["foobar", "foobar", "barfoo"])
}

#[test]
fn space_escape() {
    let matches = run_test(r"foo\ bar", &["bar foo", "foo bar", "foobar"]);
    assert_eq!(matches, &["foo bar"])
}

#[test]
fn trim() {
    let matches = run_test(r" foo bar ", &["bar foo", "foo bar", "foobar"]);
    assert_eq!(matches, &["barfoo", "foobar", "foobar"]);
    let matches = run_test(r" foo bar\ ", &["bar foo", "foo bar", "foobar"]);
    assert_eq!(matches, &["bar foo"])
}
