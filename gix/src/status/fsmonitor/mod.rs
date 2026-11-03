use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    Repository, Result,
    bstr::{BStr, BString, ByteSlice},
    config::tree::Core,
};
use gix_index::{entry::Flags, extension::fs_monitor::Token};

mod hook;

pub(super) struct Refresh {
    token: Option<Token>,
    pub(super) valid: Vec<AtomicBool>,
}

impl Refresh {
    pub(super) fn has_changes(&self, index: &gix_index::State) -> bool {
        if self.token.as_ref() != index.fs_monitor().map(gix_index::extension::FsMonitor::token) {
            return true;
        }
        self.token.is_some()
            && index
                .fs_monitor()
                .and_then(|monitor| monitor.valid_entries(self.valid.len()))
                .is_none_or(|previous| {
                    previous
                        .into_iter()
                        .zip(&self.valid)
                        .any(|(previous, valid)| previous != valid.load(Ordering::Relaxed))
                })
    }

    pub(super) fn apply(self, index: &mut gix_index::State) {
        for (entry, valid) in index.entries_mut().iter_mut().zip(self.valid) {
            entry.flags.set(Flags::FSMONITOR_VALID, valid.into_inner());
        }
        index.set_fs_monitor_token(self.token);
    }
}

pub(super) fn refresh<'a>(
    repo: &Repository,
    index: &'a gix_index::State,
    lookup: Option<&gix_index::AccelerateLookup<'a>>,
) -> Result<Option<Refresh>> {
    let configured = repo
        .config
        .resolved
        .string_implicit_filter(Core::FS_MONITOR, repo.filter_config_section());
    let enabled = match configured.as_ref() {
        None => false,
        Some(None) => true,
        Some(Some(value)) => gix_config::Boolean::try_from(value.as_bstr()).map_or(true, |value| value.0),
    };
    if !enabled {
        return Ok(index.fs_monitor().map(|_| Refresh {
            token: None,
            valid: (0..index.entries().len()).map(|_| AtomicBool::new(false)).collect(),
        }));
    }

    let previous = index.fs_monitor().map(gix_index::extension::FsMonitor::token);
    let command = configured.flatten();
    let builtin = command
        .as_ref()
        .is_none_or(|value| gix_config::Boolean::try_from(value.as_bstr()).is_ok_and(|value| value.0));
    let response = if builtin {
        // Native daemon support is added separately; a full scan is always safe.
        None
    } else {
        let version = repo
            .config
            .resolved
            .integer_filter(Core::FS_MONITOR_HOOK_VERSION, repo.filter_config_section());
        let version = match version? {
            None => None,
            Some(version @ (1 | 2)) => Some(version),
            Some(_) => return Err(crate::error::message("core.fsmonitorHookVersion must be 1 or 2").into()),
        };
        hook::query(
            repo,
            command.as_ref().expect("a hook has a command").as_bstr(),
            previous,
            version,
        )
        .ok()
        .flatten()
    };
    let mut valid = index
        .fs_monitor()
        .and_then(|monitor| monitor.valid_entries(index.entries().len()))
        .unwrap_or_else(|| vec![false; index.entries().len()]);
    let token = if let Some((token, paths)) = response {
        if !invalidate(index, &mut valid, &paths, lookup) {
            valid.fill(false);
        }
        Some(token)
    } else {
        valid.fill(false);
        None
    };
    for (entry, valid) in index.entries().iter().zip(&mut valid) {
        if entry.mode.is_submodule() || entry.stage_raw() != 0 || entry.flags.contains(Flags::INTENT_TO_ADD) {
            *valid = false;
        }
    }
    Ok(Some(Refresh {
        token,
        valid: valid.into_iter().map(AtomicBool::new).collect(),
    }))
}

fn invalidate<'a>(
    index: &'a gix_index::State,
    valid: &mut [bool],
    data: &[u8],
    lookup: Option<&gix_index::AccelerateLookup<'a>>,
) -> bool {
    if data.is_empty() {
        return true;
    }
    for path in data.strip_suffix(b"\0").unwrap_or(data).split(|byte| *byte == 0) {
        let path = path.strip_suffix(b"/").unwrap_or(path);
        if path.is_empty()
            || path.starts_with(b"/")
            || path
                .split(|byte| *byte == b'/')
                .any(|part| part == b".." || part == b".")
        {
            return false;
        }
        let path = path.as_bstr();
        if !invalidate_path(index, valid, path)
            && let Some(lookup) = lookup
        {
            if let Some(entry) = index.entry_by_path_icase(path, true, lookup) {
                invalidate_path(index, valid, entry.path(index));
            } else if let Some(entry) = index.entry_closest_to_directory_or_directory_icase(path, true, lookup) {
                invalidate_path(index, valid, entry.path(index)[..path.len()].as_bstr());
            }
        }
    }
    true
}

fn invalidate_path(index: &gix_index::State, valid: &mut [bool], path: &BStr) -> bool {
    let entry = index.entry_index_by_path(path).ok();
    if let Some(entry) = entry {
        valid[entry] = false;
    }
    let mut directory = path.to_owned();
    directory.push(b'/');
    let range = index.prefixed_entries_range(directory.as_bstr());
    let found = entry.is_some() || range.is_some();
    if let Some(range) = range {
        valid[range].fill(false);
    }
    found
}

fn token_bytes(token: Option<&Token>) -> BString {
    match token {
        Some(Token::V1 { nanos_since_1970 }) => nanos_since_1970.to_string().into(),
        Some(Token::V2 { token }) => token.clone(),
        None => "0".into(),
    }
}

fn parse_v2(data: Vec<u8>) -> Option<(Token, Vec<u8>)> {
    let end = data.find_byte(0)?;
    (end != 0).then(|| {
        (
            Token::V2 {
                token: data[..end].into(),
            },
            data[end + 1..].to_vec(),
        )
    })
}
