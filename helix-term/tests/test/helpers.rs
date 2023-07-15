mod app_builder;
pub mod file;
pub mod test_harness;

pub use app_builder::AppBuilder;

use super::backend::TestBackend;
use helix_term::application::Application;
use std::{fs::File, io::Read};

pub type TestApplication = Application<TestBackend>;

/// Replaces all LF chars with the system's appropriate line feed
/// character, and if one doesn't exist already, appends the system's
/// appropriate line ending to the end of a string.
pub fn platform_line(input: &str) -> String {
    let line_end = helix_core::NATIVE_LINE_ENDING.as_str();

    // we can assume that the source files in this code base will always
    // be LF, so indoc strings will always insert LF
    let mut output = input.replace('\n', line_end);

    if !output.ends_with(line_end) {
        output.push_str(line_end);
    }

    output
}

pub fn assert_eq_contents(mut file: File, str: &str) {
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    assert_eq!(contents, platform_line(str));
}
