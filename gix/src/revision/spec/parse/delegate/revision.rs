use gix_error::{bail, message, ErrorExt, Exn, ResultExt, Something};
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
    revision::spec::parse::{Delegate, Error, RefsHint},
};

impl delegate::Revision for Delegate<'_> {
    fn find_ref(&mut self, name: &BStr) -> Result<(), Exn> {
        self.unset_disambiguate_call();
        if self.has_err() && self.refs[self.idx].is_some() {
            bail!(Something)
        }
        match self.repo.refs.find(name) {
            Ok(r) => {
                assert!(self.refs[self.idx].is_none(), "BUG: cannot set the same ref twice");
                self.refs[self.idx] = Some(r);
                Ok(())
            }
            Err(err) => {
                bail!(err.raise_something())
            }
        }
    }

    fn disambiguate_prefix(
        &mut self,
        prefix: gix_hash::Prefix,
        _must_be_commit: Option<delegate::PrefixHint<'_>>,
    ) -> Result<(), Exn> {
        self.last_call_was_disambiguate_prefix[self.idx] = true;
        let mut candidates = Some(HashSet::default());
        self.prefix[self.idx] = Some(prefix);

        let empty_tree_id = gix_hash::ObjectId::empty_tree(prefix.as_oid().kind());
        let ok = if prefix.as_oid() == empty_tree_id {
            candidates.as_mut().expect("set").insert(empty_tree_id);
            Ok(Some(Err(())))
        } else {
            self.repo.objects.lookup_prefix(prefix, candidates.as_mut())
        }
        .or_something()?;

        match ok {
            None => Err(message!("An object prefixed {} could not be found", prefix).raise_something()),
            Some(Ok(_) | Err(())) => {
                assert!(self.objs[self.idx].is_none(), "BUG: cannot set the same prefix twice");
                let candidates = candidates.expect("set above");
                match self.opts.refs_hint {
                    RefsHint::PreferObjectOnFullLengthHexShaUseRefOtherwise
                        if prefix.hex_len() == candidates.iter().next().expect("at least one").kind().len_in_hex() =>
                    {
                        self.ambiguous_objects[self.idx] = Some(candidates.clone());
                        self.objs[self.idx] = Some(candidates);
                        Ok(())
                    }
                    RefsHint::PreferObject => {
                        self.ambiguous_objects[self.idx] = Some(candidates.clone());
                        self.objs[self.idx] = Some(candidates);
                        Ok(())
                    }
                    RefsHint::PreferRef | RefsHint::PreferObjectOnFullLengthHexShaUseRefOtherwise | RefsHint::Fail => {
                        match self.repo.refs.find(&prefix.to_string()) {
                            Ok(ref_) => {
                                assert!(self.refs[self.idx].is_none(), "BUG: cannot set the same ref twice");
                                if self.opts.refs_hint == RefsHint::Fail {
                                    self.refs[self.idx] = Some(ref_.clone());
                                    self.delayed_errors.push(
                                        message!(
                                        "The short hash {prefix} matched both the reference {name} and at least one object",
                                        name = ref_.name
                                    )
                                        .into(),
                                    );
                                    Err(error::ambiguous(candidates, prefix, self.repo).raise_something())
                                } else {
                                    self.refs[self.idx] = Some(ref_);
                                    Ok(())
                                }
                            }
                            Err(_) => {
                                self.ambiguous_objects[self.idx] = Some(candidates.clone());
                                self.objs[self.idx] = Some(candidates);
                                Ok(())
                            }
                        }
                    }
                }
            }
        }
    }

    fn reflog(&mut self, query: ReflogLookup) -> Result<(), Exn> {
        self.unset_disambiguate_call();
        let r = match &mut self.refs[self.idx] {
            Some(r) => r.clone().attach(self.repo),
            val @ None => match self.repo.head().map(crate::Head::try_into_referent) {
                Ok(Some(r)) => {
                    *val = Some(r.clone().detach());
                    r
                }
                Ok(None) => return Err(message!("Unborn heads do not have a reflog yet").raise_something()),
                Err(err) => return Err(err.raise_something()),
            },
        };

        let mut platform = r.log_iter();
        match platform.rev().ok().flatten() {
            Some(mut it) => match query {
                ReflogLookup::Date(date) => {
                    let mut last = None;
                    let id_to_insert = match it
                        .filter_map(Result::ok)
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
                            None => return Err(message!("Reflog does not contain any entries").raise_something()),
                            Some(id) => id,
                        },
                    };
                    self.objs[self.idx]
                        .get_or_insert_with(HashSet::default)
                        .insert(id_to_insert);
                    Ok(())
                }
                ReflogLookup::Entry(no) => match it.nth(no).and_then(Result::ok) {
                    Some(line) => {
                        self.objs[self.idx]
                            .get_or_insert_with(HashSet::default)
                            .insert(line.new_oid);
                        Ok(())
                    }
                    None => Err(message!(
                        "Reference {r:?} has {available} ref-log entries and entry number {no} is out of range",
                        available = platform.rev().ok().flatten().map_or(0, Iterator::count)
                    )
                    .raise_something()),
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
            .raise_something()),
        }
    }

    fn nth_checked_out_branch(&mut self, branch_no: usize) -> Result<(), Exn> {
        self.unset_disambiguate_call();
        fn prior_checkouts_iter<'a>(
            platform: &'a mut gix_ref::file::log::iter::Platform<'static, '_>,
        ) -> Result<impl Iterator<Item = (BString, ObjectId)> + 'a, Error> {
            match platform.rev().ok().flatten() {
                Some(log) => Ok(log.filter_map(Result::ok).filter_map(|line| {
                    line.message
                        .strip_prefix(b"checkout: moving from ")
                        .and_then(|from_to| from_to.find(" to ").map(|pos| &from_to[..pos]))
                        .map(|from_branch| (from_branch.into(), line.previous_oid))
                })),
                None => Err(message!(
                    "Reference HEAD does not have a reference log, cannot search prior checked out branch",
                )
                .raise()
                .into_error()
                .into()),
            }
        }

        let head = match self.repo.head() {
            Ok(head) => head,
            Err(err) => return Err(err.raise_something()),
        };
        let ok = prior_checkouts_iter(&mut head.log_iter())
            .map(|mut it| it.nth(branch_no.saturating_sub(1)))
            .or_something()?;
        match ok {
            Some((ref_name, id)) => {
                let id = match self.repo.find_reference(ref_name.as_bstr()) {
                    Ok(mut r) => {
                        let id = r.peel_to_id().map(crate::Id::detach).unwrap_or(id);
                        self.refs[self.idx] = Some(r.detach());
                        id
                    }
                    Err(_) => id,
                };
                self.objs[self.idx].get_or_insert_with(HashSet::default).insert(id);
                Ok(())
            }
            None => Err(message!(
                "HEAD has {available} prior checkouts and checkout number {branch_no} is out of range",
                available = prior_checkouts_iter(&mut head.log_iter())
                    .map(Iterator::count)
                    .unwrap_or(0)
            )
            .raise_something()),
        }
    }

    fn sibling_branch(&mut self, kind: SiblingBranch) -> Result<(), Exn> {
        self.unset_disambiguate_call();
        let reference = match &mut self.refs[self.idx] {
            val @ None => match self.repo.head().map(crate::Head::try_into_referent) {
                Ok(Some(r)) => {
                    *val = Some(r.clone().detach());
                    r
                }
                Ok(None) => {
                    return Err(
                        message!("Unborn heads cannot have push or upstream tracking branches").raise_something()
                    )
                }
                Err(err) => {
                    return Err(err.raise_something());
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
                .into(),
            ),
            Some(Err(err)) => self
                .delayed_errors
                .push(err.raise().raise(make_message()).into_error().into()),
            Some(Ok(name)) => match self.repo.find_reference(name.as_ref()) {
                Err(err) => self
                    .delayed_errors
                    .push(err.raise().raise(make_message()).into_error().into()),
                Ok(r) => {
                    self.refs[self.idx] = r.inner.into();
                    return Ok(());
                }
            },
        }
        Err(message!("Couldn't find sibling of {kind:?}", kind = kind).raise_something())
    }
}
