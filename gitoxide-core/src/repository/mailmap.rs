use std::io;

#[cfg(feature = "serde")]
use gix::mailmap::Entry;
use gix::{
    Result,
    bstr::{BString, ByteSlice},
    error::{ResultExt, bail, validation},
};

use crate::OutputFormat;

#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
struct JsonEntry {
    new_name: Option<String>,
    new_email: Option<String>,
    old_name: Option<String>,
    old_email: String,
}

#[cfg(feature = "serde")]
impl<'a> From<Entry<'a>> for JsonEntry {
    fn from(v: Entry<'a>) -> Self {
        use gix::bstr::ByteSlice;
        JsonEntry {
            new_name: v.new_name().map(|s| s.to_str_lossy().into_owned()),
            new_email: v.new_email().map(|s| s.to_str_lossy().into_owned()),
            old_name: v.old_name().map(|s| s.to_str_lossy().into_owned()),
            old_email: v.old_email().to_str_lossy().into_owned(),
        }
    }
}

pub fn entries(
    repo: gix::Repository,
    format: OutputFormat,
    #[cfg_attr(not(feature = "serde"), allow(unused_variables))] out: impl io::Write,
    mut err: impl io::Write,
) -> Result<()> {
    if format == OutputFormat::Human {
        writeln!(err, "Defaulting to JSON as human format isn't implemented").ok();
    }

    let mut mailmap = gix::mailmap::Snapshot::default();
    if let Err(e) = repo.open_mailmap_into(&mut mailmap) {
        writeln!(err, "Error while loading mailmap, the first error is: {e}").ok();
    }

    #[cfg(feature = "serde")]
    serde_json::to_writer_pretty(out, &mailmap.iter().map(JsonEntry::from).collect::<Vec<_>>()).or_error()?;

    Ok(())
}

pub fn check(
    repo: gix::Repository,
    format: OutputFormat,
    contacts: Vec<BString>,
    mut out: impl io::Write,
    mut err: impl io::Write,
) -> Result<()> {
    if format != OutputFormat::Human {
        bail!(gix::error::unsupported("Only human output is supported right now"));
    }
    if contacts.is_empty() {
        bail!(validation("specify at least one contact to run through the mailmap"))
    }

    let mut mailmap = gix::mailmap::Snapshot::default();
    if let Err(err) = repo.open_mailmap_into(&mut mailmap) {
        bail!(err);
    }

    let mut buf = Vec::new();
    for contact in contacts {
        let actor = match gix::actor::IdentityRef::from_bytes(&contact) {
            Ok(a) => a,
            Err(_) => {
                let Some(email) = contact
                    .trim_start()
                    .strip_prefix(b"<")
                    .and_then(|rest| rest.trim_end().strip_suffix(b">"))
                else {
                    writeln!(err, "Failed to parse contact '{contact}' - skipping").or_error()?;
                    continue;
                };
                gix::actor::IdentityRef {
                    name: "".into(),
                    email: email.into(),
                }
            }
        };
        let resolved = mailmap.resolve_cow(gix::actor::SignatureRef {
            name: actor.name,
            email: actor.email,
            time: Default::default(),
        });
        let resolved = gix::actor::IdentityRef {
            name: resolved.name.as_ref(),
            email: resolved.email.as_ref(),
        };
        buf.clear();
        resolved.write_to(&mut buf).or_error()?;

        out.write_all(&buf).or_error()?;
        out.write_all(b"\n").or_error()?;
    }
    Ok(())
}
