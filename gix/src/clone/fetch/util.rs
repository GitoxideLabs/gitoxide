use std::io::Write;

use gix_error::{ResultExt, message};
use gix_ref::{
    Category, FullNameRef, PartialName,
    transaction::{LogChange, RefLog},
};

use crate::{
    Error, Repository, Result,
    bstr::{BStr, BString, ByteSlice},
};

enum WriteMode {
    Overwrite,
    Append,
}
pub fn append_remote_to_local_config_file(
    remote: &mut crate::Remote<'_>,
    remote_name: BString,
) -> Result<gix_config::File> {
    let mut config = gix_config::File::new(local_config_meta(remote.repo));
    remote
        .save_as_to(remote_name, &mut config)
        .or_raise(|| message("Failed to store configured remote in memory"))?;

    write_to_local_config(&config, WriteMode::Append)
        .or_raise(|| message("Failed to write repository configuration to disk"))?;
    Ok(config)
}

/// Reconfigure the freshly-initialized, still-empty repository `repo` to use `object_hash`
/// by rewriting the object-format related entries in its local configuration file on disk,
/// and reload the repository handle.
///
/// This relies on the initial reference database not having persisted any hash-format-dependent
/// state. That is true for the current file-based ref store, but a future reftable backend must
/// not be initialized with the wrong hash and then reused. If clone learns the remote hash only
/// after repository creation, initialize a non-reftable reference database first, then convert it
/// to reftable once the remote hash is known.
///
/// Existing local configuration, including the remote section written during clone setup,
/// is preserved. Only local sections are written back to `.git/config`,
///
/// The returned repository is reopened from disk so the object hash change affects all
/// hash-dependent state. Callers that need in-memory configuration from `repo` must
/// transfer it to the returned handle.
#[cfg(feature = "sha256")]
pub(super) fn reinitialize_with_object_hash(
    repo: &crate::Repository,
    object_hash: gix_hash::Kind,
) -> Result<crate::Repository> {
    let git_dir = repo.git_dir();
    let config_path = git_dir.join("config");

    let mut config = gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local)
        .or_raise(|| message("Failed to load repo-local git configuration before writing"))?;
    // Mirror what `crate::create` writes at init time: only SHA-256 repositories get
    // `repositoryformatversion = 1` along with the `objectformat` extension.
    let is_sha256 = object_hash == gix_hash::Kind::Sha256;
    config
        .section_mut("core", None)
        .expect("freshly initialized repository has a core section")
        .set("repositoryformatversion", if is_sha256 { "1" } else { "0" })?;
    if is_sha256 {
        config
            .section_mut_or_create_new("extensions", None)
            .expect("valid section name")
            .set("objectformat", object_hash.to_string())?;
    } else {
        // In a freshly initialized repository, this section exists solely to carry `objectformat`.
        config.remove_section("extensions", None);
    }
    let mut lock =
        gix_lock::File::acquire_to_update_resource(&config_path, gix_lock::acquire::Fail::Immediately, None, 0)
            .or_raise(|| message("Failed to acquire lock to write repository configuration to disk"))?;
    config
        .write_to_filter(&mut lock, |section| section.meta().source == gix_config::Source::Local)
        .or_raise(|| message("Failed to write repository configuration to disk"))?;
    lock.commit()
        .or_raise(|| message("Failed to commit lock after writing repository configuration to disk"))?;

    Ok(crate::ThreadSafeRepository::open_opts(git_dir, repo.options.clone())
        .or_raise(|| message("Failed to reopen the local repository after adopting the remote's object format"))?
        .to_thread_local())
}

fn local_config_meta(repo: &Repository) -> gix_config::file::Metadata {
    let meta = repo.config.resolved.meta().clone();
    assert_eq!(
        meta.source,
        gix_config::Source::Local,
        "local path is the default for new sections"
    );
    meta
}

fn write_to_local_config(config: &gix_config::File, mode: WriteMode) -> std::io::Result<()> {
    assert_eq!(
        config.meta().source,
        gix_config::Source::Local,
        "made for appending to local configuration file"
    );
    let mut local_config = std::fs::OpenOptions::new()
        .create(false)
        .write(matches!(mode, WriteMode::Overwrite))
        .append(matches!(mode, WriteMode::Append))
        .open(config.meta().path.as_deref().expect("local config with path set"))?;
    local_config.write_all(config.detect_newline_style())?;
    config.write_to_filter(&mut local_config, |s| s.meta().source == gix_config::Source::Local)
}

/// Append `config` to `repo`'s in-memory resolved configuration.
///
/// This is used after writing clone-specific local configuration to `.git/config`,
/// as the `repo` handle was opened before that write and won't observe it until
/// it is either updated in memory or reopened.
pub fn append_config_to_repo_config(repo: &mut Repository, config: gix_config::File) -> Result {
    let repo_config = gix_parallel::OwnShared::make_mut(&mut repo.config.resolved);
    repo_config
        .append(config)
        .or_raise(|| message("Failed to append repository configuration"))
        .map(|_| ())
}

/// Select the mapping for the remote's `HEAD` using Git clone's branch-selection order:
///
/// 1. If `HEAD` is symbolic, return the first mapping for its target, without falling back to other branches.
/// 2. If `HEAD` is direct, consider only mapped branches pointing to the same object ID. Prefer
///    `refs/heads/<init.defaultBranch>` (defaulting to `master`), then `refs/heads/master`.
/// 3. Otherwise, select the lexicographically first matching branch under `refs/heads/`.
///
/// Return `None` if `HEAD` is absent, unborn, or neither symbolic nor direct, or no eligible mapping exists.
/// If multiple mappings refer to the selected remote branch, return the first so its local peer is preserved.
pub(super) fn guess_remote_head<'a>(
    repo: &Repository,
    ref_map: &'a crate::remote::fetch::RefMap,
) -> Result<Option<&'a gix_protocol::fetch::refmap::Mapping>> {
    let Some(head) = ref_map
        .remote_refs
        .iter()
        .find(|reference| reference.unpack().0 == "HEAD")
    else {
        return Ok(None);
    };
    let (target, commit_id) = match head {
        gix_protocol::handshake::Ref::Symbolic { target, object, .. } => {
            (Some(target.as_bstr()), Some(object.as_ref()))
        }
        gix_protocol::handshake::Ref::Direct { object, .. } => (None, Some(object.as_ref())),
        _ => return Ok(None),
    };
    if let Some(target) = target {
        return Ok(ref_map
            .mappings
            .iter()
            .find(|mapping| mapping.remote.as_name() == Some(target)));
    }
    let branch_name = repo
        .config_snapshot()
        .string(crate::config::tree::Init::DEFAULT_BRANCH)
        .unwrap_or_else(|| "master".into());
    for name in [format!("refs/heads/{branch_name}"), "refs/heads/master".into()] {
        if let Some(mapping) = ref_map.mappings.iter().find(|mapping| {
            mapping.remote.as_name() == Some(name.as_bytes().as_bstr()) && mapping.remote.as_id() == commit_id
        }) {
            return Ok(Some(mapping));
        }
    }
    Ok(ref_map
        .mappings
        .iter()
        .filter(|mapping| {
            mapping
                .remote
                .as_name()
                .is_some_and(|name| name.starts_with(b"refs/heads/"))
                && mapping.remote.as_id() == commit_id
        })
        .min_by_key(|mapping| mapping.remote.as_name()))
}

/// Record the selected branch's mapped peer before updating local HEAD, as Git clone does.
pub(super) fn update_remote_head(
    repo: &Repository,
    ref_map: &crate::remote::fetch::RefMap,
    reflog_message: &BStr,
    remote_name: &BStr,
) -> Result<()> {
    if repo.is_bare() {
        return Ok(());
    }
    let Some(mapping) = guess_remote_head(repo, ref_map)? else {
        return Ok(());
    };
    let Some(tracking_branch) = mapping.local.as_ref() else {
        return Ok(());
    };
    let head_name =
        gix_ref::FullName::try_from(BString::from(format!("refs/remotes/{remote_name}/HEAD"))).or_error()?;
    repo.edit_reference(gix_ref::transaction::RefEdit::update_with_log(
        head_name,
        gix_ref::FullName::try_from(tracking_branch.clone()).or_error()?,
        gix_ref::transaction::PreviousValue::Any,
        LogChange {
            mode: RefLog::AndReference,
            force_create_reflog: false,
            message: reflog_message.to_owned(),
        },
    ))
    .or_raise(|| message("Failed to record the remote default branch"))?;
    Ok(())
}

/// HEAD cannot be written by means of refspec by design, so we have to do it manually here. Also create the pointed-to ref
/// if we have to, as it might not have been naturally included in the ref-specs.
/// Lastly, use `ref_name` if it was provided instead, and let `HEAD` point to it.
pub fn update_head(
    repo: &mut Repository,
    ref_map: &crate::remote::fetch::RefMap,
    reflog_message: &BStr,
    remote_name: &BStr,
    ref_name: Option<&PartialName>,
    revision: Option<&gix_refspec::RefSpec>,
) -> Result<()> {
    use gix_ref::transaction::{PreviousValue, RefEdit};
    let revision_head_id = revision
        .map(|revision| -> Result<gix_hash::ObjectId> {
            let mapping = find_revision(ref_map, revision)?;
            let id = mapping.remote.peeled_id().ok_or_else(|| revision_missing(revision))?;
            Ok(repo
                .find_object(id)
                .or_raise(|| message("The requested revision could not be read"))?
                .peel_to_commit()
                .or_raise(|| message("The requested revision did not peel to a commit"))?
                .id)
        })
        .transpose()?;
    let detached_tag_id = ref_name
        .map(|name| -> Result<Option<gix_hash::ObjectId>> {
            let (target, full_name) = find_custom_refname(ref_map, name)?;
            if full_name.starts_with(b"refs/heads/") {
                return Ok(None);
            }
            Ok(Some(repo.find_object(target)?.peel_to_commit()?.id))
        })
        .transpose()?
        .flatten();
    let head_info = match revision_head_id.as_ref().or(detached_tag_id.as_ref()) {
        Some(id) => Some((Some(id.as_ref()), None)),
        None => match ref_name {
            Some(ref_name) => {
                let (target, full_ref_name) = find_custom_refname(ref_map, ref_name)?;
                Some((
                    Some(target),
                    full_ref_name.starts_with(b"refs/heads/").then_some(full_ref_name),
                ))
            }
            None => {
                if let Some(mapping) = guess_remote_head(repo, ref_map)? {
                    Some((mapping.remote.as_id(), mapping.remote.as_name()))
                } else {
                    ref_map.remote_refs.iter().find_map(|r| {
                        Some(match r {
                            gix_protocol::handshake::Ref::Symbolic {
                                full_ref_name,
                                target: _,
                                tag: _,
                                object,
                            } if full_ref_name == "HEAD" => (Some(object.as_ref()), None),
                            gix_protocol::handshake::Ref::Direct { full_ref_name, object }
                                if full_ref_name == "HEAD" =>
                            {
                                (Some(object.as_ref()), None)
                            }
                            gix_protocol::handshake::Ref::Unborn { full_ref_name, target }
                                if full_ref_name == "HEAD" =>
                            {
                                (None, Some(target.as_bstr()))
                            }
                            _ => return None,
                        })
                    })
                }
            }
        },
    };
    let Some((head_peeled_id, head_ref)) = head_info else {
        return Ok(());
    };

    let head: gix_ref::FullName = "HEAD".try_into().expect("valid");
    let reflog_message = || LogChange {
        mode: RefLog::AndReference,
        force_create_reflog: false,
        message: reflog_message.to_owned(),
    };
    match head_ref {
        Some(referent) => {
            let referent: gix_ref::FullName = gix_ref::FullName::try_from(referent).or_raise(|| {
                message!("The remote HEAD points to a reference named {referent:?} which is invalid.").validation()
            })?;
            repo.edit_reference(RefEdit::update_with_log(
                head.clone(),
                referent.clone(),
                PreviousValue::Any,
                reflog_message(),
            ))
            .or_raise(|| message("Failed to update HEAD with values from remote"))?;
            if !repo.is_bare() {
                if let Some(head_peeled_id) = head_peeled_id {
                    let mut edit = RefEdit::update_with_log(
                        head.clone(),
                        head_peeled_id.to_owned(),
                        PreviousValue::Any,
                        reflog_message(),
                    );
                    edit.deref = true;
                    repo.edit_reference(edit)
                        .or_raise(|| message("Failed to update HEAD with values from remote"))?;
                }
                setup_branch_config(repo, referent.as_ref(), head_peeled_id, remote_name)?;
            }
        }
        None => {
            repo.edit_reference(RefEdit::update_with_log(
                head,
                head_peeled_id
                    .expect("detached heads always point to something")
                    .to_owned(),
                PreviousValue::Any,
                reflog_message(),
            ))
            .or_raise(|| message("Failed to update HEAD with values from remote"))?;
        }
    }
    Ok(())
}

/// Find the mapping produced by the exact refspec used to request `revision`.
///
/// Returns an error if the remote did not map that refspec.
pub(super) fn find_revision<'a>(
    ref_map: &'a crate::remote::fetch::RefMap,
    revision: &gix_refspec::RefSpec,
) -> Result<&'a gix_protocol::fetch::refmap::Mapping> {
    ref_map
        .mappings
        .iter()
        .find(|mapping| {
            mapping
                .spec_index
                .get(&ref_map.refspecs, &ref_map.extra_refspecs)
                .is_some_and(|spec| spec == revision)
        })
        .ok_or_else(|| revision_missing(revision))
}

fn revision_missing(revision: &gix_refspec::RefSpec) -> Error {
    message!(
        "The remote didn't have the requested revision {:?}",
        revision.to_ref().source().expect("validated revision")
    )
    .not_found_error()
}

/// Resolve `ref_name` to its object ID and full name among the mapped remote references.
///
/// Full names match directly. Partial names prefer branches over tags, then use normal refspec matching.
/// Returns an error when there is no unique match.
/// Ambiguous matches include the requested reference name bytes as `input` [metadata](gix_error::Error::metadata()).
pub(super) fn find_custom_refname<'a>(
    ref_map: &'a crate::remote::fetch::RefMap,
    ref_name: &PartialName,
) -> Result<(&'a gix_hash::oid, &'a BStr)> {
    let group = gix_refspec::MatchGroup::from_fetch_specs(Some(
        gix_refspec::parse(ref_name.as_ref().as_bstr(), gix_refspec::parse::Operation::Fetch)
            .expect("partial names are valid refs"),
    ));
    let filtered_items: Vec<_> = ref_map
        .mappings
        .iter()
        .filter_map(|m| m.remote.as_name().zip(m.remote.as_id()))
        .map(|(full_ref_name, target)| gix_refspec::match_group::Item {
            full_ref_name,
            target,
            object: None,
        })
        .collect();

    let requested_name = ref_name.as_ref().as_bstr();
    let find_item = |name: &BStr| filtered_items.iter().find(|item| item.full_ref_name == name).copied();
    // Preserve gix's documented full-ref support, then match git clone --branch by trying heads before tags.
    if let Some(item) = find_item(requested_name) {
        return Ok((item.target, item.full_ref_name));
    }
    if !requested_name.starts_with(b"refs/") {
        let branch_name = Category::LocalBranch.to_full_name(requested_name).or_error()?;
        if let Some(item) = find_item(branch_name.as_bstr()) {
            return Ok((item.target, item.full_ref_name));
        }

        let tag_name = Category::Tag.to_full_name(requested_name).or_error()?;
        if let Some(item) = find_item(tag_name.as_bstr()) {
            return Ok((item.target, item.full_ref_name));
        }
    }

    let res = group.match_lhs(filtered_items.iter().copied());
    match res.mappings.len() {
        0 => Err(message!("The remote didn't have any ref that matched '{requested_name}'").not_found_error()),
        1 => {
            let item = filtered_items[res.mappings[0]
                .item_index
                .expect("we map by name only and have no object-id in refspec")];
            Ok((item.target, item.full_ref_name))
        }
        _ => {
            let candidates = res
                .mappings
                .into_iter()
                .filter_map(|m| match m.lhs {
                    gix_refspec::match_group::SourceRef::FullName(name) => Some(name.into_owned()),
                    gix_refspec::match_group::SourceRef::ObjectId(_) => None,
                })
                .collect::<Vec<_>>();
            Err(message!(
                "The remote has {} refs for '{requested_name}', try to use a specific name: {}",
                candidates.len(),
                candidates
                    .iter()
                    .filter_map(|name| name.to_str().ok())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .with_input(ref_name.as_ref().as_bstr().to_owned())
            .validation_error())
        }
    }
}

/// Configure `branch` in `repo` to track the same-named branch on `remote_name`, looking up that remote
/// in `repo` and matching its saved fetch refspecs against `branch` and its target `branch_id`.
/// If `branch_id` is `None`, the branch is unborn and upstream configuration is written even without a match.
/// Otherwise, write it only if a refspec matches. Persist the configuration to disk and update `repo` in memory.
/// Ignore branches outside `refs/heads/` or whose short names are not valid UTF-8.
fn setup_branch_config(
    repo: &mut Repository,
    branch: &FullNameRef,
    branch_id: Option<&gix_hash::oid>,
    remote_name: &BStr,
) -> Result<()> {
    let short_name = match branch.category_and_short_name() {
        Some((gix_ref::Category::LocalBranch, shortened)) => match shortened.to_str() {
            Ok(s) => s,
            Err(_) => return Ok(()),
        },
        _ => return Ok(()),
    };
    let remote = repo
        .find_remote(remote_name)
        .expect("remote was just created and must be visible in config");
    let group = gix_refspec::MatchGroup::from_fetch_specs(remote.fetch_specs.iter().map(gix_refspec::RefSpec::to_ref));
    let null = gix_hash::ObjectId::null(repo.object_hash());
    let res = group.match_lhs(
        Some(gix_refspec::match_group::Item {
            full_ref_name: branch.as_bstr(),
            target: branch_id.unwrap_or(&null),
            object: None,
        })
        .into_iter(),
    );
    if branch_id.is_none() || !res.mappings.is_empty() {
        let mut config = repo.config_snapshot_mut();
        let mut section = config
            .new_section("branch", short_name)
            .expect("section header name is always valid per naming rules, our input branch name is valid");
        section.push("remote", remote_name)?;
        section.push("merge", branch.as_bstr())?;
        write_to_local_config(&config, WriteMode::Overwrite).or_error()?;
        config.commit().expect("configuration we set is valid");
    }
    Ok(())
}
