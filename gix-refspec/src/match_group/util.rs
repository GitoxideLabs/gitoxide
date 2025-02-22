use std::{borrow::Cow, ops::Range};

use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_hash::ObjectId;

use crate::{match_group::Item, RefSpecRef};

/// A type keeping enough information about a ref-spec to be able to efficiently match it against multiple matcher items.
pub struct Matcher<'a> {
    pub(crate) lhs: Option<Needle<'a>>,
    pub(crate) rhs: Option<Needle<'a>>,
}

impl<'a> Matcher<'a> {
    /// Match the lefthand-side `item` against this spec and return `(true, Some<rhs>)` to gain the other,
    /// transformed righthand-side of the match as configured by the refspec.
    /// Or return `(true, None)` if there was no `rhs` but the `item` matched.
    /// Lastly, return `(false, None)` if `item` didn't match at all.
    ///
    /// This may involve resolving a glob with an allocation, as the destination is built using the matching portion of a glob.
    pub fn matches_lhs(&self, item: Item<'_>) -> (bool, Option<Cow<'a, BStr>>) {
        match (self.lhs, self.rhs) {
            (Some(lhs), None) => (lhs.matches(item).is_match(), None),
            (Some(lhs), Some(rhs)) => lhs.matches(item).into_match_outcome(rhs, item),
            (None, _) => (false, None),
        }
    }

    /// Match the righthand-side `item` against this spec and return `(true, Some<lhs>)` to gain the other,
    /// transformed lefthand-side of the match as configured by the refspec.
    /// Or return `(true, None)` if there was no `lhs` but the `item` matched.
    /// Lastly, return `(false, None)` if `item` didn't match at all.
    ///
    /// This may involve resolving a glob with an allocation, as the destination is built using the matching portion of a glob.
    pub fn matches_rhs(&self, item: Item<'_>) -> (bool, Option<Cow<'a, BStr>>) {
        match (self.lhs, self.rhs) {
            (None, Some(rhs)) => (rhs.matches(item).is_match(), None),
            (Some(lhs), Some(rhs)) => rhs.matches(item).into_match_outcome(lhs, item),
            (_, None) => (false, None),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum Needle<'a> {
    FullName(&'a BStr),
    PartialName(&'a BStr),
    Glob { name: &'a BStr, asterisk_pos: usize },
    Object(ObjectId),
}

enum Match {
    /// There was no match.
    None,
    /// No additional data is provided as part of the match.
    Normal,
    /// The range of text to copy from the originating item name
    GlobRange(Range<usize>),
}

impl Match {
    fn is_match(&self) -> bool {
        !matches!(self, Match::None)
    }
    fn into_match_outcome<'a>(self, destination: Needle<'a>, item: Item<'_>) -> (bool, Option<Cow<'a, BStr>>) {
        let arg = match self {
            Match::None => return (false, None),
            Match::Normal => None,
            Match::GlobRange(range) => Some((range, item)),
        };
        (true, destination.to_bstr_replace(arg).into())
    }
}

impl<'a> Needle<'a> {
    #[inline]
    fn matches(&self, item: Item<'_>) -> Match {
        match self {
            Needle::FullName(name) => {
                if *name == item.full_ref_name {
                    Match::Normal
                } else {
                    Match::None
                }
            }
            Needle::PartialName(name) => crate::spec::expand_partial_name(name, |expanded| {
                (expanded == item.full_ref_name).then_some(Match::Normal)
            })
            .unwrap_or(Match::None),
            Needle::Glob { name, asterisk_pos } => {
                match item.full_ref_name.get(..*asterisk_pos) {
                    Some(full_name_portion) if full_name_portion != name[..*asterisk_pos] => {
                        return Match::None;
                    }
                    None => return Match::None,
                    _ => {}
                }
                let tail = &name[*asterisk_pos + 1..];
                if !item.full_ref_name.ends_with(tail) {
                    return Match::None;
                }
                let end = item.full_ref_name.len() - tail.len();
                Match::GlobRange(*asterisk_pos..end)
            }
            Needle::Object(id) => {
                if *id == item.target {
                    return Match::Normal;
                }
                match item.object {
                    Some(object) if object == *id => Match::Normal,
                    _ => Match::None,
                }
            }
        }
    }

    fn to_bstr_replace(self, range: Option<(Range<usize>, Item<'_>)>) -> Cow<'a, BStr> {
        match (self, range) {
            (Needle::FullName(name), None) => Cow::Borrowed(name),
            (Needle::PartialName(name), None) => Cow::Owned({
                let mut base: BString = "refs/".into();
                if !(name.starts_with(b"tags/") || name.starts_with(b"remotes/")) {
                    base.push_str("heads/");
                }
                base.push_str(name);
                base
            }),
            (Needle::Glob { name, asterisk_pos }, Some((range, item))) => {
                let mut buf = Vec::with_capacity(name.len() + range.len() - 1);
                buf.push_str(&name[..asterisk_pos]);
                buf.push_str(&item.full_ref_name[range]);
                buf.push_str(&name[asterisk_pos + 1..]);
                Cow::Owned(buf.into())
            }
            (Needle::Object(id), None) => {
                let mut name = id.to_string();
                name.insert_str(0, "refs/heads/");
                Cow::Owned(name.into())
            }
            (Needle::Glob { .. }, None) => unreachable!("BUG: no range provided for glob pattern"),
            (_, Some(_)) => {
                unreachable!("BUG: range provided even though needle wasn't a glob. Globs are symmetric.")
            }
        }
    }

    pub fn to_bstr(self) -> Cow<'a, BStr> {
        self.to_bstr_replace(None)
    }
}

impl<'a> From<&'a BStr> for Needle<'a> {
    fn from(v: &'a BStr) -> Self {
        if let Some(pos) = v.find_byte(b'*') {
            Needle::Glob {
                name: v,
                asterisk_pos: pos,
            }
        } else if v.starts_with(b"refs/") {
            Needle::FullName(v)
        } else if let Ok(id) = gix_hash::ObjectId::from_hex(v) {
            Needle::Object(id)
        } else {
            Needle::PartialName(v)
        }
    }
}

impl<'a> From<RefSpecRef<'a>> for Matcher<'a> {
    fn from(v: RefSpecRef<'a>) -> Self {
        Matcher {
            lhs: v.src.map(Into::into),
            rhs: v.dst.map(Into::into),
        }
    }
}
