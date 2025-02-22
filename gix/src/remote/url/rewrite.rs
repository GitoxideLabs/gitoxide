use gix_features::threading::OwnShared;

use crate::{
    bstr::{BStr, BString, ByteVec},
    config,
    remote::Direction,
};

#[derive(Debug, Clone)]
struct Replace {
    find: BString,
    with: OwnShared<BString>,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct Rewrite {
    url_rewrite: Vec<Replace>,
    push_url_rewrite: Vec<Replace>,
}

/// Init
impl Rewrite {
    pub fn from_config(
        config: &gix_config::File<'static>,
        mut filter: fn(&gix_config::file::Metadata) -> bool,
    ) -> Rewrite {
        config
            .sections_by_name_and_filter("url", &mut filter)
            .map(|sections| {
                let mut url_rewrite = Vec::new();
                let mut push_url_rewrite = Vec::new();
                for section in sections {
                    let replace = match section.header().subsection_name() {
                        Some(base) => OwnShared::new(base.to_owned()),
                        None => continue,
                    };

                    for instead_of in section.values(config::tree::Url::INSTEAD_OF.name) {
                        url_rewrite.push(Replace {
                            with: OwnShared::clone(&replace),
                            find: instead_of.into_owned(),
                        });
                    }
                    for instead_of in section.values(config::tree::Url::PUSH_INSTEAD_OF.name) {
                        push_url_rewrite.push(Replace {
                            with: OwnShared::clone(&replace),
                            find: instead_of.into_owned(),
                        });
                    }
                }
                Rewrite {
                    url_rewrite,
                    push_url_rewrite,
                }
            })
            .unwrap_or_default()
    }
}

/// Access
impl Rewrite {
    fn replacements_for(&self, direction: Direction) -> &[Replace] {
        match direction {
            Direction::Fetch => &self.url_rewrite,
            Direction::Push => &self.push_url_rewrite,
        }
    }

    pub fn longest(&self, url: &gix_url::Url, direction: Direction) -> Option<BString> {
        if self.replacements_for(direction).is_empty() {
            None
        } else {
            let mut url = url.to_bstring();
            self.rewrite_url_in_place(&mut url, direction).then_some(url)
        }
    }

    /// Rewrite the given `url` of `direction` and return `true` if a replacement happened.
    ///
    /// Note that the result must still be checked for validity, it might not be a valid URL as we do a syntax-unaware replacement.
    pub fn rewrite_url_in_place(&self, url: &mut BString, direction: Direction) -> bool {
        self.replacements_for(direction)
            .iter()
            .fold(None::<(usize, &BStr)>, |mut acc, replace| {
                if url.starts_with(replace.find.as_ref()) {
                    let (bytes_matched, prev_rewrite_with) =
                        acc.get_or_insert((replace.find.len(), replace.with.as_slice().into()));
                    if *bytes_matched < replace.find.len() {
                        *bytes_matched = replace.find.len();
                        *prev_rewrite_with = replace.with.as_slice().into();
                    }
                }
                acc
            })
            .map(|(bytes_matched, replace_with)| {
                url.replace_range(..bytes_matched, replace_with);
            })
            .is_some()
    }
}
