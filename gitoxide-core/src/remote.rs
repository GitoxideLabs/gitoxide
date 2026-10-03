use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{OutputFormat, net};
use gix::protocol::transport::client::blocking_io::connect;
use gix::{
    NestedProgress, Result,
    config::tree::Key,
    error::{ResultExt, bail},
    objs::bstr::ByteSlice,
    protocol::{self, handshake::Ref, transport},
    refs::{
        Target,
        transaction::{PreviousValue, RefEdit},
    },
};

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=2;

pub struct Context<W> {
    pub format: OutputFormat,
    pub out: W,
    pub object_hash: gix::hash::Kind,
    pub write_reflog: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RefsWriteOutcome {
    pub num_refs: usize,
    pub elapsed: Duration,
}

pub fn refs<P, W>(
    protocol: Option<net::Protocol>,
    url: &str,
    refs_directory: Option<PathBuf>,
    mut progress: P,
    ctx: Context<W>,
) -> Result<()>
where
    W: io::Write,
    P: NestedProgress + 'static,
    P::SubProgress: 'static,
{
    if ctx.format != OutputFormat::Human {
        bail!(gix::error::unsupported("JSON output isn't supported"));
    }

    let mut transport = net::connect(
        url,
        connect::Options {
            version: protocol.unwrap_or_default().into(),
            ..Default::default()
        },
    )?;
    let trace_packetlines = std::env::var_os(
        gix::config::tree::Gitoxide::TRACE_PACKET
            .environment_override()
            .expect("set"),
    )
    .is_some();

    progress.info(format!("Connecting to {url:?}"));
    let agent = protocol::agent(gix::env::agent());
    let mut handshake = protocol::handshake(
        &mut transport.inner,
        transport::Service::UploadPack,
        protocol::credentials::builtin,
        vec![("agent".into(), Some(agent.clone()))],
        &mut progress,
    )?;
    let fetch_refmap = handshake.prepare_lsrefs_or_extract_refmap(
        ("agent", Some(agent)),
        false,
        protocol::fetch::refmap::init::Context {
            fetch_refspecs: Vec::new(),
            extra_refspecs: Vec::new(),
        },
    )?;

    let refmap = fetch_refmap.fetch_blocking(&mut progress, &mut transport.inner, trace_packetlines)?;
    let refs = refmap.remote_refs;

    let refs_write = refs_directory
        .map(|directory| write_refs(&refs, directory, ctx.object_hash, ctx.write_reflog))
        .transpose()?;
    print_refs(ctx.out, &refs, refs_write).or_error()?;

    Ok(())
}

fn print_refs(mut out: impl io::Write, refs: &[Ref], refs_write: Option<RefsWriteOutcome>) -> io::Result<()> {
    crate::repository::remote::refs::print(&mut out, refs)?;
    if let Some(outcome) = refs_write {
        writeln!(out)?;
        writeln!(out, "refs-write: {} refs in {:?}", outcome.num_refs, outcome.elapsed)?;
    }
    Ok(())
}

fn write_refs(
    refs: &[Ref],
    directory: PathBuf,
    object_hash: gix::hash::Kind,
    write_reflog: bool,
) -> Result<RefsWriteOutcome> {
    let _span = gix::trace::coarse!("write remote refs", refs = refs.len(), directory = ?directory);
    std::fs::create_dir_all(&directory).or_error()?;

    let start = Instant::now();
    let precompose_unicode = gix::fs::Capabilities::probe(&directory).precompose_unicode;
    let store = gix::RefStore::at_opts(
        directory,
        object_hash,
        gix::refs::store::init::Options {
            write_reflog: if write_reflog {
                gix::refs::store::WriteReflog::Always
            } else {
                gix::refs::store::WriteReflog::Disable
            },
            precompose_unicode,
            prohibit_windows_device_names: cfg!(windows),
        },
    );
    let edits = refs
        .iter()
        .map(ref_to_edit)
        .collect::<std::result::Result<Vec<_>, gix::refs::name::Error>>()
        .or_error()?;

    store
        .transaction()
        .prepare(
            edits,
            gix::lock::acquire::Fail::Immediately,
            gix::lock::acquire::Fail::Immediately,
        )?
        .commit(write_reflog.then_some(reflog_committer()))?;
    let outcome = RefsWriteOutcome {
        num_refs: refs.len(),
        elapsed: start.elapsed(),
    };
    gix::trace::info!(
        refs = outcome.num_refs,
        elapsed_secs = outcome.elapsed.as_secs_f64(),
        "wrote remote refs"
    );
    Ok(outcome)
}

fn reflog_committer() -> gix::actor::SignatureRef<'static> {
    gix::actor::SignatureRef {
        name: b"gitoxide".as_bstr(),
        email: b"gitoxide@example.com".as_bstr(),
        time: "0 +0000",
    }
}

fn ref_to_edit(ref_: &Ref) -> std::result::Result<RefEdit, gix::refs::name::Error> {
    let (name, target) = match ref_ {
        Ref::Unborn { full_ref_name, target } => (full_ref_name, Target::Symbolic(target.as_bstr().try_into()?)),
        Ref::Symbolic {
            full_ref_name, target, ..
        } => (full_ref_name, Target::Symbolic(target.as_bstr().try_into()?)),
        Ref::Peeled { full_ref_name, tag, .. } => (full_ref_name, Target::Object(*tag)),
        Ref::Direct { full_ref_name, object } => (full_ref_name, Target::Object(*object)),
    };
    Ok(RefEdit::update(
        name.as_bstr().try_into()?,
        target,
        PreviousValue::Any,
        "remote refs",
    ))
}
