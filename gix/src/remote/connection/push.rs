use std::{io, sync::atomic::AtomicBool};

use gix_transport::client::blocking_io::Transport;

use crate::{
    Progress, Result,
    bstr::ByteSlice,
    error::{ResultExt, bail, message},
    push::{self, Options, Outcome, Rejection, Status, Update},
    remote::Connection,
};

impl<T: Transport> Connection<'_, '_, '_, T> {
    /// Push explicit reference updates after negotiating with receive-pack.
    ///
    /// Updates use fully qualified destinations and optional leases. The advertised old value
    /// always protects each wire update from concurrent changes during transfer. Matching leases
    /// permit forced updates, while unchanged tips succeed even if their lease is stale, like Git.
    /// Branches must point directly to commits. Other non-fast-forward updates and tag replacements
    /// require force or a matching lease. Replacement refs do not affect ancestry or packed objects.
    ///
    /// `progress` reports connection progress; `remote_progress(is_error, bytes)` receives server
    /// diagnostics while a background thread streams a self-contained pack. This enables the
    /// `parallel` feature, as moving a repository handle to the worker requires it.
    ///
    /// This explicit API does not expand configured push refspecs, run local pre-push hooks, or
    /// update local tracking references. The transport's receive-pack server applies its own
    /// hooks and receive policy. Credentials and transport options use the normal connection
    /// configuration. The connection is consumed even on failure and must not be reused.
    pub fn push(
        mut self,
        updates: impl IntoIterator<Item = Update>,
        options: Options<'_>,
        mut progress: impl Progress,
        remote_progress: impl FnMut(bool, &[u8]) -> io::Result<()>,
        should_interrupt: &AtomicBool,
    ) -> Result<Outcome> {
        push::plan::check_interrupt(should_interrupt)?;
        let mut repo = self.remote.repo.clone();
        repo.objects.ignore_replacements = true;
        let shallow = repo
            .shallow_commits()?
            .map(|commits| commits.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut configured_authenticate = self.configured_credentials_for_current_url();
        let authenticate = self.authenticate.as_mut().unwrap_or(&mut configured_authenticate);
        let url = self.transport.inner.to_url();
        if self.transport_options.is_none() {
            self.transport_options = repo
                .transport_options(url.as_ref(), self.remote.name().map(crate::remote::Name::as_bstr))
                .or_raise(|| message!("Failed to configure the transport before connecting to {url:?}"))?;
        }
        if let Some(config) = self.transport_options.as_ref() {
            self.transport
                .inner
                .configure(&**config)
                .or_raise(|| message("Failed to configure the transport layer"))?;
        }
        let handshake = gix_protocol::handshake(
            &mut self.transport.inner,
            gix_transport::Service::ReceivePack,
            authenticate,
            Vec::new(),
            &mut progress,
        )?;
        if handshake.server_protocol_version == gix_transport::Protocol::V2 {
            bail!(message("Receive-pack does not support protocol version 2").unsupported());
        }
        let mut updates = push::plan::updates(
            &repo,
            self.remote,
            updates,
            handshake.refs.as_deref().unwrap_or_default(),
            &shallow,
            should_interrupt,
        )?;
        if options.atomic
            && updates
                .iter()
                .any(|update| matches!(update.status, Status::Rejected(_)))
        {
            for update in &mut updates {
                if matches!(update.status, Status::WouldUpdate { .. }) {
                    update.status = Status::Rejected(Rejection::Atomic);
                }
            }
        }
        let commands: Vec<_> = updates
            .iter()
            .filter(|update| matches!(update.status, Status::WouldUpdate { .. }))
            .map(|outcome| gix_protocol::push::Command {
                name: outcome.update.destination.clone(),
                old_id: outcome.old_id.unwrap_or_else(|| repo.object_hash().null()),
                new_id: outcome.update.new_id.unwrap_or_else(|| repo.object_hash().null()),
            })
            .collect();
        let agent = repo.config.user_agent_tuple().1;
        let request = gix_protocol::push::Request::new(
            commands,
            repo.object_hash(),
            &handshake.capabilities,
            gix_protocol::push::Options {
                atomic: options.atomic,
                push_options: options.push_options,
                agent: agent.as_deref().map(|agent| agent.as_bytes().as_bstr()),
                quiet: options.quiet,
            },
        )?;
        if options.dry_run || request.commands().is_empty() {
            self.transport
                .indicate_end_of_interaction()
                .or_raise(|| message("Could not finish the push advertisement"))?;
            return Ok(Outcome {
                dry_run: options.dry_run,
                handshake,
                updates,
                response: None,
            });
        }
        let tips: Vec<_> = request
            .commands()
            .iter()
            .filter(|command| !command.new_id.is_null())
            .map(|command| command.new_id)
            .collect();
        let pack_shallow = shallow.clone();
        // The receive-pack exchange closes this transport; a second automatic flush is invalid.
        self.transport.prevent_flush_on_drop();
        let response = request.send(
            &mut self.transport.inner,
            move |output| {
                let mut object_ids = Vec::new();
                for entry in gix_traverse::object::BreadthFirst::new(tips, &repo).with_shallow_commits(pack_shallow) {
                    push::plan::check_interrupt(should_interrupt)?;
                    object_ids.push(entry?.id);
                }
                repo.write_pack(object_ids, output, crate::progress::Discard, should_interrupt)?;
                Ok(())
            },
            remote_progress,
            should_interrupt,
            gix_protocol::push::SendOptions {
                server_protocol: handshake.server_protocol_version,
                shallow: &shallow,
                trace_packetlines: self.trace,
            },
        )?;
        if let Some(response) = response.as_ref() {
            for update in &mut updates {
                if let Status::WouldUpdate { forced } = update.status {
                    let report = response
                        .refs
                        .iter()
                        .find(|report| report.name == update.update.destination)
                        .expect("the protocol validates that each command has a report");
                    update.status = match report.error.as_ref().or(response.unpack_error.as_ref()) {
                        Some(error) => Status::Rejected(Rejection::Server(error.clone())),
                        None => Status::Updated { forced },
                    };
                }
            }
        }
        Ok(Outcome {
            dry_run: options.dry_run,
            handshake,
            updates,
            response,
        })
    }
}
