/// The error returned by [`parse()`](crate::parse()).
pub type Error = gix_error::Exn<gix_error::ValidationError>;

use bstr::{BStr, ByteSlice};
use gix_error::{ErrorExt, OptionExt, ValidationError};

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
    type Item = Result<Entry<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        for line in self.lines.by_ref() {
            self.line_no += 1;
            match line.first() {
                None => continue,
                Some(b) if *b == b'#' => continue,
                Some(_) => {}
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            return parse_line(line.into(), self.line_no).into();
        }
        None
    }
}

fn parse_line(line: &BStr, line_number: usize) -> Result<Entry<'_>, Error> {
    let (name1, email1, rest) = parse_name_and_email(line, line_number)?;
    let (name2, email2, rest) = parse_name_and_email(rest, line_number)?;
    if !rest.trim().is_empty() {
        return Err(ValidationError::new_with_input(
            format!("Line {line_number} has too many names or emails, or none at all"),
            line,
        )
        .raise());
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
            return Err(ValidationError::new_with_input(
                format!("{line_number}: Emails without a name or email to map to are invalid"),
                line,
            )
            .raise())
        }
    })
}

fn parse_name_and_email(
    line: &BStr,
    line_number: usize,
) -> Result<(Option<&'_ BStr>, Option<&'_ BStr>, &'_ BStr), Error> {
    match line.find_byte(b'<') {
        Some(start_bracket) => {
            let email = &line[start_bracket + 1..];
            let closing_bracket = email.find_byte(b'>').ok_or_raise(|| {
                ValidationError::new_with_input(format!("{line_number}: Missing closing bracket '>' in email"), line)
            })?;
            let email = email[..closing_bracket].trim().as_bstr();
            if email.is_empty() {
                return Err(
                    ValidationError::new_with_input(format!("{line_number}: Email must not be empty"), line).raise(),
                );
            }
            let name = line[..start_bracket].trim().as_bstr();
            let rest = line[start_bracket + closing_bracket + 2..].as_bstr();
            Ok(((!name.is_empty()).then_some(name), Some(email), rest))
        }
        None => Ok((None, None, line)),
    }
}
