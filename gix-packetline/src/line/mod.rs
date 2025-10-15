use bstr::BStr;

use crate::{decode, BandRef, Channel, ErrorRef, PacketLineRef, TextRef, ERR_PREFIX};

impl<'a> PacketLineRef<'a> {
    /// Return this instance as slice if it's [`Data`][PacketLineRef::Data].
    pub fn as_slice(&self) -> Option<&'a [u8]> {
        match self {
            PacketLineRef::Data(d) => Some(d),
            PacketLineRef::Flush | PacketLineRef::Delimiter | PacketLineRef::ResponseEnd => None,
        }
    }
    /// Return this instance's [`as_slice()`][PacketLineRef::as_slice()] as [`BStr`].
    pub fn as_bstr(&self) -> Option<&'a BStr> {
        self.as_slice().map(Into::into)
    }
    /// Interpret this instance's [`as_slice()`][PacketLineRef::as_slice()] as [`ErrorRef`].
    ///
    /// This works for any data received in an error [channel][crate::Channel].
    ///
    /// Note that this creates an unchecked error using the slice verbatim, which is useful to [serialize it][ErrorRef::write_to()].
    /// See [`check_error()`][PacketLineRef::check_error()] for a version that assures the error information is in the expected format.
    pub fn as_error(&self) -> Option<ErrorRef<'a>> {
        self.as_slice().map(ErrorRef)
    }
    /// Check this instance's [`as_slice()`][PacketLineRef::as_slice()] is a valid [`ErrorRef`] and return it.
    ///
    /// This works for any data received in an error [channel][crate::Channel].
    pub fn check_error(&self) -> Option<ErrorRef<'a>> {
        self.as_slice().and_then(|data| {
            if data.len() >= ERR_PREFIX.len() && &data[..ERR_PREFIX.len()] == ERR_PREFIX {
                Some(ErrorRef(&data[ERR_PREFIX.len()..]))
            } else {
                None
            }
        })
    }
    /// Return this instance as text, with the trailing newline truncated if present.
    pub fn as_text(&self) -> Option<TextRef<'a>> {
        self.as_slice().map(Into::into)
    }

    /// Interpret the data in this [`slice`][PacketLineRef::as_slice()] as [`BandRef`] according to the given `kind` of channel.
    ///
    /// Note that this is only relevant in a side-band channel.
    /// See [`decode_band()`][PacketLineRef::decode_band()] in case `kind` is unknown.
    pub fn as_band(&self, kind: Channel) -> Option<BandRef<'a>> {
        self.as_slice().map(|d| match kind {
            Channel::Data => BandRef::Data(d),
            Channel::Progress => BandRef::Progress(d),
            Channel::Error => BandRef::Error(d),
        })
    }

    /// Decode the band of this [`slice`][PacketLineRef::as_slice()]
    pub fn decode_band(&self) -> Result<BandRef<'a>, decode::band::Error> {
        let d = self.as_slice().ok_or(decode::band::Error::NonDataLine)?;
        Ok(match d[0] {
            1 => BandRef::Data(&d[1..]),
            2 => BandRef::Progress(&d[1..]),
            3 => BandRef::Error(&d[1..]),
            band => return Err(decode::band::Error::InvalidSideBand { band_id: band }),
        })
    }
}

impl<'a> From<&'a [u8]> for TextRef<'a> {
    fn from(d: &'a [u8]) -> Self {
        let d = if d[d.len() - 1] == b'\n' { &d[..d.len() - 1] } else { d };
        TextRef(d)
    }
}

impl<'a> TextRef<'a> {
    /// Return this instance's data.
    pub fn as_slice(&self) -> &'a [u8] {
        self.0
    }
    /// Return this instance's data as [`BStr`].
    pub fn as_bstr(&self) -> &'a BStr {
        self.0.into()
    }
}
