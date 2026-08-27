use std::collections::BTreeMap;

use gix_object::bstr::{BStr, BString, ByteSlice};
use gix_path::RelativePath;

use crate::{FullName, PartialNameRef, Reference, name::is_pseudo_ref};

use super::{Error, Route, Snapshot, StackLocation, WorktreeContext, worktree_privacy};

impl Snapshot<'_> {
    pub(crate) fn try_find(&self, partial_name: &PartialNameRef) -> Result<Option<Reference>, Error> {
        let mut buf = BString::default();
        for consider_pseudo_ref in [true, false] {
            if !consider_pseudo_ref && !is_pseudo_ref(partial_name.as_bstr()) {
                break;
            }
            'candidates: for inbetween in ["", "tags", "heads", "remotes"] {
                let full_name = partial_name.construct_full_name_ref(inbetween, &mut buf, consider_pseudo_ref);
                if let Some(reference) = self.find_full(full_name)? {
                    return Ok(Some(reference));
                }
                if consider_pseudo_ref && is_pseudo_ref(partial_name.as_bstr()) {
                    break 'candidates;
                }
            }
        }
        if partial_name.as_bstr() == b"HEAD" {
            return Ok(None);
        }
        let remote_head = partial_name
            .to_owned()
            .join(b"HEAD".as_bstr())
            .expect("adding HEAD to a validated partial name remains valid");
        let full_name = remote_head.as_ref().construct_full_name_ref("remotes", &mut buf, true);
        self.find_full(full_name)
    }

    pub(crate) fn find_full(&self, name: &crate::FullNameRef) -> Result<Option<Reference>, Error> {
        let route = self.store.route(name);
        self.find_route(&route)
    }

    pub(crate) fn find_route(&self, route: &Route) -> Result<Option<Reference>, Error> {
        let convert = |snapshot: &gix_reftable::Snapshot| {
            snapshot
                .find_ref(route.stored_name.as_bstr())
                .map(|record| {
                    self.store
                        .reference_from_record(record, route.public_name.clone(), &route.context)
                })
                .transpose()
        };
        self.with_snapshot(route, convert)?
    }

    pub(crate) fn all(&self) -> Vec<Result<Reference, Error>> {
        self.collect_references(None, false)
    }

    pub(crate) fn prefixed(&self, prefix: &RelativePath) -> Vec<Result<Reference, Error>> {
        self.collect_references(Some(prefix.as_ref().as_bstr()), false)
    }

    pub(crate) fn pseudo(&self) -> Vec<Result<Reference, Error>> {
        self.collect_references(None, true)
    }

    fn collect_references(&self, prefix: Option<&BStr>, pseudo: bool) -> Vec<Result<Reference, Error>> {
        let mut merged = BTreeMap::<BString, (gix_reftable::RefRecord, WorktreeContext)>::new();
        self.insert_visible(&mut merged, &self.main, false);
        if let Some(current) = &self.current {
            self.insert_visible(&mut merged, current, true);
        }
        merged
            .into_iter()
            .filter(|(name, _)| {
                let is_pseudo = is_pseudo_ref(name.as_ref());
                if pseudo {
                    is_pseudo
                } else {
                    !is_pseudo && name.starts_with_str("refs/") && prefix.is_none_or(|prefix| name.starts_with(prefix))
                }
            })
            .map(|(name, (record, context))| {
                let public_name = FullName::try_from(name.clone()).map_err(|source| Error::InvalidName {
                    name: name.clone(),
                    source,
                })?;
                self.store.reference_from_record(&record, public_name, &context)
            })
            .collect()
    }

    fn insert_visible(
        &self,
        out: &mut BTreeMap<BString, (gix_reftable::RefRecord, WorktreeContext)>,
        snapshot: &gix_reftable::Snapshot,
        is_current: bool,
    ) {
        for record in snapshot.refs() {
            let Some(mut name) = self.visible_name(&record.name) else {
                continue;
            };
            let privacy = worktree_privacy(name.as_ref());
            if (is_current && privacy == Some(false))
                || (!is_current && self.current.is_some() && privacy == Some(true))
            {
                continue;
            }
            let context = WorktreeContext::Current;
            out.insert(std::mem::take(&mut name), (record.clone(), context));
        }
    }

    fn visible_name(&self, stored: &BString) -> Option<BString> {
        match &self.store.namespace {
            Some(namespace) => stored
                .strip_prefix(namespace.0.as_bytes())
                .map(|name| name.as_bstr().to_owned()),
            None => Some(stored.clone()),
        }
    }

    pub(crate) fn reflog_exists(&self, route: &Route) -> Result<bool, Error> {
        self.with_snapshot(route, |snapshot| snapshot.reflog_exists(route.stored_name.as_bstr()))
    }

    pub(crate) fn reflog_lines(&self, route: &Route) -> Result<Vec<Result<crate::log::Line, Error>>, Error> {
        self.with_snapshot(route, |snapshot| {
            snapshot
                .logs_for(route.stored_name.as_bstr())
                .into_iter()
                .map(log_line)
                .collect()
        })
    }

    fn with_snapshot<T>(&self, route: &Route, f: impl FnOnce(&gix_reftable::Snapshot) -> T) -> Result<T, Error> {
        match &route.location {
            StackLocation::Main => Ok(f(&self.main)),
            StackLocation::Current => Ok(f(self.current.as_ref().unwrap_or(&self.main))),
            StackLocation::Other(_) => {
                let path = self.store.stack_path(&route.location)?;
                let mut other = gix_features::threading::lock(&self.other);
                if !other.contains_key(&path) {
                    let snapshot = self.store.stack(&route.location)?.snapshot()?;
                    other.insert(path.clone(), snapshot);
                }
                Ok(f(other.get(&path).expect("the snapshot was inserted above")))
            }
        }
    }
}

fn log_line(record: &gix_reftable::LogRecord) -> Result<crate::log::Line, Error> {
    let gix_reftable::LogValue::Update {
        old_id,
        new_id,
        name,
        email,
        time,
        tz_offset,
        message,
    } = &record.value
    else {
        unreachable!("stack snapshots expose only live reflog entries")
    };
    let seconds = i64::try_from(*time).map_err(|_| Error::InvalidLogTime)?;
    let offset = i32::from(*tz_offset).checked_mul(60).ok_or(Error::InvalidLogTime)?;
    Ok(crate::log::Line {
        previous_oid: *old_id,
        new_oid: *new_id,
        signature: gix_actor::Signature {
            name: name.clone(),
            email: email.clone(),
            time: gix_actor::date::Time { seconds, offset },
        },
        message: message.clone(),
    })
}
