use std::path::Path;

pub mod env;
pub mod faccess;
pub mod path;
pub mod rope;

#[cfg(unix)]
pub fn get_hardlink_count(p: &Path) -> std::io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    let metadata = p.metadata()?;
    Ok(metadata.nlink())
}

#[cfg(windows)]
pub fn get_hardlink_count(p: &Path) -> std::io::Result<u64> {
    use std::os::windows::io::IntoRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let file = std::fs::File::open(p)?;
    let handle = file.into_raw_handle() as isize;
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };

    if unsafe { GetFileInformationByHandle(handle, &mut info) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(info.nNumberOfLinks as u64)
    }
}
