use gix::bstr::{BStr, BString, ByteSlice};

use crate::OutputFormat;

#[derive(Eq, PartialEq, PartialOrd, Ord)]
enum VersionPart {
    String(BString),
    Number(usize),
}

/// `Version` is used to store multi-part version numbers. It does so in a rather naive way,
/// only distinguishing between parts that can be parsed as an integer and those that cannot.
///
/// `Version` does not parse version numbers in any structure-aware way, so `v0.a` is parsed into
/// `v`, `0`, `.a`.
///
/// Comparing two `Version`s comes down to comparing their `parts`. `parts` are either compared
/// numerically or lexicographically, depending on whether they are an integer or not. That way,
/// `v0.9` sorts before `v0.10` as one would expect from a version number.
///
/// When comparing versions of different lengths, shorter versions sort before longer ones (e.g.,
/// `v1.0` < `v1.0.1`). String parts always sort before numeric parts when compared directly.
#[derive(Eq, PartialEq, Ord, PartialOrd)]
struct Version {
    parts: Vec<VersionPart>,
}

impl Version {
    fn parse(version: &BStr) -> Self {
        let parts = version
            .chunk_by(|a, b| a.is_ascii_digit() == b.is_ascii_digit())
            .map(|part| {
                if let Ok(part) = part.to_str() {
                    part.parse::<usize>()
                        .map_or_else(|_| VersionPart::String(part.into()), VersionPart::Number)
                } else {
                    VersionPart::String(part.into())
                }
            })
            .collect();

        Self { parts }
    }
}

pub fn list(repo: gix::Repository, out: &mut dyn std::io::Write, format: OutputFormat) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        anyhow::bail!("JSON output isn't supported");
    }

    let platform = repo.references()?;

    let mut tags: Vec<_> = platform
        .tags()?
        .flatten()
        .map(|mut reference| {
            let tag = reference.peel_to_tag();
            let tag_ref = tag.as_ref().map(gix::Tag::decode);

            // `name` is the name of the file in `refs/tags/`.
            // This applies to both lightweight and annotated tags.
            let name = reference.name().shorten();
            let mut fields = Vec::new();
            let version = Version::parse(name);
            match tag_ref {
                Ok(Ok(tag_ref)) => {
                    // `tag_name` is the name provided by the user via `git tag -a/-s/-u`.
                    // It is only present for annotated tags.
                    fields.push(format!(
                        "tag name: {}",
                        if name == tag_ref.name { "*".into() } else { tag_ref.name }
                    ));
                    if tag_ref.signature.is_some() {
                        fields.push("signed".into());
                    }

                    (version, format!("{name} [{fields}]", fields = fields.join(", ")))
                }
                _ => (version, name.to_string()),
            }
        })
        .collect();

    tags.sort_by(|a, b| a.0.cmp(&b.0));

    for (_, tag) in tags {
        writeln!(out, "{tag}")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn sorts_versions_correctly() {
        let mut actual = vec![
            "v2.0.0",
            "v1.10.0",
            "v1.2.1",
            "v1.0.0-beta",
            "v1.2",
            "v0.10.0",
            "v0.9.0",
            "v1.2.0",
            "v0.1.a",
            "v0.1.0",
            "v10.0.0",
            "1.0.0",
            "v1.0.0-alpha",
            "v1.0.0",
        ];

        actual.sort_by(|&a, &b| Version::parse(a.into()).cmp(&Version::parse(b.into())));
        let expected = [
            "v0.1.0",
            "v0.1.a",
            "v0.9.0",
            "v0.10.0",
            "v1.0.0",
            "v1.0.0-alpha",
            "v1.0.0-beta",
            "v1.2",
            "v1.2.0",
            "v1.2.1",
            "v1.10.0",
            "v2.0.0",
            "v10.0.0",
            "1.0.0",
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn sorts_versions_with_different_lengths_correctly() {
        let v1 = Version::parse("v1.0".into());
        let v2 = Version::parse("v1.0.1".into());

        assert_eq!(v1.cmp(&v2), Ordering::Less);
        assert_eq!(v2.cmp(&v1), Ordering::Greater);
    }
}
