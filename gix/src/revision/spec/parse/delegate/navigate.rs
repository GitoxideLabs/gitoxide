use gix_error::{bail, message, ErrorExt, Exn, OptionExt, ResultExt};
use gix_hash::ObjectId;
use gix_index::entry::Stage;
use gix_revision::spec::parse::{
    delegate,
    delegate::{PeelTo, Traversal},
};

use crate::revision::spec::parse::delegate::peel;
use crate::{
    bstr::{BStr, ByteSlice},
    ext::ObjectIdExt,
    object,
    revision::spec::parse::{delegate::Replacements, Delegate},
    Object,
};

impl delegate::Navigate for Delegate<'_> {
    fn traverse(&mut self, kind: Traversal) -> Result<(), Exn> {
        self.unset_disambiguate_call();
        self.follow_refs_to_objects_if_needed_delay_errors();

        let mut replacements = Replacements::default();
        let mut errors = Vec::<(ObjectId, Exn)>::new();
        let objs = match self.objs[self.idx].as_mut() {
            Some(objs) => objs,
            None => {
                bail!(message!("Tried to navigate the commit-graph without providing an anchor first").erased())
            }
        };
        let repo = self.repo;

        for obj in objs.iter() {
            match kind {
                Traversal::NthParent(num) => {
                    match self.repo.find_object(*obj).or_erased().and_then(|obj| {
                        obj.try_into_commit().map_err(|err| {
                            let object::try_into::Error { actual, expected, id } = err;
                            message!(
                                "Object {oid} was a {actual}, but needed it to be a {expected}",
                                oid = id.attach(repo).shorten_or_id(),
                            )
                            .erased()
                        })
                    }) {
                        Ok(commit) => match commit.parent_ids().nth(num.saturating_sub(1)) {
                            Some(id) => replacements.push((commit.id, id.detach())),
                            None => errors.push((
                                commit.id,
                                message!(
                                    "Commit {oid} has {available} parents and parent number {desired} is out of range",
                                    oid = commit.id().shorten_or_id(),
                                    desired = num,
                                    available = commit.parent_ids().count(),
                                )
                                .erased(),
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
                                message!("Commit {oid} has {available} ancestors along the first parent and ancestor number {num} is out of range",
                                    oid = id.shorten_or_id(),
                                    available = id
                                        .ancestors()
                                        .first_parent_only()
                                        .all()
                                        .expect("cannot fail without sorting")
                                        .skip(1)
                                        .count()
                                ).erased()
                        )),
                    }
                }
            }
        }

        handle_errors_and_replacements(&mut self.delayed_errors, objs, errors, &mut replacements)
    }

    fn peel_until(&mut self, kind: PeelTo<'_>) -> Result<(), Exn> {
        self.unset_disambiguate_call();
        self.follow_refs_to_objects_if_needed_delay_errors();

        let mut replacements = Replacements::default();
        let mut errors = Vec::<(ObjectId, Exn)>::new();
        let objs = self.objs[self.idx]
            .as_mut()
            .ok_or_raise_erased(|| message!("Couldn't get object at internal index {idx}", idx = self.idx))?;
        let repo = self.repo;

        match kind {
            PeelTo::ValidObject => {
                for obj in objs.iter() {
                    if let Err(err) = repo.find_object(*obj) {
                        errors.push((*obj, err.erased()));
                    }
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
                        return Ok::<_, Exn>((tree_id, gix_object::tree::EntryKind::Tree.into()));
                    }
                    let mut tree = repo.find_object(tree_id).or_erased()?.into_tree();
                    let entry = tree
                        .peel_to_entry_by_path(gix_path::from_bstr(path))
                        .or_erased()?
                        .ok_or_raise_erased(|| {
                            message!(
                                "Could not find path {path:?} in tree {tree} of parent object {object}",
                                path = path,
                                object = obj.attach(repo).shorten_or_id(),
                                tree = tree_id.attach(repo).shorten_or_id(),
                            )
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
                        Err(err) => errors.push((*oid, err.erased())),
                    }
                }
            }
        }

        handle_errors_and_replacements(&mut self.delayed_errors, objs, errors, &mut replacements)
    }

    fn find(&mut self, regex: &BStr, negated: bool) -> Result<(), Exn> {
        self.unset_disambiguate_call();
        self.follow_refs_to_objects_if_needed_delay_errors();

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
                bail!(err.erased());
            }
        };

        match self.objs[self.idx].as_mut() {
            Some(objs) => {
                let repo = self.repo;
                let mut errors = Vec::<(ObjectId, Exn)>::new();
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
                                res.map_err(|err| err.erased()).and_then(|commit| {
                                    commit
                                        .id()
                                        .object()
                                        .map_err(|err| err.erased())
                                        .map(Object::into_commit)
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
                                    message!(
                                        "None of {commits_searched} commits from {oid} matched {kind} {regex:?}",
                                        regex = regex,
                                        commits_searched = count,
                                        oid = oid.attach(repo).shorten_or_id(),
                                        kind = if cfg!(feature = "revparse-regex") {
                                            "regex"
                                        } else {
                                            "text"
                                        }
                                    )
                                    .erased(),
                                ));
                            }
                        }
                        Err(err) => errors.push((*oid, err.erased())),
                    }
                }
                handle_errors_and_replacements(&mut self.delayed_errors, objs, errors, &mut replacements)
            }
            None => {
                let references = self.repo.references().or_erased()?;
                let references = references.all().or_erased()?;
                let iter = self
                    .repo
                    .rev_walk(
                        references
                            .peeled()
                            .or_raise_erased(|| message!("Couldn't configure iterator for peeling"))?
                            .filter_map(Result::ok)
                            .filter(|r| r.id().header().ok().is_some_and(|obj| obj.kind().is_commit()))
                            .filter_map(|r| r.detach().peeled),
                    )
                    .sorting(crate::revision::walk::Sorting::ByCommitTime(Default::default()))
                    .all()
                    .or_erased()?;
                let mut matched = false;
                let mut count = 0;
                let commits = iter.map(|res| {
                    res.map_err(|err| err.erased()).and_then(|commit| {
                        commit
                            .id()
                            .object()
                            .map_err(|err| err.erased())
                            .map(Object::into_commit)
                    })
                });
                for commit in commits {
                    count += 1;
                    match commit {
                        Ok(commit) => {
                            if matches(commit.message_raw_sloppy()) {
                                let objs = self.objs[self.idx].get_or_insert_with(Vec::new);
                                if !objs.contains(&commit.id) {
                                    objs.push(commit.id);
                                }
                                matched = true;
                                break;
                            }
                        }
                        Err(err) => self.delayed_errors.push(err),
                    }
                }
                if matched {
                    Ok(())
                } else {
                    Err(message!(
                        "None of {commits_searched} commits reached from all references matched {kind} {regex:?}",
                        regex = regex,
                        commits_searched = count,
                        kind = if cfg!(feature = "revparse-regex") {
                            "regex"
                        } else {
                            "text"
                        }
                    )
                    .erased())
                }
            }
        }
    }

    fn index_lookup(&mut self, path: &BStr, stage: u8) -> Result<(), Exn> {
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
        let index = self.repo.index().or_erased()?;
        match index.entry_by_path_and_stage(path, stage) {
            Some(entry) => {
                let objs = self.objs[self.idx].get_or_insert_with(Vec::new);
                if !objs.contains(&entry.id) {
                    objs.push(entry.id);
                }

                self.paths[self.idx] = Some((
                    path.to_owned(),
                    entry
                        .mode
                        .to_tree_entry_mode()
                        .unwrap_or(gix_object::tree::EntryKind::Blob.into()),
                ));
                Ok(())
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
                Err(message!(
                    "Path {path:?} did not exist in index at stage {desired_stage}{stage_hint}{exists}",
                    exists = if exists {
                        ". It exists on disk"
                    } else {
                        ". It does not exist on disk"
                    },
                    stage_hint = stage_hint
                        .map(|actual| format!(". It does exist at stage {}", actual as u8))
                        .unwrap_or_default(),
                    desired_stage = stage as u8,
                )
                .erased())
            }
        }
    }
}

fn handle_errors_and_replacements(
    delayed_errors: &mut Vec<Exn>,
    objs: &mut Vec<ObjectId>,
    errors: Vec<(ObjectId, Exn)>,
    replacements: &mut Replacements,
) -> Result<(), Exn> {
    if errors.len() == objs.len() {
        delayed_errors.extend(errors.into_iter().map(|(_, err)| err));
        Err(delayed_errors
            .pop()
            .unwrap_or_else(|| message!("BUG: Somehow there was no error but one was expected").erased()))
    } else {
        for (obj, err) in errors {
            if let Some(pos) = objs.iter().position(|o| o == &obj) {
                objs.remove(pos);
            }
            delayed_errors.push(err);
        }
        for (find, replace) in replacements {
            if let Some(pos) = objs.iter().position(|o| o == find) {
                objs.remove(pos);
            }
            if !objs.contains(replace) {
                objs.push(*replace);
            }
        }
        Ok(())
    }
}
