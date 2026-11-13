#[cfg(windows)]
pub(crate) fn name(path: &std::path::Path) -> std::io::Result<std::ffi::OsString> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    crate::windows_path::pipe_name(path.as_os_str().encode_wide().collect())
        .map(|name| std::ffi::OsString::from_wide(&name))
}

#[cfg(test)]
mod tests {
    use crate::windows_path::pipe_name as wide_name;
    use std::io;

    #[test]
    fn windows_endpoint_uses_path_without_a_hash() -> gix_error::TestResult {
        for (input, expected) in [
            (
                r"C:/work/.git/fsmonitor--daemon.ipc",
                r"\\.\pipe\C_\work\.git\fsmonitor--daemon.ipc",
            ),
            (
                r"C:\work\.git\fsmonitor--daemon.ipc",
                r"\\.\pipe\C_\work\.git\fsmonitor--daemon.ipc",
            ),
            (
                r"\\?\C:\work\.git\fsmonitor--daemon.ipc",
                r"\\.\pipe\C_\work\.git\fsmonitor--daemon.ipc",
            ),
            (
                r"\\?\UNC\host\share\repo\.git\fsmonitor--daemon.ipc",
                r"\\.\pipe\\\host\share\repo\.git\fsmonitor--daemon.ipc",
            ),
            (
                r"\\host\share/repo/.git/fsmonitor--daemon.ipc",
                r"\\.\pipe\\\host\share\repo\.git\fsmonitor--daemon.ipc",
            ),
            (r"repo:dir/file", r"\\.\pipe\repo:dir\file"),
        ] {
            assert_eq!(
                wide_name(input.encode_utf16().collect())?,
                expected.encode_utf16().collect::<Vec<_>>(),
                "Windows naming follows Git's drive-colon and slash conversion without a trailing NUL"
            );
        }
        Ok(())
    }

    #[test]
    fn native_wide_units_are_preserved_and_nul_is_rejected() -> gix_error::TestResult {
        let mut path: Vec<_> = r"\\?\C:/".encode_utf16().collect();
        path.extend([0xd800, u16::from(b'/'), 0xdc00]);
        let mut expected: Vec<_> = r"\\.\pipe\C_\".encode_utf16().collect();
        expected.extend([0xd800, u16::from(b'\\'), 0xdc00]);
        assert_eq!(
            wide_name(path)?,
            expected,
            "unpaired surrogate units must survive the native path transformation exactly"
        );
        for path in ["\0C:/pipe", "C:/pipe\0suffix", "C:/pipe\0"] {
            assert_eq!(
                wide_name(path.encode_utf16().collect())
                    .expect_err("embedded or trailing NUL cannot name a native pipe")
                    .kind(),
                io::ErrorKind::InvalidInput,
                "NUL is rejected before any native API can truncate the pipe name"
            );
        }
        Ok(())
    }
}
