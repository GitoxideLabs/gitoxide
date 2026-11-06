use std::{
    ffi::OsString,
    io,
    os::windows::ffi::OsStringExt,
    path::{Component, Path, PathBuf},
};

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ACTION_ADDED, FILE_ACTION_MODIFIED, FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME,
    FILE_ACTION_RENAMED_OLD_NAME,
};

use crate::{Event, EventKind, PathKind};

/// Decode the initialized portion of a `ReadDirectoryChangesW` buffer without assuming alignment.
pub(super) fn events(directory: &Path, mut bytes: &[u8]) -> io::Result<Vec<Event>> {
    let mut events = Vec::new();
    loop {
        // FILE_NOTIFY_INFORMATION has three DWORDs before its variable-length UTF-16 name.
        let header = bytes.get(..12).ok_or_else(invalid)?;
        let word = |offset| {
            u32::from_le_bytes([
                header[offset],
                header[offset + 1],
                header[offset + 2],
                header[offset + 3],
            ])
        };
        let next = word(0) as usize;
        let kind = match word(4) {
            FILE_ACTION_ADDED => EventKind::Create,
            FILE_ACTION_MODIFIED => EventKind::Modify,
            FILE_ACTION_REMOVED => EventKind::Remove,
            FILE_ACTION_RENAMED_OLD_NAME | FILE_ACTION_RENAMED_NEW_NAME => EventKind::Rename,
            _ => return Err(invalid()),
        };
        let length = word(8) as usize;
        if length == 0 || !length.is_multiple_of(2) {
            return Err(invalid());
        }
        let end = 12usize.checked_add(length).ok_or_else(invalid)?;
        let name = bytes.get(12..end).ok_or_else(invalid)?;
        let wide: Vec<_> = name
            .as_chunks::<2>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes(*unit))
            .collect();
        if wide.contains(&0) {
            return Err(invalid());
        }
        let path = PathBuf::from(OsString::from_wide(&wide));
        if !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(invalid());
        }
        events.push(Event {
            paths: vec![directory.join(path)],
            kind,
            // The record has no object type, and a filesystem lookup would race the change.
            path_kind: PathKind::Any,
        });
        if next == 0 {
            return Ok(events);
        }
        if next < end {
            return Err(invalid());
        }
        bytes = bytes.get(next..).ok_or_else(invalid)?;
    }
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid Windows directory notification record",
    )
}

#[cfg(test)]
mod tests {
    use std::os::windows::ffi::OsStrExt;

    use super::*;

    fn record(action: u32, name: &[u16]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(0u32.to_le_bytes());
        out.extend(action.to_le_bytes());
        out.extend((name.len() as u32 * 2).to_le_bytes());
        out.extend(name.iter().flat_map(|unit| unit.to_le_bytes()));
        out
    }

    #[test]
    fn decodes_each_action_and_preserves_rename_endpoints() -> gix_error::TestResult {
        let directory = Path::new(r"C:\watched");
        let mut bytes = Vec::new();
        let expected = [
            (FILE_ACTION_ADDED, EventKind::Create),
            (FILE_ACTION_MODIFIED, EventKind::Modify),
            (FILE_ACTION_REMOVED, EventKind::Remove),
            (FILE_ACTION_RENAMED_OLD_NAME, EventKind::Rename),
            (FILE_ACTION_RENAMED_NEW_NAME, EventKind::Rename),
        ];
        for (index, (action, _)) in expected.iter().enumerate() {
            let mut next = record(*action, &[b'a' as u16 + index as u16]);
            if index + 1 != expected.len() {
                // A 14-byte record deliberately leaves the following DWORDs unaligned.
                let length = next.len() as u32;
                next[..4].copy_from_slice(&length.to_le_bytes());
            }
            bytes.extend(next);
        }
        let decoded = events(directory, &bytes)?;
        assert_eq!(decoded.len(), expected.len(), "every native record is retained");
        for (index, (event, (_, kind))) in decoded.iter().zip(expected).enumerate() {
            assert_eq!(event.kind, kind, "both halves of a rename invalidate their paths");
            assert_eq!(
                event.path_kind,
                PathKind::Any,
                "the native record carries no reliable type"
            );
            assert_eq!(
                event.paths,
                [directory.join(char::from(b'a' + index as u8).to_string())],
                "each native name is relative to its watched directory"
            );
        }
        Ok(())
    }

    #[test]
    fn preserves_utf16_names_including_unpaired_surrogates() -> gix_error::TestResult {
        let directory = Path::new(r"C:\watched");
        let name = [b'd' as u16, b'\\' as u16, 0x00e4, 0xd83d, 0xde80, 0xd800];
        let decoded = events(directory, &record(FILE_ACTION_ADDED, &name))?;
        let expected: Vec<_> = directory
            .as_os_str()
            .encode_wide()
            .chain([b'\\' as u16])
            .chain(name)
            .collect();
        assert_eq!(
            decoded[0].paths[0].as_os_str().encode_wide().collect::<Vec<_>>(),
            expected,
            "native names must not pass through lossy Unicode conversion"
        );
        Ok(())
    }

    #[test]
    fn rejects_truncated_or_inconsistent_records() -> gix_error::TestResult {
        let valid = record(FILE_ACTION_ADDED, &[b'a' as u16]);
        for length in 0..valid.len() {
            assert!(
                events(Path::new(r"C:\watched"), &valid[..length]).is_err(),
                "a truncated header or name requires a rescan: {length} bytes"
            );
        }
        for (offset, value) in [
            (0, 1u32),
            (0, 12),
            (0, 14),
            (0, u32::MAX),
            (4, 99),
            (8, 0),
            (8, 1),
            (8, u32::MAX),
        ] {
            let mut malformed = valid.clone();
            malformed[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(
                events(Path::new(r"C:\watched"), &malformed).is_err(),
                "invalid offsets, action codes and name lengths require a rescan: {offset}={value}"
            );
        }
        Ok(())
    }

    #[test]
    fn rejects_paths_that_can_escape_the_watched_directory() -> gix_error::TestResult {
        for name in [
            r"..\outside",
            r"nested\..\outside",
            r"\absolute",
            r"C:\absolute",
            r"C:relative",
            ".",
            "nul\0name",
        ] {
            let wide: Vec<_> = name.encode_utf16().collect();
            assert!(
                events(Path::new(r"C:\watched"), &record(FILE_ACTION_ADDED, &wide)).is_err(),
                "a native record must provide a nonempty relative descendant: {name:?}"
            );
        }
        Ok(())
    }
}
