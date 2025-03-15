use std::collections::HashSet;

use gix_hash::ObjectId;
use gix_index::entry::Stage;
use gix_revision::spec::parse::{
    delegate,
    delegate::{PeelTo, Traversal},
};

use crate::{
    bstr::{BStr, ByteSlice},
    ext::ObjectIdExt,
    object,
    revision::spec::parse::{
        delegate::{handle_errors_and_replacements, peel, Replacements},
        Delegate, Error,
    },
    Object,
};

impl delegate::Navigate for Delegate<'_> {
    fn traverse(&mut self, kind: Traversal) -> Option<()> {
        self.unset_disambiguate_call();
        self.follow_refs_to_objects_if_needed()?;

        let mut replacements = Replacements::default();
        let mut errors = Vec::new();
        let objs = self.objs[self.idx].as_mut()?;
        let repo = self.repo;

        for obj in objs.iter() {
            match kind {
                Traversal::NthParent(num) => {
                    match self.repo.find_object(*obj).map_err(Error::from).and_then(|obj| {
                        obj.try_into_commit().map_err(|err| {
                            let object::try_into::Error { actual, expected, id } = err;
                            Error::ObjectKind {
                                oid: id.attach(repo).shorten_or_id(),
                                actual,
                                expected,
                            }
                        })
                    }) {
                        Ok(commit) => match commit.parent_ids().nth(num.saturating_sub(1)) {
                            Some(id) => replacements.push((commit.id, id.detach())),
                            None => errors.push((
                                commit.id,
                                Error::ParentOutOfRange {
                                    oid: commit.id().shorten_or_id(),
                                    desired: num,
                                    available: commit.parent_ids().count(),
                                },
                            )),
                        },
                        Err(err) => errors.push((*obj, err)),
                    }
                }
                Traversal::NthAncestor(num) => {
                    let id = obj.attach(repo);
                    match id
                        .ancestors()
                        .first_parent_only()
                        .all()
                        .expect("cannot fail without sorting")
                        .skip(num)
                        .find_map(Result::ok)
                    {
                        Some(commit) => replacements.push((*obj, commit.id)),
                        None => errors.push((
                            *obj,
                            Error::AncestorOutOfRange {
                                oid: id.shorten_or_id(),
                                desired: num,
                                available: id
                                    .ancestors()
                                    .first_parent_only()
                                    .all()
                                    .expect("cannot fail without sorting")
                                    .skip(1)
                                    .count(),
                            },
                        )),
                    }
                }
            }
        }

        handle_errors_and_replacements(&mut self.err, objs, errors, &mut replacements)
    }

    fn peel_until(&mut self, kind: PeelTo<'_>) -> Option<()> {
        self.unset_disambiguate_call();
        self.follow_refs_to_objects_if_needed()?;

        let mut replacements = Replacements::default();
        let mut errors = Vec::new();
        let objs = self.objs[self.idx].as_mut()?;
        let repo = self.repo;

        match kind {
            PeelTo::ValidObject => {
                for obj in objs.iter() {
                    match repo.find_object(*obj) {
                        Ok(_) => {}
                        Err(err) => {
                            errors.push((*obj, err.into()));
                        }
                    };
                }
            }
            PeelTo::ObjectKind(kind) => {
                let peel = |obj| peel(repo, obj, kind);
                for obj in objs.iter() {
                    match peel(obj) {
                        Ok(replace) => replacements.push((*obj, replace)),
                        Err(err) => errors.push((*obj, err)),
                    }
                }
            }
            PeelTo::Path(path) => {
                let lookup_path = |obj: &ObjectId| {
                    let tree_id = peel(repo, obj, gix_object::Kind::Tree)?;
                    if path.is_empty() {
                        return Ok((tree_id, gix_object::tree::EntryKind::Tree.into()));
                    }
                    let mut tree = repo.find_object(tree_id)?.into_tree();
                    let entry =
                        tree.peel_to_entry_by_path(gix_path::from_bstr(path))?
                            .ok_or_else(|| Error::PathNotFound {
                                path: path.into(),
                                object: obj.attach(repo).shorten_or_id(),
                                tree: tree_id.attach(repo).shorten_or_id(),
                            })?;
                    Ok((entry.object_id(), entry.mode()))
                };
                for obj in objs.iter() {
                    match lookup_path(obj) {
                        Ok((replace, mode)) => {
                            if !path.is_empty() {
                                // Technically this is letting the last one win, but so be it.
                                self.paths[self.idx] = Some((path.to_owned(), mode));
                            }
                            replacements.push((*obj, replace));
                        }
                        Err(err) => errors.push((*obj, err)),
                    }
                }
            }
            PeelTo::RecursiveTagObject => {
                for oid in objs.iter() {
                    match oid.attach(repo).object().and_then(Object::peel_tags_to_end) {
                        Ok(obj) => replacements.push((*oid, obj.id)),
                        Err(err) => errors.push((*oid, err.into())),
                    }
                }
            }
        }

        handle_errors_and_replacements(&mut self.err, objs, errors, &mut replacements)
    }

    fn find(&mut self, regex: &BStr, negated: bool) -> Option<()> {
        self.unset_disambiguate_call();
        self.follow_refs_to_objects_if_needed()?;

        #[cfg(not(feature = "revparse-regex"))]
        let matches = |message: &BStr| -> bool { message.contains_str(regex) ^ negated };
        #[cfg(feature = "revparse-regex")]
        let matches = match regex::bytes::Regex::new(regex.to_str_lossy().as_ref()) {
            Ok(compiled) => {
                let needs_regex = regex::escape(compiled.as_str()) != regex;
                move |message: &BStr| -> bool {
                    if needs_regex {
                        compiled.is_match(message) ^ negated
                    } else {
                        message.contains_str(regex) ^ negated
                    }
                }
            }
            Err(err) => {
                self.err.push(err.into());
                return None;
            }
        };

        match self.objs[self.idx].as_mut() {
            Some(objs) => {
                let repo = self.repo;
                let mut errors = Vec::new();
                let mut replacements = Replacements::default();
                for oid in objs.iter() {
                    match oid
                        .attach(repo)
                        .ancestors()
                        .sorting(crate::revision::walk::Sorting::ByCommitTime(Default::default()))
                        .all()
                    {
                        Ok(iter) => {
                            let mut matched = false;
                            let mut count = 0;
                            let commits = iter.map(|res| {
                                res.map_err(Error::from).and_then(|commit| {
                                    commit.id().object().map_err(Error::from).map(Object::into_commit)
                                })
                            });
                            for commit in commits {
                                count += 1;
                                match commit {
                                    Ok(commit) => {
                                        if matches(commit.message_raw_sloppy()) {
                                            replacements.push((*oid, commit.id));
                                            matched = true;
                                            break;
                                        }
                                    }
                                    Err(err) => errors.push((*oid, err)),
                                }
                            }
                            if !matched {
                                errors.push((
                                    *oid,
                                    Error::NoRegexMatch {
                                        regex: regex.into(),
                                        commits_searched: count,
                                        oid: oid.attach(repo).shorten_or_id(),
                                    },
                                ));
                            }
                        }
                        Err(err) => errors.push((*oid, err.into())),
                    }
                }
                handle_errors_and_replacements(&mut self.err, objs, errors, &mut replacements)
            }
            None => match self.repo.references() {
                Ok(references) => match references.all() {
                    Ok(references) => {
                        match self
                            .repo
                            .rev_walk(
                                references
                                    .peeled()
                                    .ok()?
                                    .filter_map(Result::ok)
                                    .filter(|r| r.id().header().ok().is_some_and(|obj| obj.kind().is_commit()))
                                    .filter_map(|r| r.detach().peeled),
                            )
                            .sorting(crate::revision::walk::Sorting::ByCommitTime(Default::default()))
                            .all()
                        {
                            Ok(iter) => {
                                let mut matched = false;
                                let mut count = 0;
                                let commits = iter.map(|res| {
                                    res.map_err(Error::from).and_then(|commit| {
                                        commit.id().object().map_err(Error::from).map(Object::into_commit)
                                    })
                                });
                                for commit in commits {
                                    count += 1;
                                    match commit {
                                        Ok(commit) => {
                                            if matches(commit.message_raw_sloppy()) {
                                                self.objs[self.idx]
                                                    .get_or_insert_with(HashSet::default)
                                                    .insert(commit.id);
                                                matched = true;
                                                break;
                                            }
                                        }
                                        Err(err) => self.err.push(err),
                                    }
                                }
                                if matched {
                                    Some(())
                                } else {
                                    self.err.push(Error::NoRegexMatchAllRefs {
                                        regex: regex.into(),
                                        commits_searched: count,
                                    });
                                    None
                                }
                            }
                            Err(err) => {
                                self.err.push(err.into());
                                None
                            }
                        }
                    }
                    Err(err) => {
                        self.err.push(err.into());
                        None
                    }
                },
                Err(err) => {
                    self.err.push(err.into());
                    None
                }
            },
        }
    }

    fn index_lookup(&mut self, path: &BStr, stage: u8) -> Option<()> {
        let stage = match stage {
            0 => Stage::Unconflicted,
            1 => Stage::Base,
            2 => Stage::Ours,
            3 => Stage::Theirs,
            _ => unreachable!(
                "BUG: driver will not pass invalid stages (and it uses integer to avoid gix-index as dependency)"
            ),
        };
        self.unset_disambiguate_call();
        match self.repo.index() {
            Ok(index) => match index.entry_by_path_and_stage(path, stage) {
                Some(entry) => {
                    self.objs[self.idx]
                        .get_or_insert_with(HashSet::default)
                        .insert(entry.id);

                    self.paths[self.idx] = Some((
                        path.to_owned(),
                        entry
                            .mode
                            .to_tree_entry_mode()
                            .unwrap_or(gix_object::tree::EntryKind::Blob.into()),
                    ));
                    Some(())
                }
                None => {
                    let stage_hint = [Stage::Unconflicted, Stage::Base, Stage::Ours]
                        .iter()
                        .filter(|our_stage| **our_stage != stage)
                        .find_map(|stage| index.entry_index_by_path_and_stage(path, *stage).map(|_| *stage));
                    let exists = self
                        .repo
                        .workdir()
                        .is_some_and(|root| root.join(gix_path::from_bstr(path)).exists());
                    self.err.push(Error::IndexLookup {
                        desired_path: path.into(),
                        desired_stage: stage,
                        exists,
                        stage_hint,
                    });
                    None
                }
            },
            Err(err) => {
                self.err.push(err.into());
                None
            }
        }
    }
}
