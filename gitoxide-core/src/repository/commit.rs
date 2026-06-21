use std::{
    borrow::Cow,
    io::{Read, Write},
    process::Stdio,
};

use anyhow::{Context, Result, anyhow, bail};
use gix::{
    bstr::{BStr, BString},
    objs::commit::SIGNATURE_FIELD_NAME,
};

/// Note that this is a quick implementation of commit signature verification that ignores a lot of what
/// git does and can do, while focussing on the gist of it.
/// For this to go into `gix`, one will have to implement many more options and various validation programs.
pub fn verify(repo: gix::Repository, rev_spec: Option<&str>) -> Result<()> {
    let rev_spec = rev_spec.unwrap_or("HEAD");
    let commit = repo
        .rev_parse_single(format!("{rev_spec}^{{commit}}").as_str())?
        .object()?
        .into_commit();
    let (signature, signed_data) = commit
        .signature()
        .context("Could not parse commit to obtain signature")?
        .ok_or_else(|| anyhow!("Commit at {rev_spec} is not signed"))?;

    let mut signature_storage = tempfile::NamedTempFile::new()?;
    signature_storage.write_all(signature.as_ref())?;
    let signed_storage = signature_storage.into_temp_path();

    let mut cmd: std::process::Command = gix::command::prepare("gpg").into();
    cmd.args(["--keyid-format=long", "--status-fd=1", "--verify"])
        .arg(&signed_storage)
        .arg("-")
        .stdin(Stdio::piped());
    gix::trace::debug!("About to execute {cmd:?}");
    let mut child = cmd.spawn()?;
    child
        .stdin
        .take()
        .expect("configured")
        .write_all(signed_data.to_bstring().as_ref())?;

    if !child.wait()?.success() {
        bail!("Command {cmd:?} failed");
    }
    Ok(())
}

/// Note that this is a quick first prototype that lacks some of the features provided by `git verify-commit`.
pub fn sign(repo: gix::Repository, rev_spec: Option<&str>, mut out: impl std::io::Write) -> Result<()> {
    let rev_spec = rev_spec.unwrap_or("HEAD");
    let object = repo
        .rev_parse_single(format!("{rev_spec}^{{commit}}").as_str())?
        .object()?;
    let mut commit_ref = object.to_commit_ref();
    if commit_ref.extra_headers().pgp_signature().is_some() {
        gix::trace::info!("The commit {id} is already signed, did nothing", id = object.id);
        writeln!(out, "{id}", id = object.id)?;
        return Ok(());
    }

    let mut cmd: std::process::Command = gix::command::prepare("gpg").into();
    cmd.args([
        "--keyid-format=long",
        "--status-fd=2",
        "--detach-sign",
        "--sign",
        "--armor",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped());

    gix::trace::debug!("About to execute {cmd:?}");
    let mut child = cmd.spawn()?;
    child.stdin.take().expect("to be present").write_all(&object.data)?;

    if !child.wait()?.success() {
        bail!("Command {cmd:?} failed");
    }

    let mut signed_data = Vec::new();
    child.stdout.expect("to be present").read_to_end(&mut signed_data)?;

    commit_ref
        .extra_headers
        .push((BStr::new(SIGNATURE_FIELD_NAME), Cow::Owned(BString::new(signed_data))));

    let signed_id = repo.write_object(&commit_ref)?;
    writeln!(&mut out, "{signed_id}")?;

    Ok(())
}

pub fn describe(
    mut repo: gix::Repository,
    rev_spec: Option<&str>,
    mut out: impl std::io::Write,
    mut err: impl std::io::Write,
    describe::Options {
        all_tags,
        all_refs,
        first_parent,
        always,
        statistics,
        max_candidates,
        long_format,
        dirty_suffix,
    }: describe::Options,
) -> Result<()> {
    repo.object_cache_size_if_unset(4 * 1024 * 1024);
    let commit = match rev_spec {
        Some(spec) => repo.rev_parse_single(spec)?.object()?.try_into_commit()?,
        None => repo.head_commit()?,
    };
    use gix::commit::describe::SelectRef::*;
    let select_ref = if all_refs {
        AllRefs
    } else if all_tags {
        AllTags
    } else {
        Default::default()
    };
    let resolution = commit
        .describe()
        .names(select_ref)
        .traverse_first_parent(first_parent)
        .id_as_fallback(always)
        .max_candidates(max_candidates)
        .try_resolve()?
        .with_context(|| format!("Did not find a single candidate ref for naming id '{}'", commit.id))?;

    if statistics {
        writeln!(err, "traversed {} commits", resolution.outcome.commits_seen)?;
    }

    let mut describe_id = resolution.format_with_dirty_suffix(dirty_suffix)?;
    describe_id.long(long_format);

    writeln!(out, "{describe_id}")?;
    Ok(())
}

pub fn name_rev(
    repo: gix::Repository,
    rev_spec: Option<&str>,
    mut out: impl std::io::Write,
    name_rev::Options {
        tags,
        refs,
        exclude,
        always,
        no_undefined,
        name_only,
    }: name_rev::Options,
) -> Result<()> {
    let commit = match rev_spec {
        Some(spec) => repo
            .rev_parse_single(format!("{spec}^{{commit}}").as_str())?
            .object()?
            .into_commit(),
        None => repo.head_commit()?,
    };
    let mut platform = commit
        .name_rev()
        .names(if tags {
            gix::commit::name_rev::SelectRef::AllTags
        } else {
            gix::commit::name_rev::SelectRef::AllRefs
        })
        .id_as_fallback(always);

    for pattern in refs {
        platform = platform.include_ref(pattern);
    }
    for pattern in exclude {
        platform = platform.exclude_ref(pattern);
    }

    match platform.try_format()? {
        Some(format) => {
            if name_only {
                let mut name = format.to_string();
                if tags {
                    name = name.strip_prefix("tags/").unwrap_or(&name).to_owned();
                }
                writeln!(out, "{name}")?;
            } else {
                let id_or_spec = rev_spec.map_or_else(|| commit.id.to_string(), ToOwned::to_owned);
                writeln!(out, "{id_or_spec} {format}")?;
            }
        }
        None if no_undefined => anyhow::bail!("Cannot describe '{}'", commit.id),
        None if name_only => writeln!(out, "undefined")?,
        None => {
            let id_or_spec = rev_spec.map_or_else(|| commit.id.to_string(), ToOwned::to_owned);
            writeln!(out, "{id_or_spec} undefined")?;
        }
    }

    Ok(())
}

pub mod describe {
    #[derive(Debug, Clone)]
    pub struct Options {
        pub all_tags: bool,
        pub all_refs: bool,
        pub first_parent: bool,
        pub always: bool,
        pub long_format: bool,
        pub statistics: bool,
        pub max_candidates: usize,
        pub dirty_suffix: Option<String>,
    }
}

pub mod name_rev {
    #[derive(Debug, Clone)]
    pub struct Options {
        pub tags: bool,
        pub refs: Vec<String>,
        pub exclude: Vec<String>,
        pub always: bool,
        pub no_undefined: bool,
        pub name_only: bool,
    }
}
