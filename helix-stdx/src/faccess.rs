//! Functions for managine file metadata.
//! From <https://github.com/Freaky/faccess>

use std::io;
use std::path::Path;

use bitflags::bitflags;

// Licensed under MIT from faccess
bitflags! {
    /// Access mode flags for `access` function to test for.
    pub struct AccessMode: u8 {
        /// Path exists
        const EXISTS  = 0b0001;
        /// Path can likely be read
        const READ    = 0b0010;
        /// Path can likely be written to
        const WRITE   = 0b0100;
        /// Path can likely be executed
        const EXECUTE = 0b1000;
    }
}

#[cfg(unix)]
mod imp {
    use super::*;

    use rustix::fs::Access;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    pub fn access(p: &Path, mode: AccessMode) -> io::Result<()> {
        let mut imode = Access::empty();

        if mode.contains(AccessMode::EXISTS) {
            imode |= Access::EXISTS;
        }

        if mode.contains(AccessMode::READ) {
            imode |= Access::READ_OK;
        }

        if mode.contains(AccessMode::WRITE) {
            imode |= Access::WRITE_OK;
        }

        if mode.contains(AccessMode::EXECUTE) {
            imode |= Access::EXEC_OK;
        }

        rustix::fs::access(p, imode)?;
        Ok(())
    }

    pub fn chown(p: &Path, uid: Option<u32>, gid: Option<u32>) -> io::Result<()> {
        let uid = uid.map(rustix::fs::Uid::from_raw);
        let gid = gid.map(rustix::fs::Gid::from_raw);
        rustix::fs::chown(p, uid, gid)?;
        Ok(())
    }

    pub fn fchown(
        fd: impl std::os::fd::AsFd,
        uid: Option<u32>,
        gid: Option<u32>,
    ) -> io::Result<()> {
        let uid = uid.map(rustix::fs::Uid::from_raw);
        let gid = gid.map(rustix::fs::Gid::from_raw);
        rustix::fs::fchown(fd, uid, gid)?;
        Ok(())
    }

    pub fn copy_metadata(from: &Path, to: &Path) -> io::Result<()> {
        let from_meta = std::fs::metadata(from)?;
        let to_meta = std::fs::metadata(to)?;
        let from_gid = from_meta.gid();
        let to_gid = to_meta.gid();

        let mut perms = from_meta.permissions();
        perms.set_mode(perms.mode() & 0o0777);
        if from_gid != to_gid && chown(to, None, Some(from_gid)).is_err() {
            let new_perms = (perms.mode() & 0o0707) | ((perms.mode() & 0o07) << 3);
            perms.set_mode(new_perms);
        }

        #[cfg(target_os = "macos")]
        {
            use std::fs::{File, FileTimes};
            use std::os::macos::fs::FileTimesExt;

            let to_file = File::options().write(true).open(to)?;
            let times = FileTimes::new().set_created(from_meta.created()?);
            to_file.set_times(times)?;
        }

        std::fs::set_permissions(to, perms)?;

        Ok(())
    }

    pub fn hardlink_count(p: &Path) -> std::io::Result<u64> {
        let metadata = p.metadata()?;
        Ok(metadata.nlink())
    }

    pub fn copy_xattr(src: &Path, dst: &Path) -> io::Result<()> {
        use std::ffi::CStr;

        if !src.exists() || !dst.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "src or dst file was not found while copying attributes",
            ));
        }

        let size = match rustix::fs::listxattr::<_, &mut [u8]>(src, &mut [])? {
            0 => return Ok(()), // No attributes
            len => len,
        };

        let mut key_list = vec![0; size];
        let size = rustix::fs::listxattr(src, key_list.as_mut_slice())?;
        if key_list.len() != size {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "`{}`'s xattr list changed while copying attributes",
                    src.to_string_lossy()
                ),
            ));
        }

        // Iterate over null-terminated C-style strings
        // Two loops to avoid multiple allocations
        // Find max-size for attributes
        let mut max_val_len = 0;
        for key in key_list[..size].split_inclusive(|&b| b == 0) {
            // Needed on macos
            #[allow(clippy::unnecessary_cast)]
            let conv = unsafe { std::slice::from_raw_parts(key.as_ptr() as *const u8, key.len()) };
            let key = CStr::from_bytes_with_nul(conv)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            let attr_len = rustix::fs::getxattr::<_, _, &mut [u8]>(src, key, &mut [])?;
            max_val_len = max_val_len.max(attr_len);
        }

        let mut attr_buf = vec![0u8; max_val_len];
        for key in key_list[..size].split_inclusive(|&b| b == 0) {
            // Needed on macos
            #[allow(clippy::unnecessary_cast)]
            let conv = unsafe { std::slice::from_raw_parts(key.as_ptr() as *const u8, key.len()) };
            let key = CStr::from_bytes_with_nul(conv)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let read = rustix::fs::getxattr(src, key, attr_buf.as_mut_slice())?;

            // If we can't set xattr because it already exists, try to replace it
            if read != 0 {
                match rustix::fs::setxattr(
                    dst,
                    key,
                    &attr_buf[..read],
                    rustix::fs::XattrFlags::CREATE,
                ) {
                    Err(rustix::io::Errno::EXIST) => rustix::fs::setxattr(
                        dst,
                        key,
                        &attr_buf[..read],
                        rustix::fs::XattrFlags::REPLACE,
                    )?,
                    Err(e) => return Err(e.into()),
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

// Licensed under MIT from faccess except for `chown`, `copy_metadata` and `is_acl_inherited`
#[cfg(windows)]
mod imp {

    use windows_sys::Win32::Foundation::{
        CloseHandle, LocalFree, ERROR_SUCCESS, GENERIC_READ, GENERIC_WRITE, HANDLE,
    };
    use windows_sys::Win32::Security::Authorization::{
        GetNamedSecurityInfoW, SetSecurityInfo, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        AccessCheck, AclSizeInformation, GetAce, GetAclInformation, GetSidIdentifierAuthority,
        ImpersonateSelf, IsValidAcl, IsValidSid, MapGenericMask, RevertToSelf,
        SecurityImpersonation, ACCESS_ALLOWED_CALLBACK_ACE, ACL, ACL_SIZE_INFORMATION,
        DACL_SECURITY_INFORMATION, GENERIC_MAPPING, GROUP_SECURITY_INFORMATION, INHERITED_ACE,
        LABEL_SECURITY_INFORMATION, OBJECT_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
        PRIVILEGE_SET, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
        SID_IDENTIFIER_AUTHORITY, TOKEN_DUPLICATE, TOKEN_QUERY,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ACCESS_RIGHTS,
        FILE_ALL_ACCESS, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, WRITE_DAC,
        WRITE_OWNER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};

    use super::*;

    use std::ffi::c_void;

    use std::os::windows::{
        ffi::OsStrExt,
        fs::{FileTimesExt, OpenOptionsExt},
        io::AsRawHandle,
    };

    use std::fs::{File, FileTimes};

    struct SecurityDescriptor {
        sd: PSECURITY_DESCRIPTOR,
        owner: PSID,
        group: PSID,
        dacl: *mut ACL,
    }

    impl Drop for SecurityDescriptor {
        fn drop(&mut self) {
            if !self.sd.is_null() {
                unsafe {
                    LocalFree(self.sd);
                }
            }
        }
    }

    impl SecurityDescriptor {
        fn for_path(p: &Path) -> io::Result<SecurityDescriptor> {
            let path = std::fs::canonicalize(p)?;
            let pathos = path.into_os_string();
            let mut pathw: Vec<u16> = Vec::with_capacity(pathos.len() + 1);
            pathw.extend(pathos.encode_wide());
            pathw.push(0);

            let mut sd = std::ptr::null_mut();
            let mut owner = std::ptr::null_mut();
            let mut group = std::ptr::null_mut();
            let mut dacl = std::ptr::null_mut();

            let err = unsafe {
                GetNamedSecurityInfoW(
                    pathw.as_ptr(),
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION
                        | GROUP_SECURITY_INFORMATION
                        | DACL_SECURITY_INFORMATION
                        | LABEL_SECURITY_INFORMATION,
                    &mut owner,
                    &mut group,
                    &mut dacl,
                    std::ptr::null_mut(),
                    &mut sd,
                )
            };

            if err == ERROR_SUCCESS {
                Ok(SecurityDescriptor {
                    sd,
                    owner,
                    group,
                    dacl,
                })
            } else {
                Err(io::Error::last_os_error())
            }
        }

        fn is_acl_inherited(&self) -> bool {
            let mut acl_info: ACL_SIZE_INFORMATION = unsafe { ::core::mem::zeroed() };
            let acl_info_ptr: *mut c_void = &mut acl_info as *mut _ as *mut c_void;
            let mut ace: ACCESS_ALLOWED_CALLBACK_ACE = unsafe { ::core::mem::zeroed() };

            unsafe {
                GetAclInformation(
                    self.dacl,
                    acl_info_ptr,
                    std::mem::size_of_val(&acl_info) as u32,
                    AclSizeInformation,
                )
            };

            for i in 0..acl_info.AceCount {
                let mut ptr = &mut ace as *mut _ as *mut c_void;
                unsafe { GetAce(self.dacl, i, &mut ptr) };
                if (ace.Header.AceFlags as u32 & INHERITED_ACE) != 0 {
                    return true;
                }
            }

            false
        }

        fn descriptor(&self) -> &PSECURITY_DESCRIPTOR {
            &self.sd
        }

        fn owner(&self) -> &PSID {
            &self.owner
        }
    }

    struct ThreadToken(HANDLE);
    impl Drop for ThreadToken {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    impl ThreadToken {
        fn new() -> io::Result<Self> {
            unsafe {
                if ImpersonateSelf(SecurityImpersonation) == 0 {
                    return Err(io::Error::last_os_error());
                }

                let token: *mut HANDLE = std::ptr::null_mut();
                let err =
                    OpenThreadToken(GetCurrentThread(), TOKEN_DUPLICATE | TOKEN_QUERY, 0, token);

                RevertToSelf();

                if err == 0 {
                    return Err(io::Error::last_os_error());
                }

                Ok(Self(*token))
            }
        }

        fn as_handle(&self) -> &HANDLE {
            &self.0
        }
    }

    // Based roughly on Tcl's NativeAccess()
    // https://github.com/tcltk/tcl/blob/2ee77587e4dc2150deb06b48f69db948b4ab0584/win/tclWinFile.c
    fn eaccess(p: &Path, mut mode: FILE_ACCESS_RIGHTS) -> io::Result<()> {
        let md = p.metadata()?;

        if !md.is_dir() {
            // Read Only is ignored for directories
            if mode & FILE_GENERIC_WRITE == FILE_GENERIC_WRITE && md.permissions().readonly() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "File is read only",
                ));
            }

            // If it doesn't have the correct extension it isn't executable
            if mode & FILE_GENERIC_EXECUTE == FILE_GENERIC_EXECUTE {
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    match ext {
                        "exe" | "com" | "bat" | "cmd" => (),
                        _ => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "File not executable",
                            ))
                        }
                    }
                }
            }

            return std::fs::OpenOptions::new()
                .access_mode(mode)
                .open(p)
                .map(|_| ());
        }

        let sd = SecurityDescriptor::for_path(p)?;

        // Unmapped Samba users are assigned a top level authority of 22
        // ACL tests are likely to be misleading
        const SAMBA_UNMAPPED: SID_IDENTIFIER_AUTHORITY = SID_IDENTIFIER_AUTHORITY {
            Value: [0, 0, 0, 0, 0, 22],
        };
        unsafe {
            let owner = sd.owner();
            if IsValidSid(*owner) != 0
                && (*GetSidIdentifierAuthority(*owner)).Value == SAMBA_UNMAPPED.Value
            {
                return Ok(());
            }
        }

        let token = ThreadToken::new()?;

        let mut privileges: PRIVILEGE_SET = unsafe { std::mem::zeroed() };
        let mut granted_access: u32 = 0;
        let mut privileges_length = std::mem::size_of::<PRIVILEGE_SET>() as u32;
        let mut result = 0;

        let mapping = GENERIC_MAPPING {
            GenericRead: FILE_GENERIC_READ,
            GenericWrite: FILE_GENERIC_WRITE,
            GenericExecute: FILE_GENERIC_EXECUTE,
            GenericAll: FILE_ALL_ACCESS,
        };

        unsafe { MapGenericMask(&mut mode, &mapping) };

        if unsafe {
            AccessCheck(
                *sd.descriptor(),
                *token.as_handle(),
                mode,
                &mapping,
                &mut privileges,
                &mut privileges_length,
                &mut granted_access,
                &mut result,
            )
        } != 0
        {
            if result == 0 {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Permission Denied",
                ))
            } else {
                Ok(())
            }
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub fn access(p: &Path, mode: AccessMode) -> io::Result<()> {
        let mut imode = 0;

        if mode.contains(AccessMode::READ) {
            imode |= FILE_GENERIC_READ;
        }

        if mode.contains(AccessMode::WRITE) {
            imode |= FILE_GENERIC_WRITE;
        }

        if mode.contains(AccessMode::EXECUTE) {
            imode |= FILE_GENERIC_EXECUTE;
        }

        if imode == 0 {
            if p.exists() {
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "Not Found"))
            }
        } else {
            eaccess(p, imode)
        }
    }

    // SAFETY: It is the caller's responsibility to close the handle
    fn chown(handle: HANDLE, sd: SecurityDescriptor) -> io::Result<()> {
        let mut owner = std::ptr::null_mut();
        let mut group = std::ptr::null_mut();
        let mut dacl = std::ptr::null();

        let mut si = OBJECT_SECURITY_INFORMATION::default();
        if unsafe { IsValidSid(sd.owner) } != 0 {
            si |= OWNER_SECURITY_INFORMATION;
            owner = sd.owner;
        }

        if unsafe { IsValidSid(sd.group) } != 0 {
            si |= GROUP_SECURITY_INFORMATION;
            group = sd.group;
        }

        if unsafe { IsValidAcl(sd.dacl) } != 0 {
            si |= DACL_SECURITY_INFORMATION;
            if !sd.is_acl_inherited() {
                si |= PROTECTED_DACL_SECURITY_INFORMATION;
            }
            dacl = sd.dacl as *const _;
        }

        let err = unsafe {
            SetSecurityInfo(
                handle,
                SE_FILE_OBJECT,
                si,
                owner,
                group,
                dacl,
                std::ptr::null(),
            )
        };

        if err == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub fn copy_ownership(from: &Path, to: &Path) -> io::Result<()> {
        let sd = SecurityDescriptor::for_path(from)?;
        let to_file = std::fs::OpenOptions::new()
            .read(true)
            .access_mode(GENERIC_READ | GENERIC_WRITE | WRITE_OWNER | WRITE_DAC)
            .open(to)?;
        chown(to_file.as_raw_handle(), sd)?;
        Ok(())
    }

    pub fn copy_metadata(from: &Path, to: &Path) -> io::Result<()> {
        copy_ownership(from, to)?;

        let meta = std::fs::metadata(from)?;
        let perms = meta.permissions();

        let to_file = File::options().write(true).open(to)?;
        let times = FileTimes::new().set_created(meta.created()?);
        to_file.set_times(times)?;

        std::fs::set_permissions(to, perms)?;

        Ok(())
    }

    pub fn hardlink_count(p: &Path) -> std::io::Result<u64> {
        let file = std::fs::File::open(p)?;
        let handle = file.as_raw_handle();
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };

        if unsafe { GetFileInformationByHandle(handle, &mut info) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(info.nNumberOfLinks as u64)
        }
    }
}

// Licensed under MIT from faccess except for `copy_metadata`
#[cfg(not(any(unix, windows)))]
mod imp {
    use super::*;

    pub fn access(p: &Path, mode: AccessMode) -> io::Result<()> {
        if mode.contains(AccessMode::WRITE) {
            if std::fs::metadata(p)?.permissions().readonly() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Path is read only",
                ));
            } else {
                return Ok(());
            }
        }

        if p.exists() {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"))
        }
    }

    pub fn copy_metadata(from: &path, to: &Path) -> io::Result<()> {
        let meta = std::fs::metadata(from)?;
        let perms = meta.permissions();
        std::fs::set_permissions(to, perms)?;

        Ok(())
    }
}

#[cfg(unix)]
pub fn chown(p: &Path, uid: Option<u32>, gid: Option<u32>) -> io::Result<()> {
    imp::chown(p, uid, gid)
}

#[cfg(unix)]
pub fn fchown(fd: impl std::os::fd::AsFd, uid: Option<u32>, gid: Option<u32>) -> io::Result<()> {
    imp::fchown(fd, uid, gid)
}

#[cfg(unix)]
pub fn copy_xattr(src: &Path, dst: &Path) -> io::Result<()> {
    imp::copy_xattr(src, dst)
}

pub fn readonly(p: &Path) -> bool {
    match imp::access(p, AccessMode::WRITE) {
        Ok(_) => false,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

pub fn copy_metadata(from: &Path, to: &Path) -> io::Result<()> {
    imp::copy_metadata(from, to)
}

pub fn hardlink_count(p: &Path) -> io::Result<u64> {
    imp::hardlink_count(p)
}
