use std::path::Path;

/// Credentials with a complete pair of authentication fields.
///
/// Username/password authentication and pre-encoded credentials are mutually exclusive.
/// Transports validate the field contents according to their protocol's requirements.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Credentials {
    /// Authenticate with a username and password.
    UsernamePassword {
        /// The user's name.
        username: String,
        /// The user's password.
        password: String,
        /// An OAuth refresh token that may accompany the password. Treat it confidentially, just like the password.
        oauth_refresh_token: Option<String>,
    },
    /// Authenticate with a pre-encoded credential rather than a username and password.
    Encoded {
        /// The HTTP authentication scheme, such as `Bearer`.
        authtype: String,
        /// The secret, pre-encoded credential for `authtype`.
        credential: String,
        /// An OAuth refresh token that may accompany the credential. Treat it confidentially, just like the credential.
        oauth_refresh_token: Option<String>,
    },
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
            #[expect(unsafe_code)]
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
        io, mem,
        mem::MaybeUninit,
        os::windows::{
            fs::OpenOptionsExt as _,
            io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle},
        },
        path::Path,
        ptr,
    };

    macro_rules! error {
        ($msg:expr) => {{
            let inner = io::Error::last_os_error();
            error!(inner, $msg);
        }};
        ($inner:expr, $msg:expr) => {{
            return Err(io::Error::new($inner.kind(), format!("{}: {}", $msg, $inner)));
        }};
    }

    fn token_information(
        token: windows_sys::Win32::Foundation::HANDLE,
        class: i32,
        class_name: &'static str,
        subject: &'static str,
        path: &Path,
    ) -> io::Result<Vec<u8>> {
        use windows_sys::Win32::{
            Foundation::{ERROR_INSUFFICIENT_BUFFER, GetLastError},
            Security::GetTokenInformation,
        };

        #[expect(unsafe_code)]
        unsafe {
            let mut buffer_size = 36;
            let mut heap_buf = vec![0; 36];

            loop {
                if GetTokenInformation(
                    token,
                    class,
                    heap_buf.as_mut_ptr().cast(),
                    heap_buf.len() as _,
                    &mut buffer_size,
                ) != 0
                {
                    return Ok(heap_buf);
                }

                if GetLastError() != ERROR_INSUFFICIENT_BUFFER {
                    error!(format!(
                        "Couldn't acquire {class_name} for the {subject} while checking ownership of \"{}\"",
                        path.display()
                    ));
                }

                heap_buf.resize(buffer_size as _, 0);
            }
        }
    }

    /// Read a fixed-size token information record of type `T` with `GetTokenInformation`.
    ///
    /// Use this for token information classes whose result fits exactly into `T`.
    fn fixed_size_token_information<T: Copy>(
        token: windows_sys::Win32::Foundation::HANDLE,
        class: i32,
        class_name: &'static str,
        subject: &'static str,
        path: &Path,
    ) -> io::Result<T> {
        use windows_sys::Win32::Security::GetTokenInformation;

        #[expect(unsafe_code)]
        unsafe {
            let mut info = MaybeUninit::<T>::uninit();
            let mut returned_size = 0;
            if GetTokenInformation(
                token,
                class,
                info.as_mut_ptr().cast(),
                mem::size_of::<T>() as u32,
                &mut returned_size,
            ) == 0
            {
                error!(format!(
                    "Couldn't acquire {class_name} for the {subject} while checking ownership of \"{}\"",
                    path.display()
                ));
            }
            Ok(info.assume_init())
        }
    }

    pub fn is_path_owned_by_current_user(path: &Path) -> io::Result<bool> {
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("\"{}\" does not exist.", path.display()),
            ));
        }

        let ownership = is_path_owned_by_current_token(path);
        if matches!(ownership, Ok(true)) {
            return ownership;
        }

        // Home may be owned by someone else but is considered de-facto owned by the user.
        // Resolve aliases only as a fallback, avoiding a component-by-component filesystem scan
        // for the usual case of paths already owned by the current token.
        if let Some(home) = gix_path::env::home_dir()
            && gix_path::realpath(path).is_ok_and(|resolved| resolved == home)
        {
            return Ok(true);
        }
        ownership
    }

    fn is_path_owned_by_current_token(path: &Path) -> io::Result<bool> {
        use windows_sys::Win32::{
            Foundation::{ERROR_INVALID_FUNCTION, ERROR_SUCCESS, LocalFree},
            Security::{
                Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
                CheckTokenMembership, EqualSid, IsWellKnownSid, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
                TOKEN_ELEVATION_TYPE, TOKEN_LINKED_TOKEN, TOKEN_QUERY, TOKEN_USER, TokenElevationType,
                TokenElevationTypeLimited, TokenLinkedToken, TokenUser, WinBuiltinAdministratorsSid,
            },
            Storage::FileSystem::{FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, READ_CONTROL},
            System::Threading::{GetCurrentProcess, GetCurrentThread, OpenProcessToken, OpenThreadToken},
        };

        // Let Rust handle Windows path normalization and extended-length prefixes. Inspect the
        // final symlink itself, like GetNamedSecurityInfoW, and allow opening directories too.
        let file = match std::fs::OpenOptions::new()
            .access_mode(READ_CONTROL)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
        {
            Ok(file) => file,
            Err(err) if err.raw_os_error() == Some(ERROR_INVALID_FUNCTION as _) => return Ok(false),
            Err(err) => error!(
                err,
                format!("Couldn't open path \"{}\" to check ownership", path.display())
            ),
        };

        #[expect(unsafe_code)]
        unsafe {
            let (folder_owner, descriptor) = {
                let mut folder_owner = MaybeUninit::uninit();
                let mut pdescriptor = MaybeUninit::uninit();
                let result = GetSecurityInfo(
                    file.as_raw_handle(),
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION,
                    folder_owner.as_mut_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    pdescriptor.as_mut_ptr(),
                );

                if result != ERROR_SUCCESS {
                    if result == ERROR_INVALID_FUNCTION {
                        // We cannot obtain security information, so we default to reduced trust
                        // (false) rather than failing completely.
                        return Ok(false);
                    }
                    let inner = io::Error::from_raw_os_error(result as _);
                    error!(
                        inner,
                        format!("Couldn't get security information for path \"{}\"", path.display())
                    );
                }

                (folder_owner.assume_init(), pdescriptor.assume_init())
            };

            struct Descriptor(PSECURITY_DESCRIPTOR);

            impl Drop for Descriptor {
                fn drop(&mut self) {
                    #[expect(unsafe_code)]
                    // SAFETY: syscall only invoked if we have a valid descriptor
                    unsafe {
                        LocalFree(self.0.cast());
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
                    error!(format!(
                        "Couldn't acquire a thread or process token while checking ownership of \"{}\"",
                        path.display()
                    ));
                }
                token.assume_init()
            };

            let _owned_token = OwnedHandle::from_raw_handle(token.cast());

            let user_info_buf = token_information(token, TokenUser, "TokenUser", "current token", path)?;
            let token_user_info = ptr::read_unaligned(user_info_buf.as_ptr().cast::<TOKEN_USER>());
            let token_user = token_user_info.User.Sid;

            if EqualSid(folder_owner, token_user) != 0 {
                return Ok(true);
            }

            // Admin-group owned folders are considered owned by the current user, if they are in the admin group.
            if IsWellKnownSid(folder_owner, WinBuiltinAdministratorsSid) == 0 {
                return Ok(false);
            }

            let mut is_member = 0;
            if CheckTokenMembership(std::ptr::null_mut(), folder_owner, &mut is_member) == 0 {
                error!(format!(
                    "Couldn't check whether the current token is in the Administrators group while checking ownership of \"{}\"",
                    path.display()
                ));
            }

            if is_member != 0 {
                return Ok(true);
            }

            let elevation_type = fixed_size_token_information::<TOKEN_ELEVATION_TYPE>(
                token,
                TokenElevationType,
                "TokenElevationType",
                "current token",
                path,
            )?;
            if elevation_type != TokenElevationTypeLimited {
                return Ok(false);
            }

            let linked_token_info = fixed_size_token_information::<TOKEN_LINKED_TOKEN>(
                token,
                TokenLinkedToken,
                "TokenLinkedToken",
                "limited current token",
                path,
            )?;
            let linked_token = linked_token_info.LinkedToken;
            let linked_token = OwnedHandle::from_raw_handle(linked_token.cast());

            let mut is_member = 0;
            if CheckTokenMembership(linked_token.as_raw_handle(), folder_owner, &mut is_member) == 0 {
                error!(format!(
                    "Couldn't check whether the linked elevated token is in the Administrators group while checking ownership of \"{}\"",
                    path.display()
                ));
            }

            Ok(is_member != 0)
        }
    }
}
