use bstr::{BStr, ByteSlice};
use gix_error::Result;
use gix_error::{OptionExt, bail, validation};

use crate::Entry;

/// An iterator to parse mailmap lines on-demand.
pub struct Lines<'a> {
    lines: bstr::Lines<'a>,
    line_no: usize,
}

impl<'a> Lines<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Lines {
            lines: input.as_bstr().lines(),
            line_no: 0,
        }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = Result<Entry<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        for line in self.lines.by_ref() {
            self.line_no += 1;
            match line.first() {
                None => continue,
                Some(b) if *b == b'#' => continue,
                Some(_) => {}
            }
            if line.trim().is_empty() {
                continue;
            }
            return Some(parse_line(trim_git_whitespace(line), self.line_no));
        }
        None
    }
}

fn parse_line(line: &BStr, line_number: usize) -> Result<Entry<'_>> {
    let (name1, email1, rest) = parse_name_and_email(line, line_number, false)?;
    let (name2, email2, _rest) = parse_name_and_email(rest, line_number, true).unwrap_or((None, None, rest));
    if email1.is_none() {
        bail!(validation(format!("Line {line_number} does not contain an email")).with_input(line));
    }
    Ok(match (name1, email1, name2, email2) {
        (Some(proper_name), Some(commit_email), None, None) => Entry::change_name_by_email(proper_name, commit_email),
        (None, Some(proper_email), None, Some(commit_email)) => {
            Entry::change_email_by_email(proper_email, commit_email)
        }
        (Some(proper_name), Some(proper_email), None, Some(commit_email)) => {
            Entry::change_name_and_email_by_email(proper_name, proper_email, commit_email)
        }
        (Some(proper_name), Some(proper_email), Some(commit_name), Some(commit_email)) => {
            Entry::change_name_and_email_by_name_and_email(proper_name, proper_email, commit_name, commit_email)
        }
        (None, Some(proper_email), Some(commit_name), Some(commit_email)) => {
            Entry::change_email_by_name_and_email(proper_email, commit_name, commit_email)
        }
        _ => {
            bail!(
                "{line_number}: Emails without a name or email to map to are invalid"
                    .validation()
                    .with_input(line)
            );
        }
    })
}

fn parse_name_and_email(
    line: &BStr,
    line_number: usize,
    allow_empty_email: bool,
) -> Result<(Option<&'_ BStr>, Option<&'_ BStr>, &'_ BStr)> {
    match line.find_byte(b'<') {
        Some(start_bracket) => {
            let email = &line[start_bracket + 1..];
            let closing_bracket = email.find_byte(b'>').ok_or_raise(|| {
                validation(format!("{line_number}: Missing closing bracket '>' in email")).with_input(line)
            })?;
            let email = trim_git_whitespace(&email[..closing_bracket]);
            if email.is_empty() && !allow_empty_email {
                bail!(validation(format!("{line_number}: Email must not be empty")).with_input(line));
            }
            let name = trim_git_whitespace(&line[..start_bracket]);
            let rest = line[start_bracket + closing_bracket + 2..].as_bstr();
            Ok(((!name.is_empty()).then_some(name), Some(email), rest))
        }
        None => Ok((None, None, line)),
    }
}

/// Remove the whitespace that Git removes at the edges of a name: space, tab, newline and carriage
/// return, the characters its `isspace()` accepts. Any other whitespace, such as a no-break space,
/// a vertical tab or a form feed, is part of the name or the email.
fn trim_git_whitespace(value: &[u8]) -> &BStr {
    value.trim_with(|c| matches!(c, ' ' | '\t' | '\n' | '\r')).as_bstr()
}
