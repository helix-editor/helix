//! From <https://github.com/Freaky/faccess>

use std::io;
use std::path::Path;

use filetime::FileTime;

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

    fn chown(p: &Path, uid: Option<u32>, gid: Option<u32>) -> anyhow::Result<()> {
        let uid = uid.map(|n| unsafe { rustix::fs::Uid::from_raw(n) });
        let gid = gid.map(|n| unsafe { rustix::fs::Gid::from_raw(n) });
        rustix::fs::chown(p, uid, gid)?;
        Ok(())
    }

    pub fn copy_metadata(from: &Path, to: &Path) -> anyhow::Result<()> {
        let meta = std::fs::File::open(from)?.metadata()?;
        let uid = meta.gid();
        let gid = meta.uid();
        chown(to, Some(uid), Some(gid))?;

        let mut perms = meta.permissions();
        let new_perms = (perms.mode() & 0o0707) | (perms.mode() & 0o07) << 3;
        perms.set_mode(new_perms);

        std::fs::set_permissions(to, perms)?;

        // TODO: Can be replaced by std::fs::FileTimes on 1.75
        let atime = FileTime::from_last_access_time(&meta);
        let mtime = FileTime::from_last_modification_time(&meta);
        filetime::set_file_times(to, atime, mtime)?;

        Ok(())
    }
}

// Licensed under MIT from faccess except for `chown` and `copy_metadata`
#[cfg(windows)]
mod imp {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, LocalFree, BOOL, HANDLE, HLOCAL, PSID};
    use windows::Win32::Security::Authorization::{
        GetNamedSecurityInfoW, SetNamedSecurityInfoW, SE_FILE_OBJECT,
    };
    use windows::Win32::Security::{
        AccessCheck, AclSizeInformation, GetAce, GetAclInformation, GetSidIdentifierAuthority,
        ImpersonateSelf, IsValidAcl, IsValidSid, MapGenericMask, RevertToSelf,
        SecurityImpersonation, ACCESS_ALLOWED_ACE, ACL, ACL_SIZE_INFORMATION,
        DACL_SECURITY_INFORMATION, GENERIC_MAPPING, GROUP_SECURITY_INFORMATION, INHERITED_ACE,
        LABEL_SECURITY_INFORMATION, OBJECT_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
        PRIVILEGE_SET, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
        SID_IDENTIFIER_AUTHORITY, TOKEN_DUPLICATE, TOKEN_QUERY,
    };
    use windows::Win32::Storage::FileSystem::{
        FILE_ACCESS_RIGHTS, FILE_ALL_ACCESS, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ,
        FILE_GENERIC_WRITE,
    };
    use windows::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};

    use super::*;

    use std::ffi::c_void;

    use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt};

    struct SecurityDescriptor {
        sd: PSECURITY_DESCRIPTOR,
        owner: PSID,
        group: PSID,
        dacl: *mut ACL,
    }

    impl Drop for SecurityDescriptor {
        fn drop(&mut self) {
            if !self.sd.0.is_null() {
                unsafe {
                    let _ = LocalFree(HLOCAL(self.sd.0));
                }
            }
        }
    }

    impl SecurityDescriptor {
        fn for_path(p: &Path) -> std::io::Result<SecurityDescriptor> {
            let path = std::fs::canonicalize(p)?;
            let pathos = path.into_os_string();
            let mut pathw: Vec<u16> = Vec::with_capacity(pathos.len() + 1);
            pathw.extend(pathos.encode_wide());
            pathw.push(0);

            let mut sd = PSECURITY_DESCRIPTOR::default();
            let mut owner = PSID::default();
            let mut group = PSID::default();
            let mut dacl = std::ptr::null_mut();

            unsafe {
                GetNamedSecurityInfoW(
                    PCWSTR::from_raw(pathw.as_ptr()),
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION
                        | GROUP_SECURITY_INFORMATION
                        | DACL_SECURITY_INFORMATION
                        | LABEL_SECURITY_INFORMATION,
                    Some(&mut owner),
                    Some(&mut group),
                    Some(&mut dacl),
                    None,
                    &mut sd,
                )?
            };

            Ok(SecurityDescriptor {
                sd,
                owner,
                group,
                dacl,
            })
        }

        fn is_acl_inherited(&self) -> std::io::Result<bool> {
            let mut acl_info = ACL_SIZE_INFORMATION::default();
            let acl_info_ptr: *mut c_void = &mut acl_info as *mut _ as *mut c_void;
            let mut ace = ACCESS_ALLOWED_ACE::default();

            // Causes access violation when dacl is null. Is that UB?
            unsafe {
                GetAclInformation(
                    self.dacl,
                    acl_info_ptr,
                    std::mem::size_of_val(&acl_info) as u32,
                    AclSizeInformation,
                )
            }?;

            for i in 0..acl_info.AceCount {
                // TODO: check casting and returning result
                let mut ptr = &mut ace as *mut _ as *mut c_void;
                unsafe { GetAce(self.dacl, i, &mut ptr) }?;
                if (ace.Header.AceFlags as u32 & INHERITED_ACE.0) != 0 {
                    return Ok(true);
                }
            }

            Ok(false)
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
                let _ = CloseHandle(self.0);
            }
        }
    }

    impl ThreadToken {
        fn new() -> io::Result<Self> {
            unsafe {
                ImpersonateSelf(SecurityImpersonation)?;
                let mut token = HANDLE::default();
                let err = OpenThreadToken(
                    GetCurrentThread(),
                    TOKEN_DUPLICATE | TOKEN_QUERY,
                    false,
                    &mut token,
                );

                RevertToSelf()?;

                err?;

                Ok(Self(token))
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
                .access_mode(mode.0)
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
            if IsValidSid(*owner).as_bool()
                && (*GetSidIdentifierAuthority(*owner)).Value == SAMBA_UNMAPPED.Value
            {
                return Ok(());
            }
        }

        let token = ThreadToken::new()?;

        let mut privileges: PRIVILEGE_SET = PRIVILEGE_SET::default();
        let mut granted_access: u32 = 0;
        let mut privileges_length = std::mem::size_of::<PRIVILEGE_SET>() as u32;
        let mut result = BOOL(0);

        let mut mapping = GENERIC_MAPPING {
            GenericRead: FILE_GENERIC_READ.0,
            GenericWrite: FILE_GENERIC_WRITE.0,
            GenericExecute: FILE_GENERIC_EXECUTE.0,
            GenericAll: FILE_ALL_ACCESS.0,
        };

        unsafe { MapGenericMask(&mut mode.0, &mut mapping) };

        unsafe {
            AccessCheck(
                *sd.descriptor(),
                *token.as_handle(),
                mode.0,
                &mut mapping,
                Some(&mut privileges),
                &mut privileges_length as *mut _,
                &mut granted_access as *mut _,
                &mut result,
            )?
        };
        if !result.as_bool() {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Permission Denied",
            ))
        } else {
            Ok(())
        }
    }

    pub fn access(p: &Path, mode: AccessMode) -> io::Result<()> {
        let mut imode = FILE_ACCESS_RIGHTS(0);

        if mode.contains(AccessMode::READ) {
            imode |= FILE_GENERIC_READ;
        }

        if mode.contains(AccessMode::WRITE) {
            imode |= FILE_GENERIC_WRITE;
        }

        if mode.contains(AccessMode::EXECUTE) {
            imode |= FILE_GENERIC_EXECUTE;
        }

        if imode.0 == 0 {
            if p.exists() {
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "Not Found"))
            }
        } else {
            eaccess(p, imode)
        }
    }

    fn chown(p: &Path, sd: SecurityDescriptor) -> std::io::Result<()> {
        let path = std::fs::canonicalize(p)?;
        let pathos = path.as_os_str();
        let mut pathw = Vec::with_capacity(pathos.len() + 1);
        pathw.extend(pathos.encode_wide());
        pathw.push(0);

        let mut owner = PSID::default();
        let mut group = PSID::default();
        let mut dacl = None;

        let mut si = OBJECT_SECURITY_INFORMATION::default();
        if unsafe { IsValidSid(sd.owner) }.as_bool() {
            si |= OWNER_SECURITY_INFORMATION;
            owner = sd.owner;
        }

        if unsafe { IsValidSid(sd.group) }.as_bool() {
            si |= GROUP_SECURITY_INFORMATION;
            group = sd.group;
        }

        if unsafe { IsValidAcl(sd.dacl) }.as_bool() {
            si |= DACL_SECURITY_INFORMATION;
            if !sd.is_acl_inherited()? {
                si |= PROTECTED_DACL_SECURITY_INFORMATION;
            }
            dacl = Some(sd.dacl as *const _);
        }

        unsafe {
            SetNamedSecurityInfoW(
                PCWSTR::from_raw(pathw.as_ptr()),
                SE_FILE_OBJECT,
                si,
                owner,
                group,
                dacl,
                None,
            )?;
        }

        Ok(())
    }

    pub fn copy_metadata(from: &Path, to: &Path) -> std::io::Result<()> {
        let sd = SecurityDescriptor::for_path(from)?;
        chown(to, sd)?;

        let meta = std::fs::File::open(from)?.metadata()?;
        let perms = meta.permissions();

        std::fs::set_permissions(to, perms)?;

        // TODO: Can be replaced by std::fs::FileTimes on 1.75
        let atime = FileTime::from_last_access_time(&meta);
        let mtime = FileTime::from_last_modification_time(&meta);
        filetime::set_file_times(to, atime, mtime)?;

        Ok(())
    }
}

// Licensed under MIT from faccess
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

    pub fn copy_metadata(_from: &path, _to: &Path) -> std::io::Result<()> {
        let meta = std::fs::File::open(from)?.metadata()?;
        let perms = meta.permissions();
        std::fs::set_permissions(to, perms)?;

        Ok(())
    }
}

pub fn readonly(p: &Path) -> bool {
    match imp::access(p, AccessMode::WRITE) {
        Ok(_) => false,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

pub fn copy_metadata(from: &Path, to: &Path) -> anyhow::Result<()> {
    imp::copy_metadata(from, to)?;
    Ok(())
}
