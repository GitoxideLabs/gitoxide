use bstr::ByteSlice;
use winnow::{
    combinator::{opt, preceded, terminated},
    prelude::*,
    token::take_till,
};

/// The information usually found in remote progress messages as sent by a git server during
/// fetch, clone and push operations.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RemoteProgress<'a> {
    #[cfg_attr(feature = "serde", serde(borrow))]
    /// The name of the action, like "clone".
    pub action: &'a bstr::BStr,
    /// The percentage to indicate progress, between 0 and 100.
    pub percent: Option<u32>,
    /// The amount of items already processed.
    pub step: Option<usize>,
    /// The maximum expected amount of items. `step` / `max` * 100 = `percent`.
    pub max: Option<usize>,
}

impl RemoteProgress<'_> {
    /// Parse the progress from a typical git progress `line` as sent by the remote.
    pub fn from_bytes(mut line: &[u8]) -> Option<RemoteProgress<'_>> {
        parse_progress(&mut line).ok().and_then(|r| {
            if r.percent.is_none() && r.step.is_none() && r.max.is_none() {
                None
            } else {
                Some(r)
            }
        })
    }

    /// Parse `text`, which is interpreted as error if `is_error` is true, as [`RemoteProgress`] and call the respective
    /// methods on the given `progress` instance.
    pub fn translate_to_progress(is_error: bool, text: &[u8], progress: &mut impl gix_features::progress::Progress) {
        fn progress_name(current: Option<String>, action: &[u8]) -> String {
            match current {
                Some(current) => format!(
                    "{}: {}",
                    current.split_once(':').map_or(&*current, |x| x.0),
                    action.as_bstr()
                ),
                None => action.as_bstr().to_string(),
            }
        }
        if is_error {
            // ignore keep-alive packages sent with 'sideband-all'
            if !text.is_empty() {
                progress.fail(progress_name(None, text));
            }
        } else {
            match RemoteProgress::from_bytes(text) {
                Some(RemoteProgress {
                    action,
                    percent: _,
                    step,
                    max,
                }) => {
                    progress.set_name(progress_name(progress.name(), action));
                    progress.init(max, gix_features::progress::count("objects"));
                    if let Some(step) = step {
                        progress.set(step);
                    }
                }
                None => progress.set_name(progress_name(progress.name(), text)),
            }
        }
    }
}

fn parse_number(i: &mut &[u8]) -> ModalResult<usize, ()> {
    take_till(0.., |c: u8| !c.is_ascii_digit())
        .try_map(gix_utils::btoi::to_signed)
        .parse_next(i)
}

fn next_optional_percentage(i: &mut &[u8]) -> ModalResult<Option<u32>, ()> {
    opt(terminated(
        preceded(
            take_till(0.., |c: u8| c.is_ascii_digit()),
            parse_number.try_map(u32::try_from),
        ),
        b"%",
    ))
    .parse_next(i)
}

fn next_optional_number(i: &mut &[u8]) -> ModalResult<Option<usize>, ()> {
    opt(preceded(take_till(0.., |c: u8| c.is_ascii_digit()), parse_number)).parse_next(i)
}

fn parse_progress<'i>(line: &mut &'i [u8]) -> ModalResult<RemoteProgress<'i>, ()> {
    let action = take_till(1.., |c| c == b':').parse_next(line)?;
    let percent = next_optional_percentage.parse_next(line)?;
    let step = next_optional_number.parse_next(line)?;
    let max = next_optional_number.parse_next(line)?;
    Ok(RemoteProgress {
        action: action.into(),
        percent,
        step,
        max,
    })
}
