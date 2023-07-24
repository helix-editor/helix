mod app_builder;
pub mod file;
pub mod test_harness;

pub use app_builder::AppBuilder;
use tokio::time::Instant;

use super::backend::TestBackend;
use derive_more::{Deref, DerefMut};
use helix_term::application::Application;
use std::{fs::File, io::Read, time::Duration};

const TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Deref, DerefMut)]
pub struct TestApplication(Application<TestBackend>);

impl TestApplication {
    /// Returns true if app exited
    pub async fn tick(&mut self) -> bool {
        loop {
            match tokio::time::timeout_at(Instant::now() + TIMEOUT, self.0.tick()).await {
                Ok(should_continue) => {
                    if !should_continue {
                        return true;
                    }
                }
                Err(_) => return false,
            }
        }
    }
}

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

pub fn assert_eq_contents(file: &mut File, str: &str, with_plattform_line: bool) {
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let expected = match with_plattform_line {
        true => platform_line(str),
        false => str.to_string(),
    };
    assert_eq!(expected, contents);
}
