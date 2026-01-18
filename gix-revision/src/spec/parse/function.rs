use std::{str::FromStr, time::SystemTime};

use crate::{
    spec,
    spec::parse::{delegate, delegate::SiblingBranch, Delegate, Error},
};
use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_error::{ErrorExt, Exn, ResultExt};

/// Parse a git [`revspec`](https://git-scm.com/docs/git-rev-parse#_specifying_revisions) and call `delegate` for each token
/// successfully parsed.
///
/// Note that the `delegate` is expected to maintain enough state to lookup revisions properly.
/// Returns `Ok(())` if all of `input` was consumed, or the error if either the `revspec` syntax was incorrect or
/// the `delegate` failed to perform the request.
pub fn parse(mut input: &BStr, delegate: &mut impl Delegate) -> Result<(), Exn<Error>> {
    use delegate::{Kind, Revision};
    let mut delegate = InterceptRev::new(delegate);
    let mut prev_kind = None;
    if let Some(b'^') = input.first() {
        input = next(input).1;
        let kind = spec::Kind::ExcludeReachable;
        delegate
            .kind(kind)
            .or_raise(|| Error::new(format!("delegate.kind({kind:?}) failed")))?;
        prev_kind = kind.into();
    }

    let mut found_revision;
    (input, found_revision) = {
        let rest = revision(input, &mut delegate)?;
        (rest, rest != input)
    };
    if delegate.done {
        return if input.is_empty() {
            Ok(())
        } else {
            Err(Error::new_with_input("unconsumed input", input).raise())
        };
    }
    if let Some((rest, kind)) = try_range(input) {
        if let Some(prev_kind) = prev_kind {
            return Err(Error::new(format!(
                "cannot set spec kind more than once (was {prev_kind:?}, now {kind:?})"
            ))
            .raise());
        }
        if !found_revision {
            delegate
                .find_ref("HEAD".into())
                .or_raise(|| Error::new("delegate did not find the HEAD reference"))?;
        }
        delegate
            .kind(kind)
            .or_raise(|| Error::new(format!("delegate.kind({kind:?}) failed")))?;
        (input, found_revision) = {
            let remainder = revision(rest.as_bstr(), &mut delegate)?;
            (remainder, remainder != rest)
        };
        if !found_revision {
            delegate
                .find_ref("HEAD".into())
                .or_raise(|| Error::new("delegate did not find the HEAD reference"))?;
        }
    }

    if input.is_empty() {
        delegate
            .done()
            .or_raise(|| Error::new("No revision was produced after all input was consumed"))
    } else {
        Err(Error::new_with_input("unconsumed input", input).raise())
    }
}

mod intercept {
    use crate::spec::parse::{delegate, Delegate};
    use bstr::{BStr, BString};
    use gix_error::Exn;

    #[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
    pub(crate) enum PrefixHintOwned {
        MustBeCommit,
        DescribeAnchor { ref_name: BString, generation: usize },
    }

    impl PrefixHintOwned {
        pub fn to_ref(&self) -> delegate::PrefixHint<'_> {
            match self {
                PrefixHintOwned::MustBeCommit => delegate::PrefixHint::MustBeCommit,
                PrefixHintOwned::DescribeAnchor { ref_name, generation } => delegate::PrefixHint::DescribeAnchor {
                    ref_name: ref_name.as_ref(),
                    generation: *generation,
                },
            }
        }
    }

    impl<'a> From<delegate::PrefixHint<'a>> for PrefixHintOwned {
        fn from(v: delegate::PrefixHint<'a>) -> Self {
            match v {
                delegate::PrefixHint::MustBeCommit => PrefixHintOwned::MustBeCommit,
                delegate::PrefixHint::DescribeAnchor { generation, ref_name } => PrefixHintOwned::DescribeAnchor {
                    ref_name: ref_name.to_owned(),
                    generation,
                },
            }
        }
    }

    pub(crate) struct InterceptRev<'a, T> {
        pub inner: &'a mut T,
        pub last_ref: Option<BString>, // TODO: smallvec to save the unnecessary allocation? Can't keep ref due to lifetime constraints in traits
        pub last_prefix: Option<(gix_hash::Prefix, Option<PrefixHintOwned>)>,
        pub done: bool,
    }

    impl<'a, T> InterceptRev<'a, T>
    where
        T: Delegate,
    {
        pub fn new(delegate: &'a mut T) -> Self {
            InterceptRev {
                inner: delegate,
                last_ref: None,
                last_prefix: None,
                done: false,
            }
        }
    }

    impl<T> Delegate for InterceptRev<'_, T>
    where
        T: Delegate,
    {
        fn done(&mut self) -> Result<(), Exn> {
            self.done = true;
            self.inner.done()
        }
    }

    impl<T> delegate::Revision for InterceptRev<'_, T>
    where
        T: Delegate,
    {
        fn find_ref(&mut self, name: &BStr) -> Result<(), Exn> {
            self.last_ref = name.to_owned().into();
            self.inner.find_ref(name)
        }

        fn disambiguate_prefix(
            &mut self,
            prefix: gix_hash::Prefix,
            hint: Option<delegate::PrefixHint<'_>>,
        ) -> Result<(), Exn> {
            self.last_prefix = Some((prefix, hint.map(Into::into)));
            self.inner.disambiguate_prefix(prefix, hint)
        }

        fn reflog(&mut self, query: delegate::ReflogLookup) -> Result<(), Exn> {
            self.inner.reflog(query)
        }

        fn nth_checked_out_branch(&mut self, branch_no: usize) -> Result<(), Exn> {
            self.inner.nth_checked_out_branch(branch_no)
        }

        fn sibling_branch(&mut self, kind: delegate::SiblingBranch) -> Result<(), Exn> {
            self.inner.sibling_branch(kind)
        }
    }

    impl<T> delegate::Navigate for InterceptRev<'_, T>
    where
        T: Delegate,
    {
        fn traverse(&mut self, kind: delegate::Traversal) -> Result<(), Exn> {
            self.inner.traverse(kind)
        }

        fn peel_until(&mut self, kind: delegate::PeelTo<'_>) -> Result<(), Exn> {
            self.inner.peel_until(kind)
        }

        fn find(&mut self, regex: &BStr, negated: bool) -> Result<(), Exn> {
            self.inner.find(regex, negated)
        }

        fn index_lookup(&mut self, path: &BStr, stage: u8) -> Result<(), Exn> {
            self.inner.index_lookup(path, stage)
        }
    }

    impl<T> delegate::Kind for InterceptRev<'_, T>
    where
        T: Delegate,
    {
        fn kind(&mut self, kind: crate::spec::Kind) -> Result<(), Exn> {
            self.inner.kind(kind)
        }
    }
}
use intercept::InterceptRev;

trait ResultExt2 {
    fn or_else_none<F>(self, f: F) -> Option<()>
    where
        F: FnOnce(Exn);
}

impl ResultExt2 for Result<(), Exn> {
    fn or_else_none<F>(self, f: F) -> Option<()>
    where
        F: FnOnce(Exn),
    {
        match self {
            Ok(()) => Some(()),
            Err(err) => {
                f(err);
                None
            }
        }
    }
}

fn try_set_prefix(
    delegate: &mut impl Delegate,
    hex_name: &BStr,
    hint: Option<delegate::PrefixHint<'_>>,
    errors: &mut Vec<Exn>,
) -> Option<()> {
    gix_hash::Prefix::from_hex(hex_name.to_str().expect("hexadecimal only"))
        .ok()
        .and_then(|prefix| {
            delegate
                .disambiguate_prefix(prefix, hint)
                .or_else_none(|err| errors.push(err))
        })
}

fn long_describe_prefix(name: &BStr) -> Option<(&BStr, delegate::PrefixHint<'_>)> {
    let mut iter = name.rsplit(|b| *b == b'-');
    let candidate = iter.by_ref().find_map(|substr| {
        if substr.first()? != &b'g' {
            return None;
        }
        let rest = substr.get(1..)?;
        rest.iter().all(u8::is_ascii_hexdigit).then(|| rest.as_bstr())
    })?;

    let candidate = iter.clone().any(|token| !token.is_empty()).then_some(candidate);
    let hint = iter
        .next()
        .and_then(|gen| gen.to_str().ok().and_then(|gen| usize::from_str(gen).ok()))
        .and_then(|generation| {
            iter.next().map(|token| {
                let last_token_len = token.len();
                let first_token_ptr = iter.next_back().map_or(token.as_ptr(), <[_]>::as_ptr);
                // SAFETY: both pointers are definitely part of the same object
                #[allow(unsafe_code)]
                let prior_tokens_len: usize = unsafe { token.as_ptr().offset_from(first_token_ptr) }
                    .try_into()
                    .expect("positive value");
                delegate::PrefixHint::DescribeAnchor {
                    ref_name: name[..prior_tokens_len + last_token_len].as_bstr(),
                    generation,
                }
            })
        })
        .unwrap_or(delegate::PrefixHint::MustBeCommit);

    candidate.map(|c| (c, hint))
}

fn short_describe_prefix(name: &BStr) -> Option<&BStr> {
    let mut iter = name.split(|b| *b == b'-');
    let candidate = iter
        .next()
        .and_then(|prefix| prefix.iter().all(u8::is_ascii_hexdigit).then(|| prefix.as_bstr()));
    (iter.count() == 1).then_some(candidate).flatten()
}

type InsideParensRestConsumed<'a> = (std::borrow::Cow<'a, BStr>, &'a BStr, usize);
fn parens(input: &[u8]) -> Result<Option<InsideParensRestConsumed<'_>>, Error> {
    if input.first() != Some(&b'{') {
        return Ok(None);
    }
    let mut open_braces = 0;
    let mut ignore_next = false;
    let mut skip_list = Vec::new();
    for (idx, b) in input.iter().enumerate() {
        match *b {
            b'{' => {
                if ignore_next {
                    ignore_next = false;
                } else {
                    open_braces += 1;
                }
            }
            b'}' => {
                if ignore_next {
                    ignore_next = false;
                } else {
                    open_braces -= 1;
                }
            }
            b'\\' => {
                skip_list.push(idx);
                if ignore_next {
                    skip_list.pop();
                    ignore_next = false;
                } else {
                    ignore_next = true;
                }
            }
            _ => {
                if ignore_next {
                    skip_list.pop();
                }
                ignore_next = false;
            }
        }
        if open_braces == 0 {
            let inner: std::borrow::Cow<'_, _> = if skip_list.is_empty() {
                input[1..idx].as_bstr().into()
            } else {
                let mut from = 1;
                let mut buf = BString::default();
                for next in skip_list.into_iter() {
                    buf.push_str(&input[from..next]);
                    from = next + 1;
                }
                if let Some(rest) = input.get(from..idx) {
                    buf.push_str(rest);
                }
                buf.into()
            };
            return Ok(Some((inner, input[idx + 1..].as_bstr(), idx + 1)));
        }
    }
    Err(Error::new_with_input("unclosed brace pair", input))
}

fn try_parse<T: FromStr + PartialEq + Default>(input: &BStr) -> Result<Option<T>, Error> {
    input
        .to_str()
        .ok()
        .and_then(|n| {
            n.parse().ok().map(|n| {
                if n == T::default() && input[0] == b'-' {
                    return Err(Error::new_with_input(
                        "negative zero is invalid - remove the minus sign",
                        input,
                    ));
                }
                Ok(n)
            })
        })
        .transpose()
}

fn revision<'a, T>(mut input: &'a BStr, delegate: &mut InterceptRev<'_, T>) -> Result<&'a BStr, Exn<Error>>
where
    T: Delegate,
{
    use delegate::{Navigate, Revision};
    fn consume_all(res: Result<(), Exn>, err: impl FnOnce() -> String) -> Result<&'static BStr, Exn<Error>> {
        res.map(|_| "".into()).or_raise(|| Error::new(err()))
    }
    match input.as_bytes() {
        [b':'] => {
            return Err(
                Error::new("':' must be followed by either slash and regex or path to lookup in HEAD tree").raise(),
            )
        }
        [b':', b'/'] => return Err(Error::new("':/' must be followed by a regular expression").raise()),
        [b':', b'/', regex @ ..] => {
            let (regex, negated) = parse_regex_prefix(regex.as_bstr())?;
            if regex.is_empty() {
                return Err(Error::new_with_input("unconsumed input", input).raise());
            }
            return consume_all(delegate.find(regex, negated), || {
                format!("Delegate couldn't find '{regex}' (negated: {negated})")
            });
        }
        [b':', b'0', b':', path @ ..] => {
            return consume_all(delegate.index_lookup(path.as_bstr(), 0), || {
                format!("Couldn't find index '{path}' stage 0", path = path.as_bstr())
            })
        }
        [b':', b'1', b':', path @ ..] => {
            return consume_all(delegate.index_lookup(path.as_bstr(), 1), || {
                format!("Couldn't find index '{path}' stage 1", path = path.as_bstr())
            })
        }
        [b':', b'2', b':', path @ ..] => {
            return consume_all(delegate.index_lookup(path.as_bstr(), 2), || {
                format!("Couldn't find index '{path}' stage 2", path = path.as_bstr())
            })
        }
        [b':', path @ ..] => {
            return consume_all(delegate.index_lookup(path.as_bstr(), 0), || {
                format!("Couldn't find index '{path}' stage 0 (implicit)", path = path.as_bstr())
            })
        }
        _ => {}
    }

    let mut sep_pos = None;
    let mut consecutive_hex_chars = Some(0);
    {
        let mut cursor = input;
        let mut ofs = 0;
        const SEPARATORS: &[u8] = b"~^:.";
        while let Some((pos, b)) = cursor.iter().enumerate().find(|(pos, b)| {
            if **b == b'@' {
                if cursor.len() == 1 {
                    return true;
                }
                let next = cursor.get(pos + 1);
                let next_next = cursor.get(pos + 2);
                if *pos != 0 && (next, next_next) == (Some(&b'.'), Some(&b'.')) {
                    return false;
                }
                next == Some(&b'{') || next.is_some_and(|b| SEPARATORS.contains(b))
            } else if SEPARATORS.contains(b) {
                true
            } else {
                if let Some(num) = consecutive_hex_chars.as_mut() {
                    if b.is_ascii_hexdigit() {
                        *num += 1;
                    } else {
                        consecutive_hex_chars = None;
                    }
                }
                false
            }
        }) {
            if *b != b'.' || cursor.get(pos + 1) == Some(&b'.') {
                sep_pos = Some(ofs + pos);
                break;
            }
            ofs += pos + 1;
            cursor = &cursor[pos + 1..];
        }
    }

    let name = &input[..sep_pos.unwrap_or(input.len())].as_bstr();
    let mut sep = sep_pos.map(|pos| input[pos]);
    let mut has_ref_or_implied_name = name.is_empty();
    if name.is_empty() && sep == Some(b'@') && sep_pos.and_then(|pos| input.get(pos + 1)) != Some(&b'{') {
        delegate
            .find_ref("HEAD".into())
            .or_raise(|| Error::new("delegate did not find the HEAD reference"))?;
        sep_pos = sep_pos.map(|pos| pos + 1);
        sep = match sep_pos.and_then(|pos| input.get(pos).copied()) {
            None => return Ok("".into()),
            Some(pos) => Some(pos),
        };
    } else {
        let mut errors = Vec::new();
        (consecutive_hex_chars.unwrap_or(0) >= gix_hash::Prefix::MIN_HEX_LEN)
            .then(|| try_set_prefix(delegate, name, None, &mut errors))
            .flatten()
            .or_else(|| {
                let (prefix, hint) = long_describe_prefix(name)
                    .map(|(c, h)| (c, Some(h)))
                    .or_else(|| short_describe_prefix(name).map(|c| (c, None)))?;
                try_set_prefix(delegate, prefix, hint, &mut errors)
            })
            .or_else(|| {
                name.is_empty().then_some(()).or_else(|| {
                    #[allow(clippy::let_unit_value)]
                    {
                        let res = delegate.find_ref(name).or_else_none(|err| {
                            errors.push(err);
                        })?;
                        has_ref_or_implied_name = true;
                        res.into()
                    }
                })
            })
            .ok_or_else(|| Error::new_with_input("couldn't parse revision", input).raise_all(errors))?;
    }

    input = {
        if let Some(b'@') = sep {
            let past_sep = input[sep_pos.map_or(input.len(), |pos| pos + 1)..].as_bstr();
            let (nav, rest, _consumed) = parens(past_sep)?.ok_or_else(|| {
                Error::new_with_input(
                    "@ character must be standalone or followed by {<content>}",
                    &input[sep_pos.unwrap_or(input.len())..],
                )
            })?;
            let nav = nav.as_ref();
            if let Some(n) = try_parse::<isize>(nav)? {
                if n < 0 {
                    if name.is_empty() {
                        delegate.nth_checked_out_branch(n.unsigned_abs()).or_raise(|| {
                            Error::new_with_input(
                                format!("delegate.nth_checked_out_branch({n:?}) didn't find a branch"),
                                nav,
                            )
                        })?;
                    } else {
                        return Err(Error::new_with_input(
                            "reference name must be followed by positive numbers in @{n}",
                            nav,
                        )
                        .raise());
                    }
                } else if has_ref_or_implied_name {
                    let lookup = if n >= 100000000 {
                        let time = nav
                            .to_str()
                            .or_raise(|| Error::new_with_input("could not parse time for reflog lookup", nav))
                            .and_then(|date| {
                                gix_date::parse(date, None)
                                    .or_raise(|| Error::new_with_input("could not parse time for reflog lookup", nav))
                            })?;
                        delegate::ReflogLookup::Date(time)
                    } else {
                        delegate::ReflogLookup::Entry(n.try_into().expect("non-negative isize fits usize"))
                    };
                    delegate
                        .reflog(lookup)
                        .or_raise(|| Error::new_with_input(format!("delegate.reflog({lookup:?}) failed"), nav))?;
                } else {
                    return Err(Error::new_with_input("reflog entries require a ref name", *name).raise());
                }
            } else if let Some(kind) = SiblingBranch::parse(nav) {
                if has_ref_or_implied_name {
                    delegate
                        .sibling_branch(kind)
                        .or_raise(|| Error::new_with_input(format!("delegate.sibling_branch({kind:?}) failed"), nav))
                } else {
                    Err(Error::new_with_input(
                        "sibling branches like 'upstream' or 'push' require a branch name with remote configuration",
                        *name,
                    )
                    .raise())
                }?;
            } else if has_ref_or_implied_name {
                let time = nav
                    .to_str()
                    .map_err(|_| Error::new_with_input("could not parse time for reflog lookup", nav))
                    .and_then(|date| {
                        gix_date::parse(date, Some(SystemTime::now()))
                            .map_err(|_| Error::new_with_input("could not parse time for reflog lookup", nav))
                    })?;
                let lookup = delegate::ReflogLookup::Date(time);
                delegate
                    .reflog(lookup)
                    .or_raise(|| Error::new_with_input(format!("delegate.reflog({lookup:?}) failed"), nav))?;
            } else {
                return Err(Error::new_with_input("reflog entries require a ref name", *name).raise());
            }
            rest
        } else {
            if sep_pos == Some(0) && sep == Some(b'~') {
                return Err(Error::new("tilde needs to follow an anchor, like @~").raise());
            }
            input[sep_pos.unwrap_or(input.len())..].as_bstr()
        }
    };

    navigate(input, delegate)
}

fn navigate<'a, T>(input: &'a BStr, delegate: &mut InterceptRev<'_, T>) -> Result<&'a BStr, Exn<Error>>
where
    T: Delegate,
{
    use delegate::{Kind, Navigate, Revision};
    let mut cursor = 0;
    let done_msg = "navigation succeeded, but no revision was produced as intermediate step";
    while let Some(b) = input.get(cursor) {
        cursor += 1;
        match *b {
            b'~' => {
                let (number, consumed) = input
                    .get(cursor..)
                    .and_then(|past_sep| try_parse_usize(past_sep.as_bstr()).transpose())
                    .transpose()?
                    .unwrap_or((1, 0));
                if number != 0 {
                    let traversal = delegate::Traversal::NthAncestor(number);
                    delegate.traverse(traversal).or_raise(|| {
                        Error::new_with_input(format!("delegate.traverse({traversal:?}) failed"), input)
                    })?;
                }
                cursor += consumed;
            }
            b'^' => {
                let past_sep = input.get(cursor..);
                if let Some((number, negative, consumed)) = past_sep
                    .and_then(|past_sep| try_parse_isize(past_sep.as_bstr()).transpose())
                    .transpose()?
                {
                    if negative {
                        let traversal = delegate::Traversal::NthParent(
                            number
                                .checked_mul(-1)
                                .ok_or_else(|| {
                                    Error::new_with_input("could not parse number", past_sep.expect("present"))
                                })?
                                .try_into()
                                .expect("non-negative"),
                        );
                        delegate.traverse(traversal).or_raise(|| {
                            Error::new_with_input(
                                "delegate.traverse({traversal:?}) failed",
                                past_sep.unwrap_or_default(),
                            )
                        })?;
                        let kind = spec::Kind::RangeBetween;
                        delegate.kind(kind).or_raise(|| {
                            Error::new_with_input(
                                format!("delegate.kind({kind:?}) failed"),
                                past_sep.unwrap_or_default(),
                            )
                        })?;
                        if let Some((prefix, hint)) = delegate.last_prefix.take() {
                            match &hint {
                                Some(hint) => delegate.disambiguate_prefix(prefix, hint.to_ref().into()),
                                None => delegate.disambiguate_prefix(prefix, None),
                            }
                            .or_raise(|| {
                                Error::new_with_input(
                                    format!("delegate.disambiguate_prefix({hint:?}) failed"),
                                    past_sep.unwrap_or_default(),
                                )
                            })?;
                        } else if let Some(name) = delegate.last_ref.take() {
                            delegate.find_ref(name.as_bstr()).or_raise(|| {
                                Error::new_with_input(
                                    format!("delegate.find_ref({name}) failed"),
                                    past_sep.unwrap_or_default(),
                                )
                            })?;
                        } else {
                            return Err(Error::new_with_input("unconsumed input", &input[cursor..]).raise());
                        }
                        cursor += consumed;
                        let rest = input[cursor..].as_bstr();
                        delegate.done().or_raise(|| Error::new_with_input(done_msg, rest))?;
                        return Ok(rest);
                    } else if number == 0 {
                        delegate.peel_until(delegate::PeelTo::ObjectKind(gix_object::Kind::Commit))
                    } else {
                        delegate.traverse(delegate::Traversal::NthParent(
                            number.try_into().expect("positive number"),
                        ))
                    }
                    .or_raise(|| Error::new_with_input("unknown navigation", past_sep.unwrap_or_default()))?;
                    cursor += consumed;
                } else if let Some((kind, _rest, consumed)) =
                    past_sep.and_then(|past_sep| parens(past_sep).transpose()).transpose()?
                {
                    cursor += consumed;
                    let target = match kind.as_ref().as_bytes() {
                        b"commit" => delegate::PeelTo::ObjectKind(gix_object::Kind::Commit),
                        b"tag" => delegate::PeelTo::ObjectKind(gix_object::Kind::Tag),
                        b"tree" => delegate::PeelTo::ObjectKind(gix_object::Kind::Tree),
                        b"blob" => delegate::PeelTo::ObjectKind(gix_object::Kind::Blob),
                        b"object" => delegate::PeelTo::ValidObject,
                        b"" => delegate::PeelTo::RecursiveTagObject,
                        regex if regex.starts_with(b"/") => {
                            let (regex, negated) = parse_regex_prefix(regex[1..].as_bstr())?;
                            if !regex.is_empty() {
                                delegate.find(regex, negated).or_raise(|| {
                                    Error::new(format!("Delegate couldn't find '{regex}' (negated: {negated})"))
                                })?;
                            }
                            continue;
                        }
                        invalid => return Err(Error::new_with_input("cannot peel to unknown target", invalid).raise()),
                    };
                    delegate.peel_until(target).or_raise(|| {
                        Error::new_with_input(
                            format!("delegate.peel_until({target:?}) failed"),
                            past_sep.unwrap_or_default(),
                        )
                    })?;
                } else if past_sep.and_then(<[_]>::first) == Some(&b'!') {
                    let rest = input[cursor + 1..].as_bstr();
                    let kind = spec::Kind::ExcludeReachableFromParents;
                    delegate
                        .kind(kind)
                        .or_raise(|| Error::new_with_input(format!("delegate.kind({kind:?}) failed"), rest))?;
                    delegate.done().or_raise(|| Error::new_with_input(done_msg, rest))?;
                    return Ok(rest);
                } else if past_sep.and_then(<[_]>::first) == Some(&b'@') {
                    let rest = input[cursor + 1..].as_bstr();
                    let kind = spec::Kind::IncludeReachableFromParents;
                    delegate
                        .kind(kind)
                        .or_raise(|| Error::new_with_input(format!("delegate.kind({kind:?}) failed"), rest))?;
                    delegate.done().or_raise(|| Error::new_with_input(done_msg, rest))?;
                    return Ok(rest);
                } else {
                    let parent = delegate::Traversal::NthParent(1);
                    delegate.traverse(parent).or_raise(|| {
                        Error::new_with_input(
                            format!("delegate.parent({parent:?}) failed",),
                            past_sep.unwrap_or_default(),
                        )
                    })?;
                }
            }
            b':' => {
                let to = delegate::PeelTo::Path(input[cursor..].as_bstr());
                delegate
                    .peel_until(to)
                    .or_raise(|| Error::new(format!("delegate.peel_until({to:?}) failed")))?;
                return Ok("".into());
            }
            _ => return Ok(input[cursor - 1..].as_bstr()),
        }
    }
    Ok("".into())
}

fn parse_regex_prefix(regex: &BStr) -> Result<(&BStr, bool), Error> {
    Ok(match regex.strip_prefix(b"!") {
        Some(regex) if regex.first() == Some(&b'!') => (regex.as_bstr(), false),
        Some(regex) if regex.first() == Some(&b'-') => (regex[1..].as_bstr(), true),
        Some(_regex) => return Err(Error::new_with_input("need one character after /!, typically -", regex)),
        None => (regex, false),
    })
}

fn try_parse_usize(input: &BStr) -> Result<Option<(usize, usize)>, Error> {
    let mut bytes = input.iter().peekable();
    if bytes.peek().filter(|&&&b| b == b'-' || b == b'+').is_some() {
        return Err(Error::new_with_input(
            "negative or explicitly positive numbers are invalid here",
            input,
        ));
    }
    let num_digits = bytes.take_while(|b| b.is_ascii_digit()).count();
    if num_digits == 0 {
        return Ok(None);
    }
    let input = &input[..num_digits];
    let number = try_parse(input)?.ok_or_else(|| Error::new_with_input("could not parse number", input))?;
    Ok(Some((number, num_digits)))
}

fn try_parse_isize(input: &BStr) -> Result<Option<(isize, bool, usize)>, Error> {
    let mut bytes = input.iter().peekable();
    if bytes.peek().filter(|&&&b| b == b'+').is_some() {
        return Err(Error::new_with_input(
            "explicitly positive numbers are invalid here",
            input,
        ));
    }
    let negative = bytes.peek() == Some(&&b'-');
    let num_digits = bytes.take_while(|b| b.is_ascii_digit() || *b == &b'-').count();
    if num_digits == 0 {
        return Ok(None);
    } else if num_digits == 1 && negative {
        return Ok(Some((-1, negative, num_digits)));
    }
    let input = &input[..num_digits];
    let number = try_parse(input)?.ok_or_else(|| Error::new_with_input("could not parse number", input))?;
    Ok(Some((number, negative, num_digits)))
}

fn try_range(input: &BStr) -> Option<(&[u8], spec::Kind)> {
    input
        .strip_prefix(b"...")
        .map(|rest| (rest, spec::Kind::ReachableToMergeBase))
        .or_else(|| input.strip_prefix(b"..").map(|rest| (rest, spec::Kind::RangeBetween)))
}

fn next(i: &BStr) -> (u8, &BStr) {
    let b = i[0];
    (b, i[1..].as_bstr())
}
