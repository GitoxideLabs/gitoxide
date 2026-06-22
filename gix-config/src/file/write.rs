use bstr::{BStr, BString, ByteSlice};

use crate::{File, file::Section, parse::Event};

impl File {
    /// Serialize this type into a `BString` for convenience.
    ///
    /// Note that `to_string()` can also be used, but might not be lossless.
    #[must_use]
    pub fn to_bstring(&self) -> BString {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("io error impossible");
        buf.into()
    }

    /// Stream ourselves to the given `out` in order to reproduce this file mostly losslessly
    /// as it was parsed, while writing only sections for which `filter` returns true.
    pub fn write_to_filter(
        &self,
        mut out: &mut dyn std::io::Write,
        mut filter: impl FnMut(&Section<'_>) -> bool,
    ) -> std::io::Result<()> {
        let nl = self.detect_newline_style();

        {
            for event in self.frontmatter_events.as_ref() {
                event.write_to_in(&self.event_backing, &mut out)?;
            }

            if !ends_with_newline(self.frontmatter_events.as_ref(), &self.event_backing, nl, true)
                && self
                    .sections
                    .values()
                    .map(|section| Section::from_data(section, &self.event_backing))
                    .any(|section| filter(&section))
            {
                out.write_all(nl)?;
            }
        }

        let mut prev_section_ended_with_newline = true;
        for section_id in &self.section_order {
            if !prev_section_ended_with_newline {
                out.write_all(nl)?;
            }
            let section_data = self.sections.get(section_id).expect("known section-id");
            let section = Section::from_data(section_data, &self.event_backing);
            if !filter(&section) {
                continue;
            }
            section_data.header.write_to_in(&self.event_backing, &mut *out)?;
            write_body_to(&section_data.body.0, &self.event_backing, nl, &mut *out)?;

            prev_section_ended_with_newline =
                ends_with_newline(section_data.body.0.as_ref(), &self.event_backing, nl, false);
            if let Some(post_matter) = self.frontmatter_post_section.get(section_id) {
                if !prev_section_ended_with_newline {
                    out.write_all(nl)?;
                }
                for event in post_matter {
                    event.write_to_in(&self.event_backing, &mut out)?;
                }
                prev_section_ended_with_newline =
                    ends_with_newline(post_matter, &self.event_backing, nl, prev_section_ended_with_newline);
            }
        }

        if !prev_section_ended_with_newline {
            out.write_all(nl)?;
        }

        Ok(())
    }

    /// Stream ourselves to the given `out`, in order to reproduce this file mostly losslessly
    /// as it was parsed.
    pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        self.write_to_filter(out, |_| true)
    }
}

fn write_body_to(
    events: &[crate::parse::Event],
    backing: &[u8],
    nl: &BStr,
    mut out: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    if events.is_empty() {
        return Ok(());
    }

    if !events
        .iter()
        .take_while(|e| !matches!(e, Event::SectionValueName(_)))
        .any(|e| e.to_bstr_lossy_in(backing).contains_str(nl))
    {
        out.write_all(nl)?;
    }

    let mut saw_newline_after_value = true;
    let mut in_key_value_pair = false;
    for (idx, event) in events.iter().enumerate() {
        match event {
            Event::SectionValueName(_) => {
                if !saw_newline_after_value {
                    out.write_all(nl)?;
                }
                saw_newline_after_value = false;
                in_key_value_pair = true;
            }
            Event::Newline(_) if !in_key_value_pair => {
                saw_newline_after_value = true;
            }
            Event::Value(_) | Event::ValueDone(_) => {
                in_key_value_pair = false;
            }
            _ => {}
        }
        event.write_to_in(backing, &mut out)?;
        if matches!(event, Event::ValueNotDone(_)) && !matches!(events.get(idx + 1), Some(Event::Newline(_))) {
            out.write_all(nl)?;
        }
    }
    Ok(())
}

pub(crate) fn ends_with_newline(
    e: &[crate::parse::Event],
    backing: &[u8],
    nl: impl AsRef<[u8]>,
    default: bool,
) -> bool {
    if e.is_empty() {
        return default;
    }
    e.iter()
        .rev()
        .take_while(|e| e.to_bstr_lossy_in(backing).iter().all(u8::is_ascii_whitespace))
        .find_map(|e| e.to_bstr_lossy_in(backing).contains_str(nl.as_ref()).then_some(true))
        .unwrap_or(false)
}

pub(crate) fn extract_newline<'a>(e: &'a Event, backing: &'a [u8]) -> Option<&'a BStr> {
    Some(match e {
        Event::Newline(b) => {
            let nl = b.as_slice_in(backing);

            // Newlines are parsed consecutively, be sure we only take the smallest possible variant
            if nl.contains(&b'\r') {
                "\r\n".into()
            } else {
                "\n".into()
            }
        }
        _ => return None,
    })
}

pub(crate) fn platform_newline() -> &'static BStr {
    if cfg!(windows) { "\r\n" } else { "\n" }.into()
}
