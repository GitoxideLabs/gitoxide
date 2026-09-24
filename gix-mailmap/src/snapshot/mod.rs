use bstr::ByteSlice;
use gix_actor::SignatureRef;

use crate::Snapshot;

mod signature;
pub use signature::{ResolvedSignature, Signature};

mod util;
use util::cmp_ignore_ascii_case;

mod entry;
pub(crate) use entry::EmailEntry;

impl Snapshot {
    /// Create a new snapshot from the given bytes buffer, ignoring all parse errors that may occur on a line-by-line basis.
    ///
    /// This is similar to what git does.
    pub fn from_bytes(buf: &[u8]) -> Self {
        Self::new(crate::parse_ignore_errors(buf))
    }

    /// Create a new instance from `entries`, ignoring those with neither a new name nor a new email.
    ///
    /// These can be obtained using [`crate::parse()`].
    pub fn new<'a>(entries: impl IntoIterator<Item = crate::Entry<'a>>) -> Self {
        let mut snapshot = Self::default();
        snapshot.merge(entries);
        snapshot
    }

    /// Merge the given `entries` into this instance, possibly overwriting existing mappings with
    /// new ones should they collide.
    ///
    /// Email-only mappings replace only the fields they provide. Mappings matching both name and email
    /// replace the entire previous mapping for that pair.
    ///
    /// Entries with neither a new name nor a new email are ignored.
    ///
    /// Entries are sorted in bulk, so prefer passing a batch over merging one entry at a time.
    pub fn merge<'a>(&mut self, entries: impl IntoIterator<Item = crate::Entry<'a>>) -> &mut Self {
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|entry| entry.new_name.is_some() || entry.new_email.is_some())
            .map(EmailEntry::from)
            .collect();
        if entries.is_empty() {
            return self;
        }
        self.entries_by_old_email.extend(entries);
        // Stable sorting keeps existing mappings first and applies updates in input order.
        self.entries_by_old_email
            .sort_by(|a, b| cmp_ignore_ascii_case(a.old_email.as_bstr(), b.old_email.as_bstr()));
        self.entries_by_old_email.dedup_by(|later, earlier| {
            if cmp_ignore_ascii_case(earlier.old_email.as_bstr(), later.old_email.as_bstr()).is_eq() {
                earlier.merge(later);
                true
            } else {
                false
            }
        });
        for entry in &mut self.entries_by_old_email {
            entry
                .entries_by_old_name
                .sort_by(|a, b| cmp_ignore_ascii_case(a.old_name.as_bstr(), b.old_name.as_bstr()));
            entry.entries_by_old_name.dedup_by(|later, earlier| {
                if cmp_ignore_ascii_case(earlier.old_name.as_bstr(), later.old_name.as_bstr()).is_eq() {
                    earlier.new_name = later.new_name.take();
                    earlier.new_email = later.new_email.take();
                    true
                } else {
                    false
                }
            });
            entry.entries_by_old_name.shrink_to_fit();
        }
        self.entries_by_old_email.shrink_to_fit();
        self
    }

    /// Transform our acceleration structure into an iterator of entries.
    ///
    /// Note that the order is different from how they were obtained initially, and are explicitly ordered by
    /// (`old_email`, `old_name`).
    pub fn iter(&self) -> impl Iterator<Item = crate::Entry<'_>> {
        self.entries_by_old_email.iter().flat_map(|entry| {
            let initial = if entry.new_email.is_some() || entry.new_name.is_some() {
                Some(crate::Entry {
                    new_name: entry.new_name.as_ref().map(|b| b.as_bstr()),
                    new_email: entry.new_email.as_ref().map(|b| b.as_bstr()),
                    old_name: None,
                    old_email: entry.old_email.as_bstr(),
                })
            } else {
                None
            };

            let rest = entry.entries_by_old_name.iter().map(|name_entry| crate::Entry {
                new_name: name_entry.new_name.as_ref().map(|b| b.as_bstr()),
                new_email: name_entry.new_email.as_ref().map(|b| b.as_bstr()),
                old_name: name_entry.old_name.as_bstr().into(),
                old_email: entry.old_email.as_bstr(),
            });

            initial.into_iter().chain(rest)
        })
    }

    /// Transform our acceleration structure into a list of entries.
    ///
    /// Note that the order is different from how they were obtained initially, and are explicitly ordered by
    /// (`old_email`, `old_name`).
    pub fn entries(&self) -> Vec<crate::Entry<'_>> {
        self.iter().collect()
    }

    /// Try to resolve `signature` by its contained email and name and provide resolved/mapped names as reference.
    /// Return `None` if no such mapping was found.
    ///
    /// Note that opposed to what git seems to do, we also normalize the case of email addresses to match the one
    /// given in the mailmap. That is, if `Alex@example.com` is the current email, it will be matched and replaced with
    /// `alex@example.com`. This leads to better mapping results and saves entries in the mailmap.
    ///
    /// This is the fastest possible lookup as there is no allocation.
    pub fn try_resolve_ref(&self, signature: gix_actor::SignatureRef<'_>) -> Option<ResolvedSignature<'_>> {
        let pos = self
            .entries_by_old_email
            .binary_search_by(|e| cmp_ignore_ascii_case(e.old_email.as_bstr(), signature.email))
            .ok()?;
        let entry = &self.entries_by_old_email[pos];

        match entry
            .entries_by_old_name
            .binary_search_by(|e| cmp_ignore_ascii_case(e.old_name.as_bstr(), signature.name))
        {
            Ok(pos) => {
                let name_entry = &entry.entries_by_old_name[pos];
                ResolvedSignature::try_new(
                    name_entry.new_email.as_ref(),
                    entry.old_email.as_bstr(),
                    signature.email,
                    name_entry.new_name.as_ref(),
                )
            }
            Err(_) => ResolvedSignature::try_new(
                entry.new_email.as_ref(),
                entry.old_email.as_bstr(),
                signature.email,
                entry.new_name.as_ref(),
            ),
        }
    }

    /// Try to resolve `signature` by its contained email and name and provide resolved/mapped names as owned signature,
    /// with the mapped name and/or email replaced accordingly.
    ///
    /// Return `None` if no such mapping was found.
    pub fn try_resolve(&self, signature: gix_actor::SignatureRef<'_>) -> Option<gix_actor::Signature> {
        self.try_resolve_ref(signature)
            .map(|new| enriched_signature(signature, new).into())
    }

    /// Like [`try_resolve()`][Snapshot::try_resolve()], but always returns an owned signature, which might be a copy
    /// of `signature` if no mapping was found.
    ///
    /// Note that this method will always allocate.
    pub fn resolve(&self, signature: gix_actor::SignatureRef<'_>) -> gix_actor::Signature {
        self.try_resolve(signature).unwrap_or_else(|| gix_actor::Signature {
            name: signature.name.to_owned(),
            email: signature.email.to_owned(),
            time: signature.time().unwrap_or_default(),
        })
    }

    /// Like [`try_resolve()`][Snapshot::try_resolve()], but always returns a special copy-on-write signature, which contains
    /// changed names or emails as `Cow::Owned`, or `Cow::Borrowed` if no mapping was found.
    pub fn resolve_cow<'a>(&self, signature: gix_actor::SignatureRef<'a>) -> Signature<'a> {
        self.try_resolve_ref(signature)
            .map_or_else(|| signature.into(), |new| enriched_signature(signature, new))
    }
}

fn enriched_signature<'a>(
    SignatureRef { name, email, time }: SignatureRef<'a>,
    new: ResolvedSignature<'_>,
) -> Signature<'a> {
    match (new.email, new.name) {
        (Some(new_email), Some(new_name)) => Signature {
            email: new_email.to_owned().into(),
            name: new_name.to_owned().into(),
            time: time.parse().unwrap_or_default(),
        },
        (Some(new_email), None) => Signature {
            email: new_email.to_owned().into(),
            name: name.into(),
            time: time.parse().unwrap_or_default(),
        },
        (None, Some(new_name)) => Signature {
            email: email.into(),
            name: new_name.to_owned().into(),
            time: time.parse().unwrap_or_default(),
        },
        (None, None) => unreachable!("BUG: ResolvedSignatures don't exist here when nothing is set"),
    }
}
