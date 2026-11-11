#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(super) use unix::{Listener, Wake, connect};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(super) use windows::{Listener, Wake, connect};

#[cfg(any(windows, test))]
fn pipe_name(mut path: Vec<u16>) -> Vec<u16> {
    let verbatim: Vec<_> = "\\\\?\\".encode_utf16().collect();
    let verbatim_unc: Vec<_> = "\\\\?\\UNC\\".encode_utf16().collect();
    if path.starts_with(&verbatim_unc) {
        path.splice(..verbatim_unc.len(), "\\\\".encode_utf16());
    } else if path.starts_with(&verbatim) {
        path.drain(..verbatim.len());
    }
    if path.get(1) == Some(&(b':' as u16)) {
        path[1] = b'_' as u16;
    }
    for unit in &mut path {
        if *unit == b'/' as u16 {
            *unit = b'\\' as u16;
        }
    }
    let mut name: Vec<_> = "\\\\.\\pipe\\".encode_utf16().collect();
    name.extend(path);
    name.push(0);
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_endpoint_uses_path_without_a_hash() {
        for (input, expected) in [
            (
                "C:/work/.git/fsmonitor--daemon.ipc",
                "\\\\.\\pipe\\C_\\work\\.git\\fsmonitor--daemon.ipc\0",
            ),
            (
                "\\\\?\\C:\\work\\.git\\fsmonitor--daemon.ipc",
                "\\\\.\\pipe\\C_\\work\\.git\\fsmonitor--daemon.ipc\0",
            ),
            (
                "\\\\?\\UNC\\host\\share\\repo\\.git\\fsmonitor--daemon.ipc",
                "\\\\.\\pipe\\\\\\host\\share\\repo\\.git\\fsmonitor--daemon.ipc\0",
            ),
        ] {
            assert_eq!(
                pipe_name(input.encode_utf16().collect()),
                expected.encode_utf16().collect::<Vec<_>>(),
                "Windows naming follows Git's drive-colon and slash conversion"
            );
        }
        let mut units: Vec<_> = "C:/".encode_utf16().collect();
        units.push(0xd800);
        assert!(
            pipe_name(units).contains(&0xd800),
            "native wide path units are preserved"
        );
    }
}
