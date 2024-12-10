use std::path::Path;

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// An account based identity
pub struct Account {
    /// The user's name
    pub username: String,
    /// The user's password
    pub password: String,
}

/// Returns true if the given `path` is owned by the user who is executing the current process.
///
/// Note that this method is very specific to avoid having to deal with any operating system types.
pub fn is_path_owned_by_current_user(path: &Path) -> std::io::Result<bool> {
    impl_::is_path_owned_by_current_user(path)
}

// Wasi doesn't have a concept of a user, so this is implicitly true.
#[cfg(target_os = "wasi")]
mod impl_ {
    pub fn is_path_owned_by_current_user(_path: &std::path::Path) -> std::io::Result<bool> {
        Ok(true)
    }
}

#[cfg(all(not(windows), not(target_os = "wasi")))]
mod impl_ {
    use std::path::Path;

    pub fn is_path_owned_by_current_user(path: &Path) -> std::io::Result<bool> {
        fn owner_from_path(path: &Path) -> std::io::Result<u32> {
            use std::os::unix::fs::MetadataExt;
            let meta = std::fs::symlink_metadata(path)?;
            Ok(meta.uid())
        }

        fn owner_of_current_process() -> std::io::Result<u32> {
            // SAFETY: there is no documented possibility for failure
            #[allow(unsafe_code)]
            let uid = unsafe { libc::geteuid() };
            Ok(uid)
        }
        use std::str::FromStr;

        let owner_of_path = owner_from_path(path)?;
        let owner_of_process = owner_of_current_process()?;
        if owner_of_path == owner_of_process {
            Ok(true)
        } else if let Some(sudo_uid) =
            std::env::var_os("SUDO_UID").and_then(|val| val.to_str().and_then(|val_str| u32::from_str(val_str).ok()))
        {
            Ok(owner_of_path == sudo_uid)
        } else {
            Ok(false)
        }
    }
}

#[cfg(windows)]
mod impl_ {
    use std::{
        io,
        mem::MaybeUninit,
        os::windows::io::{FromRawHandle as _, OwnedHandle},
        path::Path,
        ptr,
    };

    macro_rules! error {
        ($msg:expr) => {{
            let inner = io::Error::last_os_error();
            error!(inner, $msg);
        }};
        ($inner:expr, $msg:expr) => {{
            return Err(io::Error::new($inner.kind(), $msg));
        }};
    }

    pub fn is_path_owned_by_current_user(path: &Path) -> io::Result<bool> {
        use windows_sys::Win32::{
            Foundation::{GetLastError, LocalFree, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS},
            Security::{
                Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT},
                CheckTokenMembership, EqualSid, GetTokenInformation, IsWellKnownSid, TokenOwner,
                WinBuiltinAdministratorsSid, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, TOKEN_OWNER,
                TOKEN_QUERY,
            },
            System::Threading::{GetCurrentProcess, GetCurrentThread, OpenProcessToken, OpenThreadToken},
        };

        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{path:?} does not exist."),
            ));
        }

        // Home is not actually owned by the corresponding user
        // but it can be considered de-facto owned by the user
        // Ignore errors here and just do the regular checks below
        if gix_path::realpath(path).ok() == gix_path::env::home_dir() {
            return Ok(true);
        }

        #[allow(unsafe_code)]
        unsafe {
            let (folder_owner, descriptor) = {
                let mut folder_owner = MaybeUninit::uninit();
                let mut pdescriptor = MaybeUninit::uninit();
                let result = GetNamedSecurityInfoW(
                    to_wide_path(path).as_ptr(),
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION,
                    folder_owner.as_mut_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    pdescriptor.as_mut_ptr(),
                );

                if result != ERROR_SUCCESS {
                    let inner = io::Error::from_raw_os_error(result as _);
                    error!(
                        inner,
                        format!(
                            "Couldn't get security information for path '{}' with err {inner}",
                            path.display()
                        )
                    );
                }

                (folder_owner.assume_init(), pdescriptor.assume_init())
            };

            struct Descriptor(PSECURITY_DESCRIPTOR);

            impl Drop for Descriptor {
                fn drop(&mut self) {
                    #[allow(unsafe_code)]
                    // SAFETY: syscall only invoked if we have a valid descriptor
                    unsafe {
                        LocalFree(self.0 as _);
                    }
                }
            }

            let _descriptor = Descriptor(descriptor);

            let token = {
                let mut token = MaybeUninit::uninit();

                // Use the current thread token if possible, otherwise open the process token
                if OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, token.as_mut_ptr()) == 0
                    && OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, token.as_mut_ptr()) == 0
                {
                    error!("Couldn't acquire thread or process token");
                }
                token.assume_init()
            };

            let _owned_token = OwnedHandle::from_raw_handle(token as _);

            let buf = 'token_buf: {
                let mut buffer_size = 36;
                let mut heap_buf = vec![0; 36];

                loop {
                    if GetTokenInformation(
                        token,
                        TokenOwner,
                        heap_buf.as_mut_ptr().cast(),
                        heap_buf.len() as _,
                        &mut buffer_size,
                    ) != 0
                    {
                        break 'token_buf heap_buf;
                    }

                    if GetLastError() != ERROR_INSUFFICIENT_BUFFER {
                        error!("Couldn't acquire token ownership");
                    }

                    heap_buf.resize(buffer_size as _, 0);
                }
            };

            let token_owner = (*buf.as_ptr().cast::<TOKEN_OWNER>()).Owner;

            // If the current user is the owner of the parent folder then they also
            // own this file
            if EqualSid(folder_owner, token_owner) != 0 {
                return Ok(true);
            }

            // Admin-group owned folders are considered owned by the current user, if they are in the admin group
            if IsWellKnownSid(folder_owner, WinBuiltinAdministratorsSid) == 0 {
                return Ok(false);
            }

            let mut is_member = 0;
            if CheckTokenMembership(0, folder_owner, &mut is_member) == 0 {
                error!("Couldn't check if user is an administrator");
            }

            Ok(is_member != 0)
        }
    }

    fn to_wide_path(path: impl AsRef<Path>) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        let mut wide_path: Vec<_> = path.as_ref().as_os_str().encode_wide().collect();
        wide_path.push(0);
        wide_path
    }
}
