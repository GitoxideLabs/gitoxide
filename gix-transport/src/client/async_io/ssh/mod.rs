use async_trait::async_trait;

use crate::{
    client::{SetServiceResponse, Transport, TransportWithoutIO},
    Protocol, Service,
};

pub struct NativeSsh;

impl TransportWithoutIO for NativeSsh {
    fn request(
        &mut self,
        write_mode: crate::client::WriteMode,
        on_into_read: crate::client::MessageKind,
        trace: bool,
    ) -> Result<super::RequestWriter<'_>, crate::client::Error> {
        todo!()
    }

    fn to_url(&self) -> std::borrow::Cow<'_, bstr::BStr> {
        todo!()
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        todo!()
    }

    fn configure(
        &mut self,
        config: &dyn std::any::Any,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        todo!()
    }
}

#[async_trait(?Send)]
impl Transport for NativeSsh {
    async fn handshake<'a>(
        &mut self,
        service: Service,
        extra_parameters: &'a [(&'a str, Option<&'a str>)],
    ) -> Result<SetServiceResponse<'_>, crate::client::Error> {
        todo!()
    }
}

pub async fn connect(
    url: gix_url::Url,
    desired_version: Protocol,
    trace: bool,
) -> Result<NativeSsh, crate::client::Error> {
    todo!()
}
