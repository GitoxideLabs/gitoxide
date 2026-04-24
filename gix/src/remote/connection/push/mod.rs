//! High-level push over an already-established [`Connection`].
//!
//! Mirrors [`super::fetch`]. Flow:
//!
//! 1. [`super::super::Connection::prepare_push`] performs the handshake and
//!    ref-advertisement, then returns a [`Prepare`] builder.
//! 2. The builder accepts refspecs and options.
//! 3. [`Prepare::transmit`] runs the revision walk, builds the pack, and
//!    delegates to [`gix_protocol::send_pack`] to deliver it.

mod error;
mod prepare;
mod transmit;

pub use error::Error;
pub use prepare::{Prepare, SignMode};

use gix_transport::client::blocking_io::Transport;

use crate::remote::{Connection, Direction};

impl<'remote, 'repo, T> Connection<'remote, 'repo, T>
where
    T: Transport,
{
    /// Perform a `ReceivePack` handshake with the remote and return a [`Prepare`] builder.
    ///
    /// For V0/V1 connections (the common case for `file://`, `ssh://`, and `git://` push),
    /// the ref advertisement is returned inline during the handshake.  No separate
    /// `ls-refs` round-trip is required for `receive-pack`.
    ///
    /// # Usage
    ///
    /// ```no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let repo = gix::open(".")?;
    /// # let remote = repo.find_remote("origin")?;
    /// let outcome = remote
    ///     .connect(gix::remote::Direction::Push)?
    ///     .prepare_push(gix_features::progress::Discard)?
    ///     .with_refspecs(["refs/heads/main:refs/heads/main"])
    ///     .transmit(gix_features::progress::Discard, &Default::default())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepare_push(
        mut self,
        mut progress: impl gix_features::progress::Progress,
    ) -> Result<Prepare<'remote, 'repo, T>, Error> {
        let _span = gix_trace::coarse!("remote::Connection::prepare_push()");

        // Resolve transport options from repository config if not already set.
        // Convert to owned early so the immutable borrow of `self.transport.inner`
        // is released before the mutable borrow needed by `configure()`.
        let url_owned: crate::bstr::BString = self.transport.inner.to_url().as_ref().into();
        if self.transport_options.is_none() {
            self.transport_options = self
                .remote
                .repo
                .transport_options(
                    crate::bstr::ByteSlice::as_bstr(url_owned.as_slice()),
                    self.remote.name().map(crate::remote::Name::as_bstr),
                )
                .map_err(|err| Error::GatherTransportConfig {
                    source: err,
                    url: url_owned.clone(),
                })?;
        }
        if let Some(config) = self.transport_options.as_ref() {
            self.transport.inner.configure(&**config)?;
        }

        // Build the credentials callback the same way `ref_map_by_ref` does.
        let mut credentials_storage;
        let authenticate: &mut dyn FnMut(gix_credentials::helper::Action) -> gix_credentials::protocol::Result =
            match self.authenticate.as_mut() {
                Some(f) => f,
                None => {
                    let url = self.remote.url(Direction::Push).map_or_else(
                        || gix_url::parse(url_owned.as_ref()).expect("valid URL to be provided by transport"),
                        ToOwned::to_owned,
                    );
                    credentials_storage = self.configured_credentials(url)?;
                    &mut credentials_storage
                }
            };

        // Perform the handshake for `receive-pack`.
        // V0/V1: server advertises refs inline.
        // V2: receive-pack hasn't been extended to V2; we'll always negotiate down.
        let handshake = gix_protocol::handshake(
            &mut self.transport.inner,
            gix_transport::Service::ReceivePack,
            authenticate,
            Vec::new(), // no extra handshake parameters for push
            &mut progress,
        )?;

        self.handshake = Some(handshake);
        Ok(Prepare::new(self))
    }
}
