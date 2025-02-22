use std::{borrow::Cow, io::Write};

use gix_ref::{
    transaction::{LogChange, RefLog},
    FullNameRef, PartialName,
};

use super::Error;
use crate::{
    bstr::{BStr, BString, ByteSlice},
    Repository,
};

enum WriteMode {
    Overwrite,
    Append,
}

#[allow(clippy::result_large_err)]
pub fn write_remote_to_local_config_file(
    remote: &mut crate::Remote<'_>,
    remote_name: BString,
) -> Result<gix_config::File<'static>, Error> {
    let mut config = gix_config::File::new(local_config_meta(remote.repo));
    remote.save_as_to(remote_name, &mut config)?;

    write_to_local_config(&config, WriteMode::Append)?;
    Ok(config)
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
        transaction::{PreviousValue, RefEdit},
        Target,
    };
    let head_info = match ref_name {
        Some(ref_name) => Some(find_custom_refname(ref_map, ref_name)?),
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
) -> Result<(Option<&'a gix_hash::oid>, Option<&'a BStr>), Error> {
    let group = gix_refspec::MatchGroup::from_fetch_specs(Some(
        gix_refspec::parse(ref_name.as_ref().as_bstr(), gix_refspec::parse::Operation::Fetch)
            .expect("partial names are valid refs"),
    ));
    // TODO: to fix ambiguity, implement priority system
    let filtered_items: Vec<_> = ref_map
        .mappings
        .iter()
        .filter_map(|m| {
            m.remote
                .as_name()
                .and_then(|name| m.remote.as_id().map(|id| (name, id)))
        })
        .map(|(full_ref_name, target)| gix_refspec::match_group::Item {
            full_ref_name,
            target,
            object: None,
        })
        .collect();
    let res = group.match_lhs(filtered_items.iter().copied());
    match res.mappings.len() {
        0 => Err(Error::RefNameMissing {
            wanted: ref_name.clone(),
        }),
        1 => {
            let item = filtered_items[res.mappings[0]
                .item_index
                .expect("we map by name only and have no object-id in refspec")];
            Ok((Some(item.target), Some(item.full_ref_name)))
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
