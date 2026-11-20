/// The name of the `git` client in a format suitable for presentation to a `git` server, using `name` as user-defined portion of the value.
pub fn agent(name: impl Into<String>) -> String {
    let mut name = name.into();
    if !name.starts_with("git/") {
        name.insert_str(0, "git/");
    }
    name
}
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
mod with_transport {
    #[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
    use gix_transport::client::async_io::Transport;
    #[cfg(feature = "blocking-client")]
    use gix_transport::client::blocking_io::Transport;

    /// Send a message to indicate the remote side that there is nothing more to expect from us, indicating a graceful shutdown.
    /// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
    #[crate::bisync::bisync]
    pub async fn indicate_end_of_interaction(
        mut transport: impl Transport,
        trace: bool,
    ) -> Result<(), gix_transport::client::Error> {
        // An empty request marks the (early) end of the interaction. Only relevant in stateful transports though.
        if transport.connection_persists_across_multiple_requests() {
            transport
                .request(
                    gix_transport::client::WriteMode::Binary,
                    gix_transport::client::MessageKind::Flush,
                    trace,
                )?
                .into_read()
                .await?;
        }
        Ok(())
    }

    /// A utility to automatically send a flush packet when the instance is dropped, assuring a graceful termination of any
    /// interaction with the server.
    pub struct SendFlushOnDrop<T>
    where
        T: Transport,
    {
        /// The actual transport instance.
        pub inner: T,
        /// If `true`, the packetline used to indicate the end of interaction will be traced using `gix-trace`.
        trace_packetlines: bool,
        /// If `true`, we should not send another flush packet.
        flush_packet_sent: bool,
    }

    impl<T> SendFlushOnDrop<T>
    where
        T: Transport,
    {
        /// Create a new instance with `transport`, while optionally tracing packetlines with `trace_packetlines`.
        pub fn new(transport: T, trace_packetlines: bool) -> Self {
            Self {
                inner: transport,
                trace_packetlines,
                flush_packet_sent: false,
            }
        }

        /// Suppress the automatic flush after an exchange has already terminated the connection.
        ///
        /// Use this for operations such as receive-pack that consume the connection themselves,
        /// including failed uploads for which further protocol writes are invalid.
        pub fn prevent_flush_on_drop(&mut self) {
            self.flush_packet_sent = true;
        }

        /// Useful to explicitly invalidate the connection by sending a flush-packet.
        /// This will happen exactly once, and it is not considered an error to call it multiple times.
        ///
        /// For convenience, this is not consuming, but could be to assure the underlying transport isn't used anymore.
        #[crate::bisync::bisync]
        pub async fn indicate_end_of_interaction(&mut self) -> Result<(), gix_transport::client::Error> {
            if self.flush_packet_sent {
                return Ok(());
            }

            self.flush_packet_sent = true;
            indicate_end_of_interaction(&mut self.inner, self.trace_packetlines).await
        }
    }

    impl<T> Drop for SendFlushOnDrop<T>
    where
        T: Transport,
    {
        fn drop(&mut self) {
            #[cfg(all(feature = "async-client", not(feature = "blocking-client")))]
            {
                // TODO: this should be an async drop once the feature is available.
                //       Right now we block the executor by forcing this communication, but that only
                //       happens if the user didn't actually try to receive a pack, which consumes the
                //       connection in an async context.
                crate::futures_lite::future::block_on(self.indicate_end_of_interaction()).ok();
            }
            #[cfg(feature = "blocking-client")]
            {
                self.indicate_end_of_interaction().ok();
            }
        }
    }
}
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub use with_transport::*;
