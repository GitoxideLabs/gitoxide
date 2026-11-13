use std::io;

#[cfg(windows)]
pub(crate) fn name(path: &std::path::Path) -> io::Result<std::ffi::OsString> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    wide_name(path.as_os_str().encode_wide().collect()).map(|name| std::ffi::OsString::from_wide(&name))
}

fn wide_name(mut path: Vec<u16>) -> io::Result<Vec<u16>> {
    if path.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in pipe name"));
    }
    let verbatim: Vec<_> = "\\\\?\\".encode_utf16().collect();
    let verbatim_unc: Vec<_> = "\\\\?\\UNC\\".encode_utf16().collect();
    if path.starts_with(&verbatim_unc) {
        path.splice(..verbatim_unc.len(), "\\\\".encode_utf16());
    } else if path.starts_with(&verbatim) {
        path.drain(..verbatim.len());
    }
    if path.get(1) == Some(&u16::from(b':')) {
        path[1] = u16::from(b'_');
    }
    for unit in &mut path {
        if *unit == u16::from(b'/') {
            *unit = u16::from(b'\\');
        }
    }
    let mut name: Vec<_> = "\\\\.\\pipe\\".encode_utf16().collect();
    name.extend(path);
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

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
