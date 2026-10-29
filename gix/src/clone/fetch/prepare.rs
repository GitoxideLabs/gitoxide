use std::sync::atomic::AtomicBool;

use crate::{
    Remote, Repository, Result,
    bstr::BString,
    clone::{ConfigureConnectionFn, Transport},
    error::{ResultExt, message},
    remote::{self, fetch},
};

pub(super) enum Prepare {
    Connection(fetch::PrepareDetached<'static, Box<dyn Transport + Send>>),
    #[cfg(feature = "blocking-network-client")]
    Local(Box<fetch::PrepareLocal>),
}

impl Prepare {
    #[gix_protocol::bisync::bisync]
    pub(super) async fn new(
        remote: &Remote<'_>,
        configure: &mut Option<ConfigureConnectionFn>,
        options: remote::ref_map::Options,
        shallow: &fetch::Shallow,
        progress: impl crate::Progress,
    ) -> Result<Self> {
        #[cfg(feature = "blocking-network-client")]
        if configure.is_none()
            && options.handshake_parameters.is_empty()
            && matches!(shallow, fetch::Shallow::NoChange | fetch::Shallow::DepthAtRemote(_))
            && let Some(prepare) = fetch::PrepareLocal::new(remote, options.clone())?
        {
            return Ok(Self::Local(Box::new(prepare)));
        }
        #[cfg(feature = "async-network-client")]
        let _ = shallow;
        let mut connection = remote.connect(remote::Direction::Fetch).await?;
        if let Some(configure) = configure {
            configure(&mut connection)
                .or_raise(|| message("Custom configuration of connection to use when cloning failed"))?;
        }
        Ok(Self::Connection(
            connection
                .into_detached()
                .prepare_fetch(remote.repo, progress, options)
                .await?,
        ))
    }

    pub(super) fn ref_map(&self) -> &fetch::RefMap {
        match self {
            Self::Connection(prepare) => prepare.ref_map(),
            #[cfg(feature = "blocking-network-client")]
            Self::Local(prepare) => &prepare.ref_map,
        }
    }

    #[gix_protocol::bisync::bisync]
    pub(super) async fn receive<P>(
        self,
        repo: &Repository,
        progress: P,
        reflog_message: BString,
        shallow: fetch::Shallow,
        interrupt: &AtomicBool,
    ) -> Result<fetch::Outcome>
    where
        P: crate::NestedProgress,
        P::SubProgress: 'static,
    {
        let reflog_message = fetch::RefLogMessage::Override {
            message: reflog_message,
        };
        match self {
            Self::Connection(prepare) => {
                prepare
                    .with_write_packed_refs_only(true)
                    .with_reflog_message(reflog_message)
                    .with_shallow(shallow)
                    .receive(repo, progress, interrupt)
                    .await
            }
            #[cfg(feature = "blocking-network-client")]
            Self::Local(prepare) => prepare.receive(
                repo,
                fetch::Options {
                    write_packed_refs_only: true,
                    reflog_message: Some(reflog_message),
                    shallow,
                    ..Default::default()
                },
                progress,
                interrupt,
            ),
        }
    }
}

#[gix_protocol::bisync::bisync]
pub(super) async fn ref_map(
    remote: &Remote<'_>,
    configure: &mut Option<ConfigureConnectionFn>,
    progress: impl crate::Progress,
    options: remote::ref_map::Options,
) -> Result<fetch::RefMap> {
    #[cfg(feature = "blocking-network-client")]
    if configure.is_none() && options.handshake_parameters.is_empty() {
        return remote.ref_map(progress, options).map(|(ref_map, _)| ref_map);
    }
    let mut connection = remote.connect(remote::Direction::Fetch).await?;
    if let Some(configure) = configure {
        configure(&mut connection)
            .or_raise(|| message("Custom configuration of connection to use when cloning failed"))?;
    }
    connection.ref_map(progress, options).await.map(|(ref_map, _)| ref_map)
}
