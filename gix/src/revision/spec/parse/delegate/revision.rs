use gix_error::Result;
use gix_error::{ErrorExt, message};
use gix_hash::ObjectId;
use gix_revision::spec::parse::{
    delegate,
    delegate::{ReflogLookup, SiblingBranch},
};
use std::collections::HashSet;

use crate::revision::spec::parse::error;
use crate::{
    bstr::{BStr, BString, ByteSlice},
    ext::ReferenceExt,
    remote,
    revision::spec::parse::{Delegate, RefsHint},
};

impl delegate::Revision for Delegate<'_> {
    fn find_ref(&mut self, name: &BStr) -> Result<()> {
        self.unset_disambiguate_call();
        if self.refs[self.idx].is_some() {
            // A rejected ref/object collision must not succeed via the parser's reference-only fallback.
            return Err(message("A reference was already matched by the object prefix").raise());
        }
        let r = self
            .repo
            .refs
            .find(name)
            .map_err(|err| error::with_missing_reference(err.into_exn()))?;
        assert!(self.refs[self.idx].is_none(), "BUG: cannot set the same ref twice");
        self.refs[self.idx] = Some(r);
        Ok(())
    }

    fn disambiguate_prefix(
        &mut self,
        prefix: gix_hash::Prefix,
        _must_be_commit: Option<delegate::PrefixHint<'_>>,
    ) -> Result<()> {
        self.last_call_was_disambiguate_prefix[self.idx] = true;
        let mut candidates = Some(HashSet::default());
        self.prefix[self.idx] = Some(prefix);

        let empty_tree_id = gix_hash::ObjectId::empty_tree(prefix.as_oid().kind());
        let ok = if prefix.as_oid() == empty_tree_id {
            candidates.as_mut().expect("set").insert(empty_tree_id);
            Ok(Some(Err(())))
        } else {
            self.repo.objects.lookup_prefix(prefix, candidates.as_mut())
        }?;

        match ok {
            None => Err(message!("An object prefixed {prefix} could not be found").raise()),
            Some(Ok(_) | Err(())) => {
                assert!(self.objs[self.idx].is_none(), "BUG: cannot set the same prefix twice");
                let candidates = candidates.expect("set above");
                match self.opts.refs_hint {
                    RefsHint::PreferObjectOnFullLengthHexShaUseRefOtherwise
                        if prefix.hex_len() == candidates.iter().next().expect("at least one").kind().len_in_hex() =>
                    {
                        let objs = to_sorted_vec(candidates);
                        self.ambiguous_objects[self.idx] = Some(objs.clone());
                        self.objs[self.idx] = Some(objs);
                        Ok(())
                    }
                    RefsHint::PreferObject => {
                        let objs = to_sorted_vec(candidates);
                        self.ambiguous_objects[self.idx] = Some(objs.clone());
                        self.objs[self.idx] = Some(objs);
                        Ok(())
                    }
                    RefsHint::PreferRef | RefsHint::PreferObjectOnFullLengthHexShaUseRefOtherwise | RefsHint::Fail => {
                        match self.repo.refs.find(&prefix.to_string()) {
                            Ok(ref_) => {
                                assert!(self.refs[self.idx].is_none(), "BUG: cannot set the same ref twice");
                                if self.opts.refs_hint == RefsHint::Fail {
                                    let reference = ref_.name.clone();
                                    self.refs[self.idx] = Some(ref_);
                                    Err(error::ambiguous_ref_and_object(
                                        to_sorted_vec(candidates),
                                        prefix,
                                        reference,
                                        self.repo,
                                    )
                                    .raise())
                                } else {
                                    self.refs[self.idx] = Some(ref_);
                                    Ok(())
                                }
                            }
                            Err(_) => {
                                let objs = to_sorted_vec(candidates);
                                self.ambiguous_objects[self.idx] = Some(objs.clone());
                                self.objs[self.idx] = Some(objs);
                                Ok(())
                            }
                        }
                    }
                }
            }
        }
    }

    fn reflog(&mut self, query: ReflogLookup) -> Result<()> {
        self.unset_disambiguate_call();
        let r = match &mut self.refs[self.idx] {
            Some(r) => r.clone().attach(self.repo),
            val @ None => match self.repo.head().map(crate::Head::try_into_referent) {
                Ok(Some(r)) => {
                    *val = Some(r.clone().detach());
                    r
                }
                Ok(None) => return Err(message("Unborn heads do not have a reflog yet").raise()),
                Err(err) => return Err(error::with_missing_reference(err.raise_erased()).into()),
            },
        };

        let mut platform = r.log_iter();
        match platform.rev().ok().flatten() {
            Some(mut it) => match query {
                ReflogLookup::Date(date) => {
                    let mut last = None;
                    let id_to_insert = match it
                        .filter_map(std::result::Result::ok)
                        .inspect(|d| {
                            last = Some(if d.previous_oid.is_null() {
                                d.new_oid
                            } else {
                                d.previous_oid
                            });
                        })
                        .find(|l| l.signature.time.seconds <= date.seconds)
                    {
                        Some(closest_line) => closest_line.new_oid,
                        None => match last {
                            None => return Err(message("Reflog does not contain any entries").raise()),
                            Some(id) => id,
                        },
                    };
                    let objs = self.objs[self.idx].get_or_insert_with(Vec::new);
                    if !objs.contains(&id_to_insert) {
                        objs.push(id_to_insert);
                    }
                    Ok(())
                }
                ReflogLookup::Entry(no) => match it.nth(no).and_then(std::result::Result::ok) {
                    Some(line) => {
                        let objs = self.objs[self.idx].get_or_insert_with(Vec::new);
                        if !objs.contains(&line.new_oid) {
                            objs.push(line.new_oid);
                        }
                        Ok(())
                    }
                    None => Err(message!(
                        "Reference '{name}' has {available} ref-log entries and entry number {no} is out of range",
                        name = r.name(),
                        available = platform.rev().ok().flatten().map_or(0, Iterator::count)
                    )
                    .raise()),
                },
            },
            None => Err(message!(
                "Reference {reference:?} does not have a reference log, cannot {action}",
                action = match query {
                    ReflogLookup::Entry(_) => "lookup reflog entry by index",
                    ReflogLookup::Date(_) => "lookup reflog entry by date",
                },
                reference = r.name().as_bstr()
            )
            .raise()),
        }
    }

    fn nth_checked_out_branch(&mut self, branch_no: usize) -> Result<()> {
        self.unset_disambiguate_call();
        fn prior_checkouts_iter<'a>(
            platform: &'a mut gix_ref::file::log::iter::Platform<'static, '_>,
        ) -> Result<impl Iterator<Item = (BString, ObjectId)> + 'a> {
            match platform.rev().ok().flatten() {
                Some(log) => Ok(log.filter_map(std::result::Result::ok).filter_map(|line| {
                    line.message
                        .strip_prefix(b"checkout: moving from ")
                        .and_then(|from_to| from_to.find(" to ").map(|pos| &from_to[..pos]))
                        .map(|from_branch| (from_branch.into(), line.previous_oid))
                })),
                None => Err(message(
                    "Reference HEAD does not have a reference log, cannot search prior checked out branch",
                )
                .raise()),
            }
        }

        let head = match self.repo.head() {
            Ok(head) => head,
            Err(err) => return Err(error::with_missing_reference(err.raise_erased()).into()),
        };
        let ok = prior_checkouts_iter(&mut head.log_iter()).map(|mut it| it.nth(branch_no.saturating_sub(1)))?;
        match ok {
            Some((ref_name, id)) => {
                let id = match self.repo.find_reference(ref_name.as_bstr()) {
                    Ok(mut r) => {
                        let id = r.peel_to_id().map_or(id, crate::Id::detach);
                        self.refs[self.idx] = Some(r.detach());
                        id
                    }
                    Err(err) if err.is_not_found() => match ObjectId::from_hex(ref_name.as_ref()) {
                        Ok(id) if id.kind() == self.repo.object_hash() => id,
                        _ => {
                            return Err(message!(
                                "Previous checkout '{name}' does not resolve to an existing revision",
                                name = ref_name.as_bstr()
                            )
                            .raise());
                        }
                    },
                    Err(err) => return Err(err.raise()),
                };
                let objs = self.objs[self.idx].get_or_insert_with(Vec::new);
                if !objs.contains(&id) {
                    objs.push(id);
                }
                Ok(())
            }
            None => Err(message!(
                "HEAD has {available} prior checkouts and checkout number {branch_no} is out of range",
                available = prior_checkouts_iter(&mut head.log_iter()).map_or(0, Iterator::count)
            )
            .raise()),
        }
    }

    fn sibling_branch(&mut self, kind: SiblingBranch) -> Result<()> {
        self.unset_disambiguate_call();
        let reference = match &mut self.refs[self.idx] {
            val @ None => match self.repo.head().map(crate::Head::try_into_referent) {
                Ok(Some(r)) => {
                    *val = Some(r.clone().detach());
                    r
                }
                Ok(None) => {
                    return Err(message("Unborn heads cannot have push or upstream tracking branches").raise());
                }
                Err(err) => {
                    return Err(error::with_missing_reference(err.raise_erased()).into());
                }
            },
            Some(r) => r.clone().attach(self.repo),
        };
        let direction = match kind {
            SiblingBranch::Upstream => remote::Direction::Fetch,
            SiblingBranch::Push => remote::Direction::Push,
        };
        let make_message = || {
            message!(
                "Error when obtaining {direction} tracking branch for {name}",
                name = reference.name().as_bstr(),
                direction = direction.as_str()
            )
        };
        match reference.remote_tracking_ref_name(direction) {
            None => self.delayed_errors.push(
                message!(
                    "Branch named {name} does not have a {direction} tracking branch configured",
                    name = reference.name().as_bstr(),
                    direction = direction.as_str()
                )
                .raise_erased(),
            ),
            Some(Err(err)) => self.delayed_errors.push(err.and_raise_typed(make_message()).erased()),
            Some(Ok(name)) => match self.repo.find_reference(name.as_ref()) {
                Err(err) => self.delayed_errors.push(
                    error::with_missing_reference(err.raise_erased())
                        .raise(make_message())
                        .erased(),
                ),
                Ok(r) => {
                    self.refs[self.idx] = r.inner.into();
                    return Ok(());
                }
            },
        }
        Err(message!("Couldn't find sibling of {kind:?}").raise())
    }
}

fn to_sorted_vec(objs: HashSet<ObjectId>) -> Vec<ObjectId> {
    let mut v: Vec<_> = objs.into_iter().collect();
    v.sort();
    v
}
