#[test]
fn is_path_owned_by_current_user() -> crate::Result {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("file");
    std::fs::write(&file, [])?;
    assert!(gix_sec::identity::is_path_owned_by_current_user(&file)?);
    assert!(gix_sec::identity::is_path_owned_by_current_user(dir.path())?);
    Ok(())
}

/// Ownership checks intentionally inspect the symlink itself rather than following the target.
/// This matches Git's longstanding `lstat()`-based behavior, which treats a user-owned symlink as
/// owned by that user even if its target is owned by someone else.
#[test]
#[cfg(all(unix, not(target_os = "wasi")))]
fn symlink_ownership_checks_inspect_the_link_itself() -> crate::Result {
    use std::os::unix::fs as unix_fs;
    use std::os::unix::fs::MetadataExt;

    let current_uid = unsafe { libc::geteuid() };
    let Some(candidate) = ["/etc/passwd", "/etc/hosts", "/bin/sh", "/bin/ls", "/dev/null"]
        .into_iter()
        .map(std::path::Path::new)
        .find(|path| path.exists() && std::fs::metadata(path).is_ok_and(|meta| meta.uid() != current_uid))
    else {
        return Ok(());
    };

    let dir = tempfile::tempdir()?;
    let symlink = dir.path().join("trusted-link");
    unix_fs::symlink(candidate, &symlink)?;

    assert!(
        gix_sec::identity::is_path_owned_by_current_user(&symlink)?,
        "ownership checks intentionally trust the user-owned symlink itself, matching Git"
    );
    Ok(())
}

#[test]
#[cfg(windows)]
fn windows_home() -> crate::Result {
    let home = gix_path::env::home_dir().expect("home dir is available");
    assert!(gix_sec::identity::is_path_owned_by_current_user(&home)?);
    Ok(())
}

/// An administrator must not implicitly own every path on the machine.
///
/// `GIX_TEST_FOREIGN_OWNED_PATH` may name a path owned by neither the current user nor the
/// Administrators group. Without it, this test tries common Windows locations and runs against the
/// first suitable path. It remains a successful no-op when the environment provides no such path.
#[test]
#[cfg(windows)]
fn windows_foreign_owned_path_is_not_owned_by_current_user() -> crate::Result {
    use std::path::{Path, PathBuf};

    #[cfg(windows)]
    #[expect(unsafe_code, reason = "Windows ownership requires Win32 security APIs")]
    fn windows_owner_is_current_user_or_administrators(path: &std::path::Path) -> std::io::Result<bool> {
        use std::{
            mem::MaybeUninit,
            os::windows::{ffi::OsStrExt as _, io::FromRawHandle as _},
            ptr,
        };
        use windows_sys::Win32::{
            Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, GetLastError, LocalFree},
            Security::{
                Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT},
                EqualSid, GetTokenInformation, IsWellKnownSid, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
                TOKEN_QUERY, TOKEN_USER, TokenUser, WinBuiltinAdministratorsSid,
            },
            System::Threading::{GetCurrentProcess, GetCurrentThread, OpenProcessToken, OpenThreadToken},
        };

        let wide_path = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();

        unsafe {
            let mut path_owner = MaybeUninit::uninit();
            let mut descriptor = MaybeUninit::uninit();
            let result = GetNamedSecurityInfoW(
                wide_path.as_ptr(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION,
                path_owner.as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                descriptor.as_mut_ptr(),
            );
            if result != ERROR_SUCCESS {
                return Err(std::io::Error::from_raw_os_error(result as _));
            }
            let path_owner = path_owner.assume_init();

            struct Descriptor(PSECURITY_DESCRIPTOR);
            impl Drop for Descriptor {
                fn drop(&mut self) {
                    unsafe {
                        LocalFree(self.0.cast());
                    }
                }
            }
            let _descriptor = Descriptor(descriptor.assume_init());

            let mut token = ptr::null_mut();
            if OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token) == 0
                && OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            let _token = std::os::windows::io::OwnedHandle::from_raw_handle(token.cast());

            let mut buffer_size = 0;
            if GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut buffer_size) != 0
                || GetLastError() != ERROR_INSUFFICIENT_BUFFER
            {
                return Err(std::io::Error::last_os_error());
            }
            let mut user_info = vec![0; buffer_size as usize];
            if GetTokenInformation(
                token,
                TokenUser,
                user_info.as_mut_ptr().cast(),
                buffer_size,
                &mut buffer_size,
            ) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            let token_user_info = ptr::read_unaligned(user_info.as_ptr().cast::<TOKEN_USER>());
            let token_user = token_user_info.User.Sid;

            Ok(EqualSid(path_owner, token_user) != 0 || IsWellKnownSid(path_owner, WinBuiltinAdministratorsSid) != 0)
        }
    }

    fn assert_reduced(path: &Path) -> crate::Result {
        eprintln!(
            "checking independently verified foreign-owned path '{}'",
            path.display()
        );
        assert!(
            !gix_sec::identity::is_path_owned_by_current_user(path)?,
            "a path owned by neither the current user nor Administrators must receive reduced trust: '{}'",
            path.display()
        );
        Ok(())
    }

    let is_home_dir = |path: &Path| gix_path::realpath(path).ok() == gix_path::env::home_dir();

    if let Some(path) = std::env::var_os("GIX_TEST_FOREIGN_OWNED_PATH").map(PathBuf::from) {
        if !path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("GIX_TEST_FOREIGN_OWNED_PATH '{}' does not exist", path.display()),
            )
            .into());
        }
        if is_home_dir(&path) {
            return Ok(());
        }
        if windows_owner_is_current_user_or_administrators(&path)? {
            return Err(std::io::Error::other(format!(
                "GIX_TEST_FOREIGN_OWNED_PATH '{}' must be owned by neither the current user nor Administrators",
                path.display()
            ))
            .into());
        }
        return assert_reduced(&path);
    }

    let mut candidates = ["PUBLIC", "SystemRoot", "ProgramData", "ProgramFiles"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if let Some(system_drive) = std::env::var_os("SystemDrive") {
        candidates.push(PathBuf::from(system_drive).join("Users").join("Public"));
    }

    for path in candidates {
        if !path.exists() || is_home_dir(&path) {
            continue;
        }
        match windows_owner_is_current_user_or_administrators(&path) {
            Ok(false) => return assert_reduced(&path),
            Ok(true) => {}
            Err(err) => eprintln!("could not inspect ownership of '{}': {err}", path.display()),
        }
    }

    eprintln!(
        "no readable well-known path owned by neither the current user nor Administrators was available; \
         set GIX_TEST_FOREIGN_OWNED_PATH to exercise this regression test"
    );
    Ok(())
}
