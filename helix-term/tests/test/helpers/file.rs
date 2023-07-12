use std::{
    fs::File,
    io::{Read, Write},
};
use tempfile::NamedTempFile;

pub fn temp_file_with_contents<S: AsRef<str>>(
    content: S,
) -> anyhow::Result<tempfile::NamedTempFile> {
    let mut temp_file = tempfile::NamedTempFile::new()?;

    temp_file
        .as_file_mut()
        .write_all(content.as_ref().as_bytes())?;

    temp_file.flush()?;
    temp_file.as_file_mut().sync_all()?;
    Ok(temp_file)
}

/// Creates a new temporary file that is set to read only. Useful for
/// testing write failures.
pub fn new_readonly_tempfile() -> anyhow::Result<NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new()?;
    let metadata = file.as_file().metadata()?;
    let mut perms = metadata.permissions();
    perms.set_readonly(true);
    file.as_file_mut().set_permissions(perms)?;
    Ok(file)
}

pub fn assert_file_has_content(file: &mut File, content: &str) -> anyhow::Result<()> {
    file.flush()?;
    file.sync_all()?;

    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;
    assert_eq!(content, file_content);

    Ok(())
}
