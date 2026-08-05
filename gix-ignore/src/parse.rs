use bstr::ByteSlice;

/// An iterator over line-wise ignore patterns parsed from a buffer.
pub struct Lines<'a> {
    lines: bstr::Lines<'a>,
    line_no: usize,
    /// Only if `true` we will be able to parse precious files.
    support_precious: bool,
}

impl<'a> Lines<'a> {
    /// Create a new instance from `buf` to parse ignore patterns from.
    ///
    /// If `support_precious` is `true`, we will parse `$` prefixed entries as precious.
    /// This is backward-incompatible as files that actually start with `$` like `$houdini`
    /// will then not be ignored anymore, instead it ignores `houdini`.
    pub fn new(buf: &'a [u8], support_precious: bool) -> Self {
        let bom = unicode_bom::Bom::from(buf);
        Lines {
            lines: buf[bom.len()..].lines(),
            line_no: 0,
            support_precious,
        }
    }
}

impl Iterator for Lines<'_> {
    type Item = (gix_glob::Pattern, usize, crate::Kind);

    fn next(&mut self) -> Option<Self::Item> {
        for mut line in self.lines.by_ref() {
            self.line_no += 1;
            let first = match line.first().copied() {
                Some(b'#') | None => continue,
                Some(c) => c,
            };
            let (kind, can_negate) = if self.support_precious && first == b'$' {
                line = &line[1..];
                (crate::Kind::Precious, false)
            } else {
                let second = line.get(1);
                // Negating a precious entry is contradictory, but that's only true while `$` means
                // precious. Without that support `$` is an ordinary character, and Git parses
                // `!$foo` as a negation of the literal `$foo` - dropping it would make the
                // backward-compatible mode reject a valid line.
                if self.support_precious && first == b'!' && second == Some(&b'$') {
                    gix_trace::error!(
                        "Line {} starts with !$ which is not allowed ('{}')",
                        self.line_no,
                        line.as_bstr()
                    );
                    continue;
                }
                if first == b'\\' && second == Some(&b'$') {
                    line = &line[1..];
                }
                (crate::Kind::Expendable, true)
            };

            line = truncate_non_escaped_trailing_spaces(line);
            let res = if can_negate {
                gix_glob::Pattern::from_bytes(line)
            } else {
                gix_glob::Pattern::from_bytes_without_negation(line)
            };
            match res {
                None => continue,
                Some(pattern) => return Some((pattern, self.line_no, kind)),
            }
        }
        None
    }
}

/// We always copy just because that's ultimately needed anyway, not because we always have to.
fn truncate_non_escaped_trailing_spaces(buf: &[u8]) -> &[u8] {
    let mut last_space_pos = None;
    let mut bytes = buf.iter().enumerate();
    while let Some((pos, b)) = bytes.next() {
        match *b {
            b' ' => {
                last_space_pos.get_or_insert(pos);
                continue;
            }
            b'\\' if bytes.next().is_none() => {
                return buf;
            }
            _ => {}
        }
        last_space_pos = None;
    }

    if let Some(pos) = last_space_pos {
        &buf[..pos]
    } else {
        buf
    }
}
