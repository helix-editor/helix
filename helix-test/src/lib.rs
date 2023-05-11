use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(path: PathBuf) -> anyhow::Result<Command> {
    let mut runner = None;
    let mut dir = path.clone();

    while runner.is_none() {
        if !dir.pop() {
            return Err(anyhow!("No viable test runner found"));
        }
        runner = get_runner(dir.as_path());
    }

    let command = runner.unwrap().get_command(path.as_path());
    Ok(command)
}

/// This method attempts to get a test runner for the current directory. It will usually use
/// file-based cues, such as the existence of Cargo.toml, to determine what test runner is
/// appropriate for the context.
fn get_runner(dir: &Path) -> Option<Box<dyn Runner>> {
    if let Some(cargo) = Cargo::detect(dir) {
        Some(Box::new(cargo))
    } else {
        None
    }
}

pub trait Runner {
    fn get_command(&self, path: &Path) -> Command;
}

struct Cargo {
    dir: PathBuf,
}

impl Cargo {
    fn detect(dir: &Path) -> Option<Cargo> {
        if dir.join("Cargo.toml").exists() {
            Some(Cargo{
                dir: dir.to_path_buf(),
            })
        } else {
            None
        }
    }
}

impl Runner for Cargo {
    fn get_command(&self, path: &Path) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.arg("test").arg(&path.strip_prefix(&self.dir).unwrap_or(path).as_os_str());
        cmd.current_dir(&self.dir);
        cmd
    }
}
