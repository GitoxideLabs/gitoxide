use bstr::{BStr, BString, ByteSlice};

use crate::{File, file::Section, parse::Event};

impl File<'_> {
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
                event.write_to(&mut out)?;
            }

            if !ends_with_newline(self.frontmatter_events.as_ref(), nl, true) && self.sections.values().any(&mut filter)
            {
                out.write_all(nl)?;
            }
        }

        let mut prev_section_ended_with_newline = true;
        for section_id in &self.section_order {
            if !prev_section_ended_with_newline {
                out.write_all(nl)?;
            }
            let section = self.sections.get(section_id).expect("known section-id");
            if !filter(section) {
                continue;
            }
            section.write_to(&mut out)?;

            prev_section_ended_with_newline = ends_with_newline(section.body.0.as_ref(), nl, false);
            if let Some(post_matter) = self.frontmatter_post_section.get(section_id) {
                if !prev_section_ended_with_newline {
                    out.write_all(nl)?;
                }
                for event in post_matter {
                    event.write_to(&mut out)?;
                }
                prev_section_ended_with_newline = ends_with_newline(post_matter, nl, prev_section_ended_with_newline);
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

pub(crate) fn ends_with_newline(e: &[crate::parse::Event<'_>], nl: impl AsRef<[u8]>, default: bool) -> bool {
    if e.is_empty() {
        return default;
    }
    e.iter()
        .rev()
        .take_while(|e| e.to_bstr_lossy().iter().all(u8::is_ascii_whitespace))
        .find_map(|e| e.to_bstr_lossy().contains_str(nl.as_ref()).then_some(true))
        .unwrap_or(false)
}

pub(crate) fn extract_newline<'a>(e: &'a Event<'_>) -> Option<&'a BStr> {
    Some(match e {
        Event::Newline(b) => {
            let nl = b.as_ref();

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
