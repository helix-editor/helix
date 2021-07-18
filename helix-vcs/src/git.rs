use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use git2::{DiffOptions, IntoCString, Repository};

use crate::{LineChange, LineChanges};

pub struct Git {
    relative_path: PathBuf,
    repo: Repository,
    pub line_changes: Option<LineChanges>,
}
impl Git {
    pub fn from_path(filename: &Path) -> Option<Self> {
        if let Ok(repo) = Repository::discover(&filename) {
            let repo_path_absolute = fs::canonicalize(repo.workdir()?).ok()?;

            let relative_path = fs::canonicalize(&filename)
                .ok()?
                .strip_prefix(&repo_path_absolute)
                .ok()?
                .to_path_buf();
            Some(Git {
                repo,
                relative_path,
                line_changes: None,
            })
        } else {
            None
        }
    }

    /// Taken from https://github.com/sharkdp/bat/blob/master/src/diff.rs
    pub fn diff(&mut self) {
        let mut diff_options = DiffOptions::new();
        let pathspec = if let Ok(p) = self.relative_path.clone().into_c_string() {
            p
        } else {
            return;
        };
        diff_options.pathspec(pathspec);
        diff_options.context_lines(0);

        let diff = if let Ok(d) = self
            .repo
            .diff_index_to_workdir(None, Some(&mut diff_options))
        {
            d
        } else {
            return;
        };

        let mut line_changes: LineChanges = HashMap::new();

        let mark_section =
            |line_changes: &mut LineChanges, start: u32, end: i32, change: LineChange| {
                for line in start..=end as u32 {
                    line_changes.insert(line as usize, change);
                }
            };

        let _ = diff.foreach(
            &mut |_, _| true,
            None,
            Some(&mut |delta, hunk| {
                let path = delta.new_file().path().unwrap_or_else(|| Path::new(""));

                if self.relative_path != path {
                    return false;
                }

                let old_lines = hunk.old_lines();
                let new_start = hunk.new_start();
                let new_lines = hunk.new_lines();
                let new_end = (new_start + new_lines) as i32 - 1;

                if old_lines == 0 && new_lines > 0 {
                    mark_section(&mut line_changes, new_start, new_end, LineChange::Added);
                } else if new_lines == 0 && old_lines > 0 {
                    if new_start == 0 {
                        mark_section(&mut line_changes, 1, 1, LineChange::RemovedAbove);
                    } else {
                        mark_section(
                            &mut line_changes,
                            new_start,
                            new_start as i32,
                            LineChange::RemovedBelow,
                        );
                    }
                } else {
                    mark_section(&mut line_changes, new_start, new_end, LineChange::Modified);
                }

                true
            }),
            None,
        );

        self.line_changes = Some(line_changes);
    }
}
<<<<<<< HEAD
=======

>>>>>>> d1c25e5 (Shows line changes relative to VCS)
