use std::{collections::BTreeSet, io};

use gix_error::{ResultExt, bail, message};
use gix_hash::ObjectId;
use gix_ref::bstr::ByteSlice;

use crate::{Repository, Result};

pub(super) fn revisions(repo: &Repository, explicit: Option<Vec<ObjectId>>) -> Result<Vec<ObjectId>> {
    let mut revisions = BTreeSet::new();
    if let Some(explicit) = explicit {
        for object_id in explicit {
            revisions.insert(repo.find_object(object_id)?.peel_to_commit()?.id);
        }
        return Ok(revisions.into_iter().collect());
    }
    let Some(paths) = repo
        .config
        .resolved
        .strings_filter("blame.ignoreRevsFile", repo.filter_config_section())
    else {
        return Ok(Vec::new());
    };
    let install_dir = crate::path::install_dir().ok();
    let home = crate::config::cache::access::home_dir(repo.open_options().permissions.env);
    for path in paths {
        if path.is_empty() {
            revisions.clear();
            continue;
        }
        let path = gix_config::Path::from(path);
        let optional = path.is_optional;
        if optional && path.is_empty() {
            continue;
        }
        let path = path.interpolate(crate::config::cache::interpolate_context(
            install_dir.as_deref(),
            home.as_deref(),
        ))?;
        let path = repo
            .current_dir()
            .join(repo.workdir().unwrap_or(repo.git_dir()))
            .join(path);
        let contents = match std::fs::read(&path) {
            Ok(contents) => contents,
            Err(err) if optional && err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(err).or_raise(|| message!("Could not read ignored revisions from {}", path.display()));
            }
        };
        for (line_number, line) in contents.lines().enumerate() {
            let line = line.split(|byte| *byte == b'#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let object_id = ObjectId::from_hex(line)
                .or_raise(|| message!("Invalid ignored revision at {}:{}", path.display(), line_number + 1))?;
            if object_id.kind() != repo.object_hash() {
                bail!(message!("Invalid ignored revision at {}:{}", path.display(), line_number + 1).validation());
            }
            // Like Git's file-based ignore lists, unavailable objects and non-commits are skipped.
            if let Ok(commit) = repo.find_object(object_id).and_then(crate::Object::peel_to_commit) {
                revisions.insert(commit.id);
            }
        }
    }
    Ok(revisions.into_iter().collect())
}
