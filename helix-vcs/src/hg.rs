use anyhow::{bail, Context, Result};
use arc_swap::ArcSwap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::process::Command;

use crate::DiffProvider;

#[cfg(test)]
mod test;

pub struct Hg;

fn exec_hg_cmd_raw(bin: &str, args: &str, root: Option<&str>) -> Result<Vec<u8>> {
    let mut cmd = Command::new(bin);

    cmd.env("HGPLAIN", "").env("HGRCPATH", "");

    if let Some(dir) = root {
        cmd.arg("--cwd").arg(dir);
    }

    cmd.args(args.split_whitespace());

    match cmd.output() {
        Ok(result) => Ok(result.stdout),
        Err(e) => bail!("`hg {args}` failed: {}", e),
    }
}

fn exec_hg_cmd(bin: &str, args: &str, root: Option<&str>) -> Result<String> {
    match exec_hg_cmd_raw(bin, args, root) {
        Ok(result) => {
            Ok(String::from_utf8(result).context("Failed to parse output of `hg {args}`")?)
        }
        Err(e) => Err(e),
    }
}

impl Hg {
    fn get_repo_root(path: &Path) -> Result<PathBuf> {
        if path.is_symlink() {
            bail!("ignoring symlinks");
        };

        let workdir = if path.is_dir() {
            path
        } else {
            path.parent().context("path has no parent")?
        };

        match exec_hg_cmd("rhg", "root", workdir.to_str()) {
            Ok(output) => {
                let root = output
                    .strip_suffix("\n")
                    .or(output.strip_suffix("\r\n"))
                    .unwrap_or(output.as_str());

                if root.is_empty() {
                    bail!("did not find root")
                };

                let arg = format!("files {}", path.to_str().unwrap());
                match exec_hg_cmd("rhg", &arg, Some(root)) {
                    Ok(output) => {
                        let tracked = output
                            .strip_suffix("\n")
                            .or(output.strip_suffix("\r\n"))
                            .unwrap_or(output.as_str());

                        if (output.len() > 0)
                            && (Path::new(tracked) == path.strip_prefix(root).unwrap())
                        {
                            Ok(Path::new(&root).to_path_buf())
                        } else {
                            bail!("not a tracked file")
                        }
                    }
                    Err(_) => bail!("not a tracked file"),
                }
            }
            Err(_) => bail!("not in a hg repo"),
        }
    }
}

impl DiffProvider for Hg {
    fn get_diff_base(&self, file: &Path) -> Result<Vec<u8>> {
        debug_assert!(!file.exists() || file.is_file());
        debug_assert!(file.is_absolute());

        let root = Hg::get_repo_root(file).context("not a hg repo")?;

        let arg = format!("cat --rev=. {}", file.to_str().unwrap());
        let content =
            exec_hg_cmd_raw("rhg", &arg, root.to_str()).context("could not get file content")?;

        Ok(content)
    }

    fn get_current_head_name(&self, file: &Path) -> Result<Arc<ArcSwap<Box<str>>>> {
        debug_assert!(!file.exists() || file.is_file());
        debug_assert!(file.is_absolute());

        let root = Hg::get_repo_root(file).context("not a hg repo")?;

        let branch = exec_hg_cmd(
            "hg",
            "--config extensions.evolve= log --rev=wdir() --template={branch}",
            root.to_str(),
        )
        .context("could not get branch name")?;
        Ok(Arc::new(ArcSwap::from_pointee(branch.into_boxed_str())))
    }
}
