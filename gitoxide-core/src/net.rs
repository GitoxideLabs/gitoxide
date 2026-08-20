use std::str::FromStr;

#[cfg(feature = "async-client")]
use gix::protocol::transport::client::async_io as io_mode;
#[cfg(feature = "blocking-client")]
use gix::protocol::transport::client::blocking_io as io_mode;

#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub enum Protocol {
    V1,
    #[default]
    V2,
}

impl FromStr for Protocol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "1" => Protocol::V1,
            "2" => Protocol::V2,
            _ => return Err(format!("Unsupported protocol version '{s}', choose '1' or '2'")),
        })
    }
}

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
mod impls {
    use gix::protocol::transport;

    use super::Protocol;

    impl From<Protocol> for transport::Protocol {
        fn from(v: Protocol) -> Self {
            match v {
                Protocol::V1 => transport::Protocol::V1,
                Protocol::V2 => transport::Protocol::V2,
            }
        }
    }
}

#[cfg(any(feature = "async-client", feature = "blocking-client"))]
#[gix::protocol::bisync::bisync]
pub async fn connect<Url, E>(
    url: Url,
    options: io_mode::connect::Options,
) -> Result<gix::protocol::SendFlushOnDrop<Box<dyn io_mode::Transport + Send>>, io_mode::connect::Error>
where
    Url: TryInto<gix::url::Url, Error = E>,
    E: std::error::Error + Send + Sync + 'static,
{
    Ok(gix::protocol::SendFlushOnDrop::new(
        io_mode::connect::connect(url, options).await?,
        false,
    ))
}
