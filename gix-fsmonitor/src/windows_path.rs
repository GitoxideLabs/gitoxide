use std::io;

/// Change only the verbatim prefix; callers must check filesystem equivalence when it matters.
pub(crate) fn remove_verbatim_prefix(path: &mut Vec<u16>) {
    let verbatim: Vec<_> = "\\\\?\\".encode_utf16().collect();
    let verbatim_unc: Vec<_> = "\\\\?\\UNC\\".encode_utf16().collect();
    if path.starts_with(&verbatim_unc) {
        path.splice(..verbatim_unc.len(), "\\\\".encode_utf16());
    } else if path.starts_with(&verbatim) {
        path.drain(..verbatim.len());
    }
}

/// Derive Git's pipe spelling without a trailing NUL, preserving native wide path units.
pub(crate) fn pipe_name(mut path: Vec<u16>) -> io::Result<Vec<u16>> {
    if path.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in pipe name"));
    }
    remove_verbatim_prefix(&mut path);
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
    fn removing_verbatim_prefix_only_changes_the_prefix() {
        for (input, expected) in [
            (r"C:\work\repo", r"C:\work\repo"),
            (r"\\?\C:\work\repo", r"C:\work\repo"),
            (r"\\host\share\repo", r"\\host\share\repo"),
            (r"\\?\UNC\host\share\repo", r"\\host\share\repo"),
            (r"UNC\host\share\repo", r"UNC\host\share\repo"),
            (r"\\?\C:/work/../repo", r"C:/work/../repo"),
            ("", ""),
        ] {
            let mut path: Vec<_> = input.encode_utf16().collect();
            remove_verbatim_prefix(&mut path);
            assert_eq!(
                path,
                expected.encode_utf16().collect::<Vec<_>>(),
                "prefix removal does not normalize the path or reinterpret a relative UNC component"
            );
        }
    }

    #[test]
    fn windows_endpoint_uses_path_without_a_hash() -> gix_testtools::TestResult {
        for (input, expected) in [
            (
                r"C:\work\.git\fsmonitor--daemon.ipc",
                r"\\.\pipe\C_\work\.git\fsmonitor--daemon.ipc",
            ),
            (
                "C:/work/.git/fsmonitor--daemon.ipc",
                r"\\.\pipe\C_\work\.git\fsmonitor--daemon.ipc",
            ),
            (
                r"\\?\C:\work\.git\fsmonitor--daemon.ipc",
                r"\\.\pipe\C_\work\.git\fsmonitor--daemon.ipc",
            ),
            (
                r"\\host\share\repo\.git\fsmonitor--daemon.ipc",
                r"\\.\pipe\\\host\share\repo\.git\fsmonitor--daemon.ipc",
            ),
            (
                r"\\?\UNC\host\share\repo\.git\fsmonitor--daemon.ipc",
                r"\\.\pipe\\\host\share\repo\.git\fsmonitor--daemon.ipc",
            ),
        ] {
            assert_eq!(
                pipe_name(input.encode_utf16().collect())?,
                expected.encode_utf16().collect::<Vec<_>>(),
                "clients and servers share Git's drive-colon and slash conversion without a trailing NUL"
            );
        }
        Ok(())
    }

    #[test]
    fn native_wide_units_are_preserved() -> gix_testtools::TestResult {
        let mut path: Vec<_> = r"\\?\C:/".encode_utf16().collect();
        path.push(0xd800);
        path.extend("/ipc".encode_utf16());
        let mut expected: Vec<_> = r"\\.\pipe\C_\".encode_utf16().collect();
        expected.push(0xd800);
        expected.extend(r"\ipc".encode_utf16());
        assert_eq!(
            pipe_name(path)?,
            expected,
            "unpaired surrogates survive both prefix removal and pipe-name conversion"
        );
        Ok(())
    }

    #[test]
    fn pipe_names_reject_embedded_nul() {
        assert_eq!(
            pipe_name("C:/pipe\0suffix".encode_utf16().collect())
                .expect_err("native pipe names cannot contain NUL")
                .kind(),
            io::ErrorKind::InvalidInput,
            "reject NUL before a native API can truncate the name"
        );
    }
}
