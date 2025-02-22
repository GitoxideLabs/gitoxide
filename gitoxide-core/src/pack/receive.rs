use crate::net;
use crate::pack::receive::protocol::fetch::negotiate;
use crate::OutputFormat;
use gix::config::tree::Key;
use gix::protocol::maybe_async;
use gix::remote::fetch::Error;
use gix::DynNestedProgress;
pub use gix::{
    hash::ObjectId,
    objs::bstr::{BString, ByteSlice},
    odb::pack,
    protocol,
    protocol::{
        fetch::{Arguments, Response},
        handshake::Ref,
        transport,
        transport::client::Capabilities,
    },
    NestedProgress, Progress,
};
use std::{
    io,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=3;
pub struct Context<W> {
    pub thread_limit: Option<usize>,
    pub format: OutputFormat,
    pub should_interrupt: Arc<AtomicBool>,
    pub out: W,
    pub object_hash: gix::hash::Kind,
}

#[maybe_async::maybe_async]
pub async fn receive<P, W>(
    protocol: Option<net::Protocol>,
    url: &str,
    directory: Option<PathBuf>,
    refs_directory: Option<PathBuf>,
    mut wanted_refs: Vec<BString>,
    mut progress: P,
    ctx: Context<W>,
) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: NestedProgress + 'static,
    P::SubProgress: 'static,
{
    let mut transport = net::connect(
        url,
        gix::protocol::transport::client::connect::Options {
            version: protocol.unwrap_or_default().into(),
            ..Default::default()
        },
    )
    .await?;
    let trace_packetlines = std::env::var_os(
        gix::config::tree::Gitoxide::TRACE_PACKET
            .environment_override()
            .expect("set"),
    )
    .is_some();

    let agent = gix::protocol::agent(gix::env::agent());
    let mut handshake = gix::protocol::fetch::handshake(
        &mut transport.inner,
        gix::protocol::credentials::builtin,
        vec![("agent".into(), Some(agent.clone()))],
        &mut progress,
    )
    .await?;
    if wanted_refs.is_empty() {
        wanted_refs.push("refs/heads/*:refs/remotes/origin/*".into());
    }
    let fetch_refspecs: Vec<_> = wanted_refs
        .into_iter()
        .map(|ref_name| {
            gix::refspec::parse(ref_name.as_bstr(), gix::refspec::parse::Operation::Fetch).map(|r| r.to_owned())
        })
        .collect::<Result<_, _>>()?;
    let user_agent = ("agent", Some(agent.clone().into()));
    let refmap = gix::protocol::fetch::RefMap::new(
        &mut progress,
        &fetch_refspecs,
        gix::protocol::fetch::Context {
            handshake: &mut handshake,
            transport: &mut transport.inner,
            user_agent: user_agent.clone(),
            trace_packetlines,
        },
        gix::protocol::fetch::refmap::init::Options::default(),
    )
    .await?;

    if refmap.mappings.is_empty() && !refmap.remote_refs.is_empty() {
        return Err(Error::NoMapping {
            refspecs: refmap.refspecs.clone(),
            num_remote_refs: refmap.remote_refs.len(),
        }
        .into());
    }

    let mut negotiate = Negotiate { refmap: &refmap };
    gix::protocol::fetch(
        &mut negotiate,
        |read_pack, progress, should_interrupt| {
            receive_pack_blocking(
                directory,
                refs_directory,
                read_pack,
                progress,
                &refmap.remote_refs,
                should_interrupt,
                ctx.out,
                ctx.thread_limit,
                ctx.object_hash,
                ctx.format,
            )
            .map(|_| true)
        },
        progress,
        &ctx.should_interrupt,
        gix::protocol::fetch::Context {
            handshake: &mut handshake,
            transport: &mut transport.inner,
            user_agent,
            trace_packetlines,
        },
        gix::protocol::fetch::Options {
            shallow_file: "no shallow file required as we reject it to keep it simple".into(),
            shallow: &Default::default(),
            tags: Default::default(),
            reject_shallow_remote: true,
        },
    )
    .await?;
    Ok(())
}

struct Negotiate<'a> {
    refmap: &'a gix::protocol::fetch::RefMap,
}

impl gix::protocol::fetch::Negotiate for Negotiate<'_> {
    fn mark_complete_and_common_ref(&mut self) -> Result<negotiate::Action, negotiate::Error> {
        Ok(negotiate::Action::MustNegotiate {
            remote_ref_target_known: vec![], /* we don't really negotiate */
        })
    }

    fn add_wants(&mut self, arguments: &mut Arguments, _remote_ref_target_known: &[bool]) -> bool {
        let mut has_want = false;
        for id in self.refmap.mappings.iter().filter_map(|m| m.remote.as_id()) {
            arguments.want(id);
            has_want = true;
        }
        has_want
    }

    fn one_round(
        &mut self,
        _state: &mut negotiate::one_round::State,
        _arguments: &mut Arguments,
        _previous_response: Option<&Response>,
    ) -> Result<(negotiate::Round, bool), negotiate::Error> {
        Ok((
            negotiate::Round {
                haves_sent: 0,
                in_vain: 0,
                haves_to_send: 0,
                previous_response_had_at_least_one_in_common: false,
            },
            // is done
            true,
        ))
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JsonBundleWriteOutcome {
    pub index_version: pack::index::Version,
    pub index_hash: String,

    pub data_hash: String,
    pub num_objects: u32,
}

impl From<pack::index::write::Outcome> for JsonBundleWriteOutcome {
    fn from(v: pack::index::write::Outcome) -> Self {
        JsonBundleWriteOutcome {
            index_version: v.index_version,
            num_objects: v.num_objects,
            data_hash: v.data_hash.to_string(),
            index_hash: v.index_hash.to_string(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JsonOutcome {
    pub index: JsonBundleWriteOutcome,
    pub pack_kind: pack::data::Version,

    pub index_path: Option<PathBuf>,
    pub data_path: Option<PathBuf>,

    pub refs: Vec<crate::repository::remote::JsonRef>,
}

impl JsonOutcome {
    pub fn from_outcome_and_refs(v: pack::bundle::write::Outcome, refs: &[Ref]) -> Self {
        JsonOutcome {
            index: v.index.into(),
            pack_kind: v.pack_version,
            index_path: v.index_path,
            data_path: v.data_path,
            refs: refs.iter().cloned().map(Into::into).collect(),
        }
    }
}

fn print_hash_and_path(out: &mut impl io::Write, name: &str, id: ObjectId, path: Option<PathBuf>) -> io::Result<()> {
    match path {
        Some(path) => writeln!(out, "{}: {} ({})", name, id, path.display()),
        None => writeln!(out, "{name}: {id}"),
    }
}

fn print(out: &mut impl io::Write, res: pack::bundle::write::Outcome, refs: &[Ref]) -> io::Result<()> {
    print_hash_and_path(out, "index", res.index.index_hash, res.index_path)?;
    print_hash_and_path(out, "pack", res.index.data_hash, res.data_path)?;
    writeln!(out)?;
    crate::repository::remote::refs::print(out, refs)?;
    Ok(())
}

fn write_raw_refs(refs: &[Ref], directory: PathBuf) -> std::io::Result<()> {
    let assure_dir_exists = |path: &BString| {
        assert!(!path.starts_with_str("/"), "no ref start with a /, they are relative");
        let path = directory.join(gix::path::from_byte_slice(path));
        std::fs::create_dir_all(path.parent().expect("multi-component path")).map(|_| path)
    };
    for r in refs {
        let (path, content) = match r {
            Ref::Unborn { full_ref_name, target } => {
                (assure_dir_exists(full_ref_name)?, format!("unborn HEAD: {target}"))
            }
            Ref::Symbolic {
                full_ref_name: path,
                target,
                ..
            } => (assure_dir_exists(path)?, format!("ref: {target}")),
            Ref::Peeled {
                full_ref_name: path,
                tag: object,
                ..
            }
            | Ref::Direct {
                full_ref_name: path,
                object,
            } => (assure_dir_exists(path)?, object.to_string()),
        };
        std::fs::write(path, content.as_bytes())?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn receive_pack_blocking(
    mut directory: Option<PathBuf>,
    mut refs_directory: Option<PathBuf>,
    mut input: impl io::BufRead,
    progress: &mut dyn DynNestedProgress,
    refs: &[Ref],
    should_interrupt: &AtomicBool,
    mut out: impl std::io::Write,
    thread_limit: Option<usize>,
    object_hash: gix::hash::Kind,
    format: OutputFormat,
) -> io::Result<()> {
    let options = pack::bundle::write::Options {
        thread_limit,
        index_version: pack::index::Version::V2,
        iteration_mode: pack::data::input::Mode::Verify,
        object_hash,
    };
    let outcome = pack::Bundle::write_to_directory(
        &mut input,
        directory.take().as_deref(),
        progress,
        should_interrupt,
        None::<gix::objs::find::Never>,
        options,
    )
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    if let Some(directory) = refs_directory.take() {
        write_raw_refs(refs, directory)?;
    }

    match format {
        OutputFormat::Human => drop(print(&mut out, outcome, refs)),
        #[cfg(feature = "serde")]
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut out, &JsonOutcome::from_outcome_and_refs(outcome, refs))?;
        }
    }
    Ok(())
}
