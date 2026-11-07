use std::time::SystemTime;

use crate::{
    Repository, Result,
    bstr::{BStr, ByteSlice},
    error::{ResultExt, message},
};
use gix_fsmonitor::client::hook::Version;
use gix_index::extension::fs_monitor::Token;

pub(super) fn query(
    repo: &Repository,
    command: &BStr,
    previous: Option<&Token>,
    version: Option<i64>,
) -> Result<Option<(Token, Vec<u8>)>> {
    let nanos_since_1970 = u64::try_from(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .or_raise(|| message("Could not determine the filesystem monitor timestamp"))?
            .as_nanos(),
    )
    .or_raise(|| message("Filesystem monitor timestamp exceeds u64::MAX"))?;
    let previous = super::token_bytes(previous);
    let version = version.map(|version| if version == 1 { Version::One } else { Version::Two });
    let response = gix_fsmonitor::client::hook::query(command, previous.as_bstr(), version, repo.command_context()?)?;
    Ok(response.and_then(|(version, data)| match version {
        Version::One => Some((Token::V1 { nanos_since_1970 }, data)),
        Version::Two => super::parse_v2(data),
    }))
}
