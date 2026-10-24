use gix_error::Result;
use gix_error::{ErrorExt, bail, message};
use gix_hash::ObjectId;
use gix_ref::file::ReferenceExt as _;
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
            bail!("A reference was already matched by the object prefix");
        }
        let r = self
            .repo
            .refs
            .find_resolved(name)
            // Keep deferred peel diagnostics when no candidate can be resolved.
            .or_else(|_| self.repo.refs.find(name))
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
        // Short prefixes cannot encode their hash kind; use the repository's object format.
        let prefix = if prefix.as_oid().kind() != self.repo.object_hash() {
            let mut object_id = ObjectId::null(self.repo.object_hash());
            let len = prefix.hex_len().div_ceil(2);
            if len > object_id.as_bytes().len() {
                bail!("Object prefix {prefix} is too long for the repository's hash kind");
            }
            object_id.as_mut_slice()[..len].copy_from_slice(&prefix.as_oid().as_bytes()[..len]);
            gix_hash::Prefix::new(&object_id, prefix.hex_len())?
        } else {
            prefix
        };
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
                        match self.repo.refs.find_resolved(&prefix.to_string()) {
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
        let mut r = match &mut self.refs[self.idx] {
            Some(r) => r.clone().attach(self.repo),
            val @ None => match self.repo.head().map(crate::Head::try_into_referent) {
                Ok(Some(r)) => {
                    *val = Some(r.clone().detach());
                    r
                }
                Ok(None) => bail!("Unborn heads do not have a reflog yet"),
                Err(err) => bail!(error::with_missing_reference(err.raise_erased())),
            },
        };

        let mut resolved = r.clone();
        let current_id = resolved
            .follow_to_object()
            .map_err(|err| error::with_missing_reference(err.into_exn()))?
            .detach();
        if !r.log_exists()? {
            r = resolved;
        }
        let mut platform = r.log_iter();
        let Some(it) = platform.rev().ok().flatten() else {
            bail!(
                "Reference {reference:?} does not have a reference log, cannot {action}",
                action = match query {
                    ReflogLookup::Entry(_) => "lookup reflog entry by index",
                    ReflogLookup::Date(_) => "lookup reflog entry by date",
                },
                reference = r.name().as_bstr()
            );
        };
        let mut available = 0;
        let mut previous_id = ObjectId::null(self.repo.object_hash());
        let mut oldest_id = None;
        let mut selected_id = None;
        for line in it.filter_map(std::result::Result::ok) {
            let matches = match query {
                ReflogLookup::Entry(no) => available == no,
                ReflogLookup::Date(date) => line.signature.time.seconds <= date.seconds,
            };
            if matches {
                // A discontinuity after a deletion or a stale newest log keeps the current ref value.
                let exact_date =
                    matches!(query, ReflogLookup::Date(date) if line.signature.time.seconds == date.seconds);
                selected_id = Some(if !previous_id.is_null() || exact_date {
                    line.new_oid
                } else {
                    current_id
                });
                break;
            }
            available += 1;
            previous_id = line.previous_oid;
            oldest_id = Some(if line.previous_oid.is_null() {
                line.new_oid
            } else {
                line.previous_oid
            });
        }
        let id_to_insert = match (selected_id, query) {
            (Some(id), _) => id,
            (None, ReflogLookup::Entry(no)) if no == available && !previous_id.is_null() => previous_id,
            (None, ReflogLookup::Entry(0)) => current_id,
            (None, ReflogLookup::Entry(no)) => {
                bail!(
                    "Reference '{name}' has {available} ref-log entries and entry number {no} is out of range",
                    name = r.name()
                );
            }
            (None, ReflogLookup::Date(_)) => match oldest_id {
                Some(id) => id,
                None => bail!("Reflog does not contain any entries"),
            },
        };
        let objs = self.objs[self.idx].get_or_insert_with(Vec::new);
        if !objs.contains(&id_to_insert) {
            objs.push(id_to_insert);
        }
        Ok(())
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
            Err(err) => bail!(error::with_missing_reference(err.raise_erased())),
        };
        let ok = prior_checkouts_iter(&mut head.log_iter()).map(|mut it| it.nth(branch_no.saturating_sub(1)))?;
        match ok {
            Some((ref_name, _)) => {
                let id = match self.repo.refs.find_resolved(ref_name.as_bstr()) {
                    Ok(mut r) => {
                        let packed = self.repo.refs.cached_packed_buffer()?;
                        let id = r.follow_to_object_packed(&self.repo.refs, packed.as_ref().map(|b| &***b))?;
                        self.refs[self.idx] = Some(r);
                        id
                    }
                    Err(err) => match ObjectId::from_hex(ref_name.as_ref()) {
                        Ok(id) if id.kind() == self.repo.object_hash() => id,
                        _ if err.is_not_found() => {
                            bail!(
                                "Previous checkout '{name}' does not resolve to an existing revision",
                                name = ref_name.as_bstr()
                            );
                        }
                        _ => return Err(err),
                    },
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
        let mut reference = match &mut self.refs[self.idx] {
            val @ None => match self.repo.head().map(crate::Head::try_into_referent) {
                Ok(Some(r)) => {
                    *val = Some(r.clone().detach());
                    r
                }
                Ok(None) => {
                    bail!("Unborn heads cannot have push or upstream tracking branches");
                }
                Err(err) => {
                    bail!(error::with_missing_reference(err.raise_erased()));
                }
            },
            Some(r) => r.clone().attach(self.repo),
        };
        if reference.name() == "HEAD" {
            reference
                .follow_to_object()
                .map_err(|err| error::with_missing_reference(err.into_exn()))?;
        }
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
