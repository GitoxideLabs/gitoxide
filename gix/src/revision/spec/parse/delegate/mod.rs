use std::collections::HashSet;

use super::{error, Delegate, Error, ObjectKindHint};
use crate::{
    ext::{ObjectIdExt, ReferenceExt},
    Repository,
};
use gix_error::{bail, message, ErrorExt, Exn, Message, ResultExt, Something};
use gix_hash::ObjectId;
use gix_revision::spec::{parse, parse::delegate};
use smallvec::SmallVec;

type Replacements = SmallVec<[(ObjectId, ObjectId); 1]>;

impl<'repo> Delegate<'repo> {
    pub fn new(repo: &'repo Repository, opts: crate::revision::spec::parse::Options) -> Self {
        Delegate {
            refs: Default::default(),
            objs: Default::default(),
            paths: Default::default(),
            ambiguous_objects: Default::default(),
            idx: 0,
            kind: None,
            delayed_errors: Vec::new(),
            prefix: Default::default(),
            last_call_was_disambiguate_prefix: Default::default(),
            opts,
            repo,
        }
    }

    pub fn into_delayed_errors(mut self) -> Option<Error> {
        let repo = self.repo;
        let mut errors = self.delayed_errors;
        for err in self
            .ambiguous_objects
            .iter_mut()
            .zip(self.prefix)
            .filter_map(|(a, b)| a.take().filter(|candidates| candidates.len() > 1).zip(b))
            .map(|(candidates, prefix)| error::ambiguous(candidates, prefix, repo))
            .rev()
        {
            errors.insert(0, err.into());
        }
        (!errors.is_empty()).then(|| Error::from_errors(errors))
    }

    pub fn into_rev_spec(mut self) -> Result<crate::revision::Spec<'repo>, Error> {
        fn zero_or_one_objects_or_ambiguity_err(
            mut candidates: [Option<HashSet<ObjectId>>; 2],
            prefix: [Option<gix_hash::Prefix>; 2],
            repo: &Repository,
        ) -> Result<[Option<ObjectId>; 2], Error> {
            let mut errors = Vec::new();
            let mut out = [None, None];
            for ((candidates, prefix), out) in candidates.iter_mut().zip(prefix).zip(out.iter_mut()) {
                let candidates = candidates.take();
                match candidates {
                    None => *out = None,
                    Some(candidates) => match candidates.len() {
                        0 => {
                            unreachable!("BUG: let's avoid still being around if no candidate matched the requirements")
                        }
                        1 => {
                            *out = candidates.into_iter().next();
                        }
                        _ => {
                            errors.insert(
                                0,
                                error::ambiguous(candidates, prefix.expect("set when obtaining candidates"), repo)
                                    .into(),
                            );
                            return Err(Error::from_errors(errors));
                        }
                    },
                }
            }
            Ok(out)
        }

        fn kind_to_spec(
            kind: Option<gix_revision::spec::Kind>,
            [first, second]: [Option<ObjectId>; 2],
        ) -> Result<gix_revision::Spec, Error> {
            pub fn malformed() -> Message {
                message!("The rev-spec is malformed and misses a ref name")
            }
            use gix_revision::spec::Kind::*;
            Ok(match kind.unwrap_or_default() {
                IncludeReachable => gix_revision::Spec::Include(first.ok_or_else(malformed)?),
                ExcludeReachable => gix_revision::Spec::Exclude(first.ok_or_else(malformed)?),
                RangeBetween => gix_revision::Spec::Range {
                    from: first.ok_or_else(malformed)?,
                    to: second.ok_or_else(malformed)?,
                },
                ReachableToMergeBase => gix_revision::Spec::Merge {
                    theirs: first.ok_or_else(malformed)?,
                    ours: second.ok_or_else(malformed)?,
                },
                IncludeReachableFromParents => gix_revision::Spec::IncludeOnlyParents(first.ok_or_else(malformed)?),
                ExcludeReachableFromParents => gix_revision::Spec::ExcludeParents(first.ok_or_else(malformed)?),
            })
        }

        let range = zero_or_one_objects_or_ambiguity_err(self.objs, self.prefix, self.repo)?;
        Ok(crate::revision::Spec {
            path: self.paths[0].take().or(self.paths[1].take()),
            first_ref: self.refs[0].take(),
            second_ref: self.refs[1].take(),
            inner: kind_to_spec(self.kind, range)?,
            repo: self.repo,
        })
    }
}

impl parse::Delegate for Delegate<'_> {
    fn done(&mut self) -> Result<(), Exn> {
        self.follow_refs_to_objects_if_needed_delay_errors()?;
        self.disambiguate_objects_by_fallback_hint_delay_errors(
            self.kind_implies_committish()
                .then_some(ObjectKindHint::Committish)
                .or(self.opts.object_kind_hint),
        );
        if self.delayed_errors.is_empty() {
            Ok(())
        } else {
            let err = Exn::from_iter(
                self.delayed_errors.drain(..),
                message!("Encountered unprocessed errors when finishing"),
            );
            Err(err.something())
        }
    }
}

impl delegate::Kind for Delegate<'_> {
    fn kind(&mut self, kind: gix_revision::spec::Kind) -> Result<(), Exn> {
        use gix_revision::spec::Kind::*;
        self.kind = Some(kind);

        if self.kind_implies_committish() {
            self.disambiguate_objects_by_fallback_hint_delay_errors(ObjectKindHint::Committish.into());
        }
        if matches!(kind, RangeBetween | ReachableToMergeBase) {
            self.idx += 1;
        }

        Ok(())
    }
}

impl Delegate<'_> {
    fn has_err(&self) -> bool {
        !self.delayed_errors.is_empty()
    }
    fn kind_implies_committish(&self) -> bool {
        self.kind.unwrap_or(gix_revision::spec::Kind::IncludeReachable) != gix_revision::spec::Kind::IncludeReachable
    }

    fn disambiguate_objects_by_fallback_hint_delay_errors(&mut self, hint: Option<ObjectKindHint>) {
        fn require_object_kind(repo: &Repository, obj: &gix_hash::oid, kind: gix_object::Kind) -> Result<(), Exn> {
            let obj = repo.find_object(obj).or_something()?;
            if obj.kind == kind {
                Ok(())
            } else {
                Err(message!(
                    "Object {oid} was a {actual}, but needed it to be a {expected}",
                    actual = obj.kind,
                    expected = kind,
                    oid = obj.id.attach(repo).shorten_or_id(),
                )
                .raise_something())
            }
        }

        if self.last_call_was_disambiguate_prefix[self.idx] {
            self.unset_disambiguate_call();

            if let Some(objs) = self.objs[self.idx].as_mut() {
                let repo = self.repo;
                let errors: Vec<_> = match hint {
                    Some(kind_hint) => match kind_hint {
                        ObjectKindHint::Treeish | ObjectKindHint::Committish => {
                            let kind = match kind_hint {
                                ObjectKindHint::Treeish => gix_object::Kind::Tree,
                                ObjectKindHint::Committish => gix_object::Kind::Commit,
                                _ => unreachable!("BUG: we narrow possibilities above"),
                            };
                            objs.iter()
                                .filter_map(|obj| peel(repo, obj, kind).err().map(|err| (*obj, err)))
                                .collect()
                        }
                        ObjectKindHint::Tree | ObjectKindHint::Commit | ObjectKindHint::Blob => {
                            let kind = match kind_hint {
                                ObjectKindHint::Tree => gix_object::Kind::Tree,
                                ObjectKindHint::Commit => gix_object::Kind::Commit,
                                ObjectKindHint::Blob => gix_object::Kind::Blob,
                                _ => unreachable!("BUG: we narrow possibilities above"),
                            };
                            objs.iter()
                                .filter_map(|obj| require_object_kind(repo, obj, kind).err().map(|err| (*obj, err)))
                                .collect()
                        }
                    },
                    None => return,
                };

                if errors.len() == objs.len() {
                    self.delayed_errors
                        .extend(errors.into_iter().map(|(_, err)| gix_error::Error::from(err).into()));
                } else {
                    for (obj, err) in errors {
                        objs.remove(&obj);
                        self.delayed_errors.push(gix_error::Error::from(err).into());
                    }
                }
            }
        }
    }

    fn follow_refs_to_objects_if_needed_delay_errors(&mut self) -> Result<(), Exn> {
        let repo = self.repo;
        for (r, obj) in self.refs.iter().zip(self.objs.iter_mut()) {
            if let (Some(ref_), obj_opt @ None) = (r, obj) {
                if let Some(id) = ref_.target.try_id().map(ToOwned::to_owned).or_else(|| {
                    match ref_.clone().attach(repo).peel_to_id() {
                        Err(err) => {
                            self.delayed_errors.push(
                                err.raise()
                                    .raise(message!(
                                        "Could not peel '{}' to obtain its target",
                                        ref_.name.as_bstr()
                                    ))
                                    .into_error()
                                    .into(),
                            );
                            None
                        }
                        Ok(id) => Some(id.detach()),
                    }
                }) {
                    obj_opt.get_or_insert_with(HashSet::default).insert(id);
                }
            }
        }
        Ok(())
    }

    fn unset_disambiguate_call(&mut self) {
        self.last_call_was_disambiguate_prefix[self.idx] = false;
    }
}

fn peel(repo: &Repository, obj: &gix_hash::oid, kind: gix_object::Kind) -> Result<ObjectId, Exn> {
    let mut obj = repo.find_object(obj).or_something()?;
    obj = obj.peel_to_kind(kind).or_something()?;
    debug_assert_eq!(obj.kind, kind, "bug in Object::peel_to_kind() which didn't deliver");
    Ok(obj.id)
}

fn handle_errors_and_replacements(
    destination: &mut Vec<Error>,
    objs: &mut HashSet<ObjectId>,
    errors: Vec<(ObjectId, Exn)>,
    replacements: &mut Replacements,
) -> Result<(), Exn> {
    if errors.len() == objs.len() {
        destination.extend(errors.into_iter().map(|(_, err)| gix_error::Error::from(err).into()));
        bail!(Something)
    } else {
        for (obj, err) in errors {
            objs.remove(&obj);
            destination.push(gix_error::Error::from(err).into());
        }
        for (find, replace) in replacements {
            objs.remove(find);
            objs.insert(*replace);
        }
        Ok(())
    }
}

mod navigate;
mod revision;
