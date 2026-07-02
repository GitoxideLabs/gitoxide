use std::{borrow::Cow, io::Write};

use gix_ref::{
    Category, FullNameRef, PartialName,
    transaction::{LogChange, RefLog},
};

use super::Error;
use crate::{
    Repository,
    bstr::{BStr, BString, ByteSlice},
};

enum WriteMode {
    Overwrite,
    Append,
}

/// Persist the provided remote into the repository's local config, optionally adding partial clone settings.
#[allow(clippy::result_large_err)]
pub fn append_remote_to_local_config_file(
    remote: &mut crate::Remote<'_>,
    remote_name: BString,
    filter: Option<&str>,
) -> Result<gix_config::File<'static>, Error> {
    use gix_config::parse::section::ValueName;

    let mut config = gix_config::File::new(local_config_meta(remote.repo));
    remote.save_as_to(remote_name.clone(), &mut config)?;

    if let Some(filter_spec) = filter {
        let subsection = remote_name.to_str().map_err(|err| Error::RemoteNameNotUtf8 {
            remote_name: remote_name.clone(),
            source: err,
        })?;
        let mut remote_section = config.section_mut_or_create_new("remote", Some(subsection.into()))?;

        while remote_section.remove("partialclonefilter").is_some() {}
        while remote_section.remove("promisor").is_some() {}

        let partial_clone_filter = ValueName::try_from("partialclonefilter").expect("known to be valid");
        remote_section.push(partial_clone_filter, Some(filter_spec.into()));

        let promisor = ValueName::try_from("promisor").expect("known to be valid");
        remote_section.push(promisor, Some("true".into()));

        let mut extensions_section = config.section_mut_or_create_new("extensions", None)?;
        while extensions_section.remove("partialClone").is_some() {}

        let partial_clone = ValueName::try_from("partialClone").expect("known to be valid");
        extensions_section.push(partial_clone, Some(remote_name.as_ref()));
    }

    write_to_local_config(&config, WriteMode::Append)?;
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
) -> Result<crate::Repository, Error> {
    use gix_config::parse::section::ValueName;

    let git_dir = repo.git_dir();
    let config_path = git_dir.join("config");

    let mut config = gix_config::File::from_path_no_includes(config_path.clone(), gix_config::Source::Local)?;
    // Mirror what `crate::create` writes at init time: only SHA-256 repositories get
    // `repositoryformatversion = 1` along with the `objectformat` extension.
    let is_sha256 = object_hash == gix_hash::Kind::Sha256;
    config
        .section_mut("core", None)
        .expect("freshly initialized repository has a core section")
        .set(
            ValueName::try_from("repositoryformatversion").expect("valid"),
            if is_sha256 { "1" } else { "0" }.into(),
        );
    if is_sha256 {
        config
            .section_mut_or_create_new("extensions", None)
            .expect("valid section name")
            .set(
                ValueName::try_from("objectformat").expect("valid"),
                object_hash.to_string().as_str().into(),
            );
    } else {
        // In a freshly initialized repository, this section exists solely to carry `objectformat`.
        config.remove_section("extensions", None);
    }
    let mut lock =
        gix_lock::File::acquire_to_update_resource(&config_path, gix_lock::acquire::Fail::Immediately, None)?;
    config.write_to_filter(&mut lock, |section| section.meta().source == gix_config::Source::Local)?;
    lock.commit()?;

    Ok(crate::ThreadSafeRepository::open_opts(git_dir, repo.options.clone())?.to_thread_local())
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

fn write_to_local_config(config: &gix_config::File<'static>, mode: WriteMode) -> std::io::Result<()> {
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
pub fn append_config_to_repo_config(repo: &mut Repository, config: gix_config::File<'static>) {
    let repo_config = gix_features::threading::OwnShared::make_mut(&mut repo.config.resolved);
    repo_config.append(config);
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
) -> Result<(), Error> {
    use gix_ref::{
        Target,
        transaction::{PreviousValue, RefEdit},
    };
    let head_info = match ref_name {
        Some(ref_name) => {
            let (target, full_ref_name) = find_custom_refname(ref_map, ref_name)?;
            Some((Some(target), Some(full_ref_name)))
        }
        None => ref_map.remote_refs.iter().find_map(|r| {
            Some(match r {
                gix_protocol::handshake::Ref::Symbolic {
                    full_ref_name,
                    target,
                    tag: _,
                    object,
                } if full_ref_name == "HEAD" => (Some(object.as_ref()), Some(target.as_bstr())),
                gix_protocol::handshake::Ref::Direct { full_ref_name, object } if full_ref_name == "HEAD" => {
                    (Some(object.as_ref()), None)
                }
                gix_protocol::handshake::Ref::Unborn { full_ref_name, target } if full_ref_name == "HEAD" => {
                    (None, Some(target.as_bstr()))
                }
                _ => return None,
            })
        }),
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
            let referent: gix_ref::FullName = referent.try_into().map_err(|err| Error::InvalidHeadRef {
                head_ref_name: referent.to_owned(),
                source: err,
            })?;
            repo.refs
                .transaction()
                .packed_refs(gix_ref::file::transaction::PackedRefs::DeletionsAndNonSymbolicUpdates(
                    Box::new(&repo.objects),
                ))
                .prepare(
                    {
                        let mut edits = vec![RefEdit {
                            change: gix_ref::transaction::Change::Update {
                                log: reflog_message(),
                                expected: PreviousValue::Any,
                                new: Target::Symbolic(referent.clone()),
                            },
                            name: head.clone(),
                            deref: false,
                        }];
                        if let Some(head_peeled_id) = head_peeled_id {
                            edits.push(RefEdit {
                                change: gix_ref::transaction::Change::Update {
                                    log: reflog_message(),
                                    expected: PreviousValue::Any,
                                    new: Target::Object(head_peeled_id.to_owned()),
                                },
                                name: referent.clone(),
                                deref: false,
                            });
                        }
                        edits
                    },
                    gix_lock::acquire::Fail::Immediately,
                    gix_lock::acquire::Fail::Immediately,
                )
                .map_err(crate::reference::edit::Error::from)?
                .commit(
                    repo.committer()
                        .transpose()
                        .map_err(|err| Error::HeadUpdate(crate::reference::edit::Error::ParseCommitterTime(err)))?,
                )
                .map_err(crate::reference::edit::Error::from)?;

            if let Some(head_peeled_id) = head_peeled_id {
                let mut log = reflog_message();
                log.mode = RefLog::Only;
                repo.edit_reference(RefEdit {
                    change: gix_ref::transaction::Change::Update {
                        log,
                        expected: PreviousValue::Any,
                        new: Target::Object(head_peeled_id.to_owned()),
                    },
                    name: head,
                    deref: false,
                })?;
            }

            setup_branch_config(repo, referent.as_ref(), head_peeled_id, remote_name)?;
        }
        None => {
            repo.edit_reference(RefEdit {
                change: gix_ref::transaction::Change::Update {
                    log: reflog_message(),
                    expected: PreviousValue::Any,
                    new: Target::Object(
                        head_peeled_id
                            .expect("detached heads always point to something")
                            .to_owned(),
                    ),
                },
                name: head,
                deref: false,
            })?;
        }
    }
    Ok(())
}

pub(super) fn find_custom_refname<'a>(
    ref_map: &'a crate::remote::fetch::RefMap,
    ref_name: &PartialName,
) -> Result<(&'a gix_hash::oid, &'a BStr), Error> {
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
        let branch_name = Category::LocalBranch.to_full_name(requested_name)?;
        if let Some(item) = find_item(branch_name.as_bstr()) {
            return Ok((item.target, item.full_ref_name));
        }

        let tag_name = Category::Tag.to_full_name(requested_name)?;
        if let Some(item) = find_item(tag_name.as_bstr()) {
            return Ok((item.target, item.full_ref_name));
        }
    }

    let res = group.match_lhs(filtered_items.iter().copied());
    match res.mappings.len() {
        0 => Err(Error::RefNameMissing {
            wanted: ref_name.clone(),
        }),
        1 => {
            let item = filtered_items[res.mappings[0]
                .item_index
                .expect("we map by name only and have no object-id in refspec")];
            Ok((item.target, item.full_ref_name))
        }
        _ => Err(Error::RefNameAmbiguous {
            wanted: ref_name.clone(),
            candidates: res
                .mappings
                .into_iter()
                .filter_map(|m| match m.lhs {
                    gix_refspec::match_group::SourceRef::FullName(name) => Some(name.into_owned()),
                    gix_refspec::match_group::SourceRef::ObjectId(_) => None,
                })
                .collect(),
        }),
    }
}

/// Set up the remote configuration for `branch` so that it points to itself, but on the remote, if and only if currently
/// saved refspecs are able to match it.
/// For that we reload the remote of `remote_name` and use its `ref_specs` for match.
fn setup_branch_config(
    repo: &mut Repository,
    branch: &FullNameRef,
    branch_id: Option<&gix_hash::oid>,
    remote_name: &BStr,
) -> Result<(), Error> {
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
    if !res.mappings.is_empty() {
        let mut config = repo.config_snapshot_mut();
        let mut section = config
            .new_section("branch", Some(Cow::Owned(short_name.into())))
            .expect("section header name is always valid per naming rules, our input branch name is valid");
        section.push("remote".try_into().expect("valid at compile time"), Some(remote_name));
        section.push(
            "merge".try_into().expect("valid at compile time"),
            Some(branch.as_bstr()),
        );
        write_to_local_config(&config, WriteMode::Overwrite)?;
        config.commit().expect("configuration we set is valid");
    }
    Ok(())
}
