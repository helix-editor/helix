use anyhow::Context as _;
use anyhow::Result;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;

use super::{get_repo_dir, open_repo};

/// Allows us to save compute resources when requesting blame for the same line
/// To go from an `ObjectId` (which represents a commit) to `LineBLame`, we have to perform some work.
///
/// With this struct, we only do this work once. Getting `LineBlame` for the same line for the 2nd and subsequent
/// times is going to be free. This is important because we do this step on every render, in the main thread.
#[derive(Debug)]
enum LineBlameUnit {
    /// The raw object id of the commit for a line.
    /// It will take a bit of compute in order to obtain the `LineBlame` for it.
    Unprocessed(gix::ObjectId),
    /// Fully processed line blame information.
    Processed(LineBlame),
}

/// Stores information about the blame for a file
#[derive(Debug)]
pub struct FileBlame {
    /// A map from line numbers to blame for that line
    blame: Mutex<HashMap<u32, LineBlameUnit>>,
    /// The owning repository for this file's `ObjectId`s
    repo: gix::ThreadSafeRepository,
}

impl FileBlame {
    /// Get the blame information corresponding to a line in the file and diff for that line
    #[inline]
    pub fn blame_for_line(&self, line: u32, inserted_lines: u32, removed_lines: u32) -> LineBlame {
        // Because gix_blame doesn't care about stuff that is not commited, we have to "normalize" the
        // line number to account for uncommited code.
        //
        // You'll notice that blame_line can be 0 when, for instance we have:
        // - removed 0 lines
        // - added 10 lines
        // - cursor_line is 8
        //
        // So when our cursor is on the 10th added line or earlier, blame_line will be 0. This means
        // the blame will be incorrect. But that's fine, because when the cursor_line is on some hunk,
        // we can show to the user nothing at all. This is detected in the editor
        let blame_line = line.saturating_sub(inserted_lines) + removed_lines;
        let repo = self.repo.to_thread_local();

        let mut blame = self.blame.lock();
        let line_blame_unit = blame.get_mut(&blame_line);

        let commit = match line_blame_unit {
            // Slow path: This is the first time we're trying to get the blame for this line
            Some(LineBlameUnit::Unprocessed(object_id)) => repo.find_commit(*object_id).ok(),
            // Fast path: We've already processed this line before so we don't need to
            // search all of the repo's commits to compute the blame
            Some(LineBlameUnit::Processed(line_blame)) => return line_blame.clone(),
            // This line does not have any blame associated with it
            None => None,
        };

        let message = commit.as_ref().and_then(|c| c.message().ok());
        let author = commit.as_ref().and_then(|c| c.author().ok());
        let time = author.and_then(|a| a.time.parse::<gix::date::Time>().ok());

        let line_blame = LineBlame {
            commit_hash: commit
                .as_ref()
                .and_then(|c| c.short_id().map(|id| id.to_string()).ok()),
            author_name: author.map(|a| a.name.to_string()),
            author_email: author.map(|a| a.email.to_string()),
            commit_date: time.map(|time| time.format(gix::date::time::format::SHORT)),
            commit_title: message.as_ref().map(|msg| msg.title.to_string()),
            commit_body: message
                .as_ref()
                .and_then(|msg| msg.body.map(|body| body.to_string())),
            time_stamp: time.map(|time| (time.seconds, time.offset)),
            time_ago: None,
        };

        // we know that `line_blame_unit` here is not processed
        if let Some(line_blame_unit) = line_blame_unit {
            *line_blame_unit = LineBlameUnit::Processed(line_blame.clone());
        };

        line_blame
    }

    /// Compute blame for this file (expensive)
    pub fn try_new(file: PathBuf) -> Result<Self> {
        let thread_safe_repo =
            open_repo(get_repo_dir(&file)?).context("Failed to open git repo")?;
        let repo = thread_safe_repo.to_thread_local();
        let head = repo.head()?.peel_to_commit_in_place()?.id;

        let mut resource_cache = repo.diff_resource_cache_for_tree_diff()?;
        let file_blame = gix::blame::file(
            &repo.objects,
            head,
            None,
            &mut resource_cache,
            // NOTE: bstr always uses unix separators `/`, even on
            // Windows which uses the `\` separator
            &gix::path::to_unix_separators_on_windows(gix::path::try_into_bstr(
                file.strip_prefix(
                    repo.path()
                        .parent()
                        .context("Could not get the parent path of the repo")?,
                )?,
            )?),
            gix::blame::Options::default(),
        )?
        .entries;

        Ok(Self {
            blame: Mutex::new(
                file_blame
                    .into_iter()
                    .flat_map(|blame| {
                        (blame.start_in_blamed_file..blame.start_in_blamed_file + blame.len.get())
                            .map(move |i| (i, LineBlameUnit::Unprocessed(blame.commit_id)))
                    })
                    .collect(),
            ),
            repo: thread_safe_repo,
        })
    }
}

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct LineBlame {
    commit_hash: Option<String>,
    author_name: Option<String>,
    author_email: Option<String>,
    commit_date: Option<String>,
    commit_title: Option<String>,
    commit_body: Option<String>,
    /// Used to compute `time-ago`
    time_stamp: Option<(i64, i32)>,
    /// This field is the only one that needs to be re-computed every time
    /// we request the `LineBlame`. It exists here for lifetime purposes, so we can return
    /// `&str` from `Self::get_variable`.
    ///
    /// This should only be set from within and never initialized.
    time_ago: Option<String>,
}

impl LineBlame {
    /// Longest variable is: `time-ago` (and `message`)
    // this is just to reduce allocation by a little bit by specifying the max size we would expect a
    // variable to be up-front. This function is called every render.
    const LONGEST_VARIABLE_LENGTH: usize = 7;

    /// # Returns
    ///
    /// None => Invalid variable
    /// Some(None) => Valid variable, but is empty
    #[inline]
    fn get_variable(&mut self, var: &str) -> Option<Option<&str>> {
        Some(
            // if adding new variables, update `Self::LONGEST_VARIABLE_LENGTH`
            match var {
                "commit" => &self.commit_hash,
                "author" => &self.author_name,
                "date" => &self.commit_date,
                "title" => &self.commit_title,
                "email" => &self.author_email,
                "body" => &self.commit_body,
                "time-ago" => {
                    let time_ago = self.time_stamp.map(|(utc_seconds, timezone_offset)| {
                        helix_stdx::time::format_relative_time(utc_seconds, timezone_offset)
                    });
                    self.time_ago = time_ago;
                    &self.time_ago
                }
                _ => return None,
            }
            .as_deref(),
        )
    }

    /// Parse the user's blame format
    #[inline]
    pub fn parse_format(&mut self, format: &str) -> String {
        let mut line_blame = String::new();
        let mut content_before_variable = String::with_capacity(format.len());

        let mut chars = format.char_indices().peekable();
        // in all cases, when any of the variables is empty we exclude the content before the variable
        // However, if the variable is the first and it is empty - then exclude the content after the variable
        let mut exclude_content_after_variable = false;
        while let Some((ch_idx, ch)) = chars.next() {
            if ch == '{' {
                let mut variable = String::with_capacity(Self::LONGEST_VARIABLE_LENGTH);
                // eat all characters until the end
                while let Some((_, ch)) = chars.next_if(|(_, ch)| *ch != '}') {
                    variable.push(ch);
                }
                // eat the '}' if it was found
                let has_closing = chars.next().is_some();

                #[derive(PartialEq, Eq, PartialOrd, Ord)]
                enum Variable<'a> {
                    Valid(&'a str),
                    Invalid(&'a str),
                    Empty,
                }

                let variable_value = self.get_variable(&variable).map_or_else(
                    || {
                        // Invalid variable. So just add whatever we parsed before.
                        // The length of the variable, including opening and optionally
                        // closing curly braces
                        let variable_len = 1 + variable.len() + has_closing as usize;

                        Variable::Invalid(&format[ch_idx..ch_idx + variable_len])
                    },
                    |s| s.map(Variable::Valid).unwrap_or(Variable::Empty),
                );

                match variable_value {
                    Variable::Invalid(value) | Variable::Valid(value) => {
                        if exclude_content_after_variable {
                            // don't push anything.
                            exclude_content_after_variable = false;
                        } else {
                            line_blame.push_str(&content_before_variable);
                        }
                        line_blame.push_str(value);
                    }
                    Variable::Empty => {
                        if line_blame.is_empty() {
                            // exclude content AFTER this variable (at next iteration of the loop,
                            // we'll exclude the content before a valid variable)
                            exclude_content_after_variable = true;
                        } else {
                            // exclude content BEFORE this variable
                            // also just don't add anything.
                        }
                    }
                }

                // we've consumed the content before the variable so just get rid of it and
                // make space for new
                content_before_variable.drain(..);
            } else {
                content_before_variable.push(ch);
            }
        }

        line_blame
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::git::test::create_commit_with_message;
    use crate::git::test::empty_git_repo;
    use std::fs::File;

    /// describes how a line was modified
    #[derive(PartialEq, PartialOrd, Ord, Eq)]
    enum LineDiff {
        /// this line is added
        Insert,
        /// this line is deleted
        Delete,
        /// no changes for this line
        None,
    }

    /// checks if the first argument is `no_commit` or not
    macro_rules! no_commit_flag {
        (no_commit, $commit_msg:literal) => {
            false
        };
        (, $commit_msg:literal) => {
            true
        };
        ($any:tt, $commit_msg:literal) => {
            compile_error!(concat!(
                "expected `no_commit` or nothing for commit ",
                $commit_msg
            ))
        };
    }

    /// checks if the first argument is `insert` or `delete`
    macro_rules! line_diff_flag {
        (insert, $commit_msg:literal, $line:expr) => {
            LineDiff::Insert
        };
        (delete, $commit_msg:literal, $line:expr) => {
            LineDiff::Delete
        };
        (, $commit_msg:literal, $line:expr) => {
            LineDiff::None
        };
        ($any:tt, $commit_msg:literal, $line:expr) => {
            compile_error!(concat!(
                "expected `insert`, `delete` or nothing for commit ",
                $commit_msg,
                " line ",
                $line
            ))
        };
    }

    /// This macro exists because we can't pass a `match` statement into `concat!`
    /// we would like to exclude any lines that are `delete`
    macro_rules! line_diff_flag_str {
        (insert, $commit_msg:literal, $line:expr) => {
            concat!($line, newline_literal!())
        };
        (delete, $commit_msg:literal, $line:expr) => {
            ""
        };
        (, $commit_msg:literal, $line:expr) => {
            concat!($line, newline_literal!())
        };
        ($any:tt, $commit_msg:literal, $line:expr) => {
            compile_error!(concat!(
                "expected `insert`, `delete` or nothing for commit ",
                $commit_msg,
                " line ",
                $line
            ))
        };
    }

    #[cfg(windows)]
    macro_rules! newline_literal {
        () => {
            "\r\n"
        };
    }
    #[cfg(not(windows))]
    macro_rules! newline_literal {
        () => {
            "\n"
        };
    }

    /// Helper macro to create a history of the same file being modified.
    macro_rules! assert_line_blame_progress {
        (
            $(
                // a unique identifier for the commit, other commits must not use this
                // If `no_commit` option is used, use the identifier of the previous commit
                $commit_msg:literal
                // must be `no_commit` if exists.
                // If exists, this block won't be committed
                $($no_commit:ident)? =>
                $(
                    // contents of a line in the file
                    $line:literal
                    // what commit identifier we are expecting for this line
                    $($expected:literal)?
                    // must be `insert` or `delete` if exists
                    // if exists, must be used with `no_commit`
                    // - `insert`: this line is added
                    // - `delete`: this line is deleted
                    $($line_diff:ident)?
                ),+
            );+
            $(;)?
        ) => {{
            use std::fs::OpenOptions;
            use std::io::Write;

            let repo = empty_git_repo();
            let file = repo.path().join("file.txt");
            File::create(&file).expect("could not create file");

            $(
                let file_content = concat!(
                    $(
                        line_diff_flag_str!($($line_diff)?, $commit_msg, $line),
                    )*
                );
                eprintln!("at commit {}:\n\n{file_content}", stringify!($commit_msg));

                let mut f = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&file)
                    .unwrap();

                f.write_all(file_content.as_bytes()).unwrap();

                let should_commit = no_commit_flag!($($no_commit)?, $commit_msg);
                if should_commit {
                    create_commit_with_message(repo.path(), true, stringify!($commit_msg));
                }

                let mut line_number = 0;
                let mut added_lines = 0;
                let mut removed_lines = 0;

                $(
                    let line_diff_flag = line_diff_flag!($($line_diff)?, $commit_msg, $line);
                    #[allow(unused_assignments)]
                    match line_diff_flag {
                        LineDiff::Insert => added_lines += 1,
                        LineDiff::Delete => removed_lines += 1,
                        LineDiff::None => ()
                    }
                    // completely skip lines that are marked as `delete`
                    if line_diff_flag != LineDiff::Delete {
                        // if there is no $expected, then we don't care what blame_line returns
                        // because we won't show it to the user.
                        $(
                            let blame_result =
                                FileBlame::try_new(file.clone())
                                    .unwrap()
                                    .blame_for_line(line_number, added_lines, removed_lines)
                                    .commit_title;

                            assert_eq!(
                                blame_result,
                                Some(concat!(stringify!($expected), newline_literal!()).to_owned()),
                                "Blame mismatch\nat commit: {}\nat line: {}\nline contents: {}\nexpected commit: {}\nbut got commit: {}",
                                $commit_msg,
                                line_number,
                                file_content
                                    .lines()
                                    .nth(line_number.try_into().unwrap())
                                    .unwrap(),
                                stringify!($expected),
                                blame_result
                                    .as_ref()
                                    .map(|blame| blame.trim_end())
                                    .unwrap_or("<no commit>")
                            );
                        )?
                        #[allow(unused_assignments)]
                        {
                            line_number += 1;
                        }
                    }
                )*
            )*
        }};
    }

    // For some reasons the CI is failing on windows with the message "Commits not found".
    // The created temporary repository has no commits... But this is not an issue on unix.
    // There is nothing platform-specific in this implementation. This is a problem only
    // for tests on Windows.
    // As such it should be fine to disable this test in Windows.
    // As long as these tests pass on other platforms, on Windows it will work too.
    #[cfg(not(windows))]
    #[test]
    pub fn blamed_lines() {
        assert_line_blame_progress! {
            // initialize
            1 =>
                "fn main() {" 1,
                "" 1,
                "}" 1;
            // modifying a line works
            2 =>
                "fn main() {" 1,
                "  one" 2,
                "}" 1;
            // inserting a line works
            3 =>
                "fn main() {" 1,
                "  one" 2,
                "  two" 3,
                "}" 1;
            // deleting a line works
            4 =>
                "fn main() {" 1,
                "  two" 3,
                "}" 1;
            // when a line is inserted in-between the blame order is preserved
            4 no_commit =>
                "fn main() {" 1,
                "  hello world" insert,
                "  two" 3,
                "}" 1;
            // Having a bunch of random lines interspersed should not change which lines
            // have blame for which commits
            4 no_commit =>
                "  six" insert,
                "  three" insert,
                "fn main() {" 1,
                "  five" insert,
                "  four" insert,
                "  two" 3,
                "  five" insert,
                "  four" insert,
                "}" 1,
                "  five" insert,
                "  four" insert;
            // committing all of those insertions should recognize that they are
            // from the current commit, while still keeping the information about
            // previous commits
            5 =>
                "  six" 5,
                "  three" 5,
                "fn main() {" 1,
                "  five" 5,
                "  four" 5,
                "  two" 3,
                "  five" 5,
                "  four" 5,
                "}" 1,
                "  five" 5,
                "  four" 5;
            // several lines deleted
            5 no_commit =>
                "  six" 5,
                "  three" 5,
                "fn main() {" delete,
                "  five" delete,
                "  four" delete,
                "  two" delete,
                "  five" delete,
                "  four" 5,
                "}" 1,
                "  five" 5,
                "  four" 5;
            // committing the deleted changes
            6 =>
                "  six" 5,
                "  three" 5,
                "  four" 5,
                "}" 1,
                "  five" 5,
                "  four" 5;
            // mixing inserts with deletes
            6 no_commit =>
                "  six" delete,
                "  2" insert,
                "  three" delete,
                "  four" 5,
                "  1" insert,
                "}" 1,
                "]" insert,
                "  five" delete,
                "  four" 5;
            // committing inserts and deletes
            7 =>
                "  2" 7,
                "  four" 5,
                "  1" 7,
                "}" 1,
                "]" 7,
                "  four" 5;
        };
    }

    fn bob() -> LineBlame {
        LineBlame {
            commit_hash: Some("f14ab1cf".to_owned()),
            author_name: Some("Bob TheBuilder".to_owned()),
            author_email: Some("bob@bob.com".to_owned()),
            commit_date: Some("2028-01-10".to_owned()),
            commit_title: Some("feat!: extend house".to_owned()),
            commit_body: Some("BREAKING CHANGE: Removed door".to_owned()),
            time_stamp: None,
            time_ago: None,
        }
    }

    #[test]
    pub fn inline_blame_format_parser() {
        let format = "{author}, {date} • {title} • {commit}";

        assert_eq!(
            bob().parse_format(format),
            "Bob TheBuilder, 2028-01-10 • feat!: extend house • f14ab1cf".to_owned()
        );
        assert_eq!(
            LineBlame {
                author_name: None,
                ..bob()
            }
            .parse_format(format),
            "2028-01-10 • feat!: extend house • f14ab1cf".to_owned()
        );
        assert_eq!(
            LineBlame {
                commit_date: None,
                ..bob()
            }
            .parse_format(format),
            "Bob TheBuilder • feat!: extend house • f14ab1cf".to_owned()
        );
        assert_eq!(
            LineBlame {
                commit_title: None,
                author_email: None,
                ..bob()
            }
            .parse_format(format),
            "Bob TheBuilder, 2028-01-10 • f14ab1cf".to_owned()
        );
        assert_eq!(
            LineBlame {
                commit_hash: None,
                ..bob()
            }
            .parse_format(format),
            "Bob TheBuilder, 2028-01-10 • feat!: extend house".to_owned()
        );
        assert_eq!(
            LineBlame {
                commit_date: None,
                author_name: None,
                ..bob()
            }
            .parse_format(format),
            "feat!: extend house • f14ab1cf".to_owned()
        );
        assert_eq!(
            LineBlame {
                author_name: None,
                commit_title: None,
                ..bob()
            }
            .parse_format(format),
            "2028-01-10 • f14ab1cf".to_owned()
        );
    }
}
