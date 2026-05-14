use futures_lite::io::AsyncWriteExt;
use gix_transport::client::{
    self,
    async_io::{ExtendedBufRead, Transport, TransportV2Ext},
};

use crate::{Command, fetch::Arguments};

impl Arguments {
    /// Send fetch arguments to the server, and indicate this is the end of negotiations only if `add_done_argument` is present.
    pub async fn send<'a, T: Transport + 'a>(
        &mut self,
        transport: &'a mut T,
        add_done_argument: bool,
    ) -> Result<Box<dyn ExtendedBufRead<'a> + Unpin + 'a>, client::Error> {
        if self.haves.is_empty() {
            assert!(add_done_argument, "If there are no haves, is_done must be true.");
        }
        match self.version {
            gix_transport::Protocol::V0 | gix_transport::Protocol::V1 => {
                let (on_into_read, retained_state) = self.prepare_v1(
                    transport.connection_persists_across_multiple_requests(),
                    add_done_argument,
                )?;
                let mut line_writer = transport.request(
                    client::WriteMode::OneLfTerminatedLinePerWriteCall,
                    on_into_read,
                    self.trace,
                )?;
                let had_args = !self.args.is_empty();
                for arg in self.args.drain(..) {
                    line_writer.write_all(&arg).await?;
                }
                if had_args {
                    line_writer.write_message(client::MessageKind::Flush).await?;
                }
                for line in self.haves.drain(..) {
                    line_writer.write_all(&line).await?;
                }
                if let Some(next_args) = retained_state {
                    self.args = next_args;
                }
                Ok(line_writer.into_read().await?)
            }
            gix_transport::Protocol::V2 => {
                let retained_state = self.args.clone();
                self.args.append(&mut self.haves);
                if add_done_argument {
                    self.args.push("done".into());
                }
                transport
                    .invoke(
                        Command::Fetch.as_str(),
                        self.features.iter().filter(|(_, v)| v.is_some()).cloned(),
                        Some(std::mem::replace(&mut self.args, retained_state).into_iter()),
                        self.trace,
                    )
                    .await
            }
        }
    }
}
