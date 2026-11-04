use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    Repository, Result,
    bstr::{BStr, BString, ByteSlice},
    config::tree::Core,
};
use gix_index::{entry::Flags, extension::fs_monitor::Token};

mod daemon;
mod hook;
mod untracked;

pub(super) struct Refresh {
    token: Option<Token>,
    pub(super) valid: Vec<AtomicBool>,
    untracked: Option<gix_index::extension::UntrackedCache>,
    use_untracked: bool,
}

impl Refresh {
    pub(super) fn has_changes(&self, index: &gix_index::State) -> bool {
        if self
            .untracked
            .as_ref()
            .map(|cache| cache.directories().iter().filter(|dir| dir.stat().is_some()).count())
            != index
                .untracked()
                .map(|cache| cache.directories().iter().filter(|dir| dir.stat().is_some()).count())
        {
            return true;
        }
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
        index.set_untracked(self.untracked);
    }
}

pub(super) fn refresh<'a>(
    repo: &Repository,
    index: &'a gix_index::State,
    lookup: Option<&gix_index::AccelerateLookup<'a>>,
) -> Result<Option<Refresh>> {
    let _span = gix_trace::coarse!("gix::status::fsmonitor::refresh", index_entries = index.entries().len());
    let configured = repo
        .config
        .resolved
        .string_implicit_filter(Core::FS_MONITOR, repo.filter_config_section());
    let enabled = match configured.as_ref() {
        None => false,
        Some(None) => true,
        Some(Some(value)) => gix_config::Boolean::try_from(value.as_bstr()).map_or(true, |value| value.0),
    };
    let discard_untracked = repo
        .config
        .resolved
        .boolean_filter(Core::UNTRACKED_CACHE, repo.filter_config_section())
        .is_ok_and(|value| value == Some(false));
    if !enabled && index.fs_monitor().is_none() && !(discard_untracked && index.untracked().is_some()) {
        return Ok(None);
    }
    let mut untracked = if discard_untracked {
        None
    } else {
        index.untracked().cloned()
    };
    if !enabled {
        if let Some(cache) = &mut untracked {
            cache.invalidate_all();
        }
        gix_trace::debug!("filesystem monitor disabled");
        return Ok(Some(Refresh {
            token: None,
            valid: (0..index.entries().len()).map(|_| AtomicBool::new(false)).collect(),
            untracked,
            use_untracked: false,
        }));
    }

    let previous = index.fs_monitor().map(gix_index::extension::FsMonitor::token);
    let command = configured.flatten();
    let builtin = command
        .as_ref()
        .is_none_or(|value| gix_config::Boolean::try_from(value.as_bstr()).is_ok_and(|value| value.0));
    gix_trace::debug!(
        kind = if builtin { "daemon" } else { "hook" },
        has_previous_token = previous.is_some(),
        "querying filesystem monitor"
    );
    let response = if builtin {
        let token = previous.map_or_else(|| "builtin:fake".into(), |token| token_bytes(Some(token)));
        daemon::query(repo, token.as_bstr()).ok().and_then(parse_v2)
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
    let previous_valid = index
        .fs_monitor()
        .and_then(|monitor| monitor.valid_entries(index.entries().len()));
    let mut use_untracked = previous_valid.is_some();
    let mut valid = previous_valid.unwrap_or_else(|| {
        gix_trace::debug!("no usable filesystem monitor bitmap; scanning all entries");
        vec![false; index.entries().len()]
    });
    gix_trace::debug!(
        valid_entries = valid.iter().filter(|valid| **valid).count(),
        "restored filesystem monitor bitmap"
    );
    let token = if let Some((token, paths)) = response {
        gix_trace::debug!(
            reported_paths = if paths.is_empty() {
                0
            } else {
                paths
                    .strip_suffix(b"\0")
                    .unwrap_or(&paths)
                    .split(|byte| *byte == 0)
                    .count()
            },
            "received filesystem monitor changes"
        );
        if !invalidate(index, &mut valid, &paths, lookup, untracked.as_mut()) {
            valid.fill(false);
            use_untracked = false;
        }
        Some(token)
    } else {
        gix_trace::debug!("filesystem monitor query failed or returned an invalid response; scanning all entries");
        valid.fill(false);
        use_untracked = false;
        builtin.then(|| Token::V2 {
            token: "builtin:fake".into(),
        })
    };
    // shortcut: UNTR does not record filename composition; reuse non-ASCII names once that policy can be validated.
    if use_untracked
        && untracked.as_ref().is_some_and(|cache| {
            index.entries().iter().any(|entry| !entry.path(index).is_ascii())
                || cache
                    .directories()
                    .iter()
                    .any(|dir| !dir.name().is_ascii() || dir.untracked_entries().iter().any(|name| !name.is_ascii()))
        })
    {
        gix_trace::debug!("untracked cache filename composition is uncertain; scanning directories");
        use_untracked = false;
    }
    if !use_untracked && let Some(cache) = &mut untracked {
        cache.invalidate_all();
    }
    for (entry, valid) in index.entries().iter().zip(&mut valid) {
        if entry.mode.is_submodule() || entry.stage_raw() != 0 || entry.flags.contains(Flags::INTENT_TO_ADD) {
            *valid = false;
        }
    }
    gix_trace::debug!(
        valid_entries = valid.iter().filter(|valid| **valid).count(),
        "refreshed filesystem monitor bitmap"
    );
    Ok(Some(Refresh {
        token,
        valid: valid.into_iter().map(AtomicBool::new).collect(),
        untracked,
        use_untracked,
    }))
}

fn invalidate<'a>(
    index: &'a gix_index::State,
    valid: &mut [bool],
    data: &[u8],
    lookup: Option<&gix_index::AccelerateLookup<'a>>,
    mut untracked: Option<&mut gix_index::extension::UntrackedCache>,
) -> bool {
    if data.is_empty() {
        return true;
    }
    for reported_path in data.strip_suffix(b"\0").unwrap_or(data).split(|byte| *byte == 0) {
        if reported_path == b"/" {
            gix_trace::debug!("filesystem monitor requested a full scan");
            return false;
        }
        let path = reported_path.strip_suffix(b"/").unwrap_or(reported_path);
        if path.is_empty()
            || path.starts_with(b"/")
            || path
                .split(|byte| *byte == b'/')
                .any(|part| part == b".." || part == b".")
        {
            gix_trace::debug!("filesystem monitor returned an invalid path; scanning all entries");
            return false;
        }
        let path = path.as_bstr();
        if let Some(cache) = &mut untracked {
            if lookup.is_some() || reported_path.rsplit(|byte| *byte == b'/').next() == Some(b".gitignore".as_slice()) {
                cache.invalidate_all();
            } else {
                cache.invalidate_path(reported_path.as_bstr());
            }
        }
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
    let Some(end) = data.find_byte(0).filter(|end| *end != 0) else {
        gix_trace::debug!("filesystem monitor response has a missing or empty v2 token");
        return None;
    };
    Some((
        Token::V2 {
            token: data[..end].into(),
        },
        data[end + 1..].to_vec(),
    ))
}
