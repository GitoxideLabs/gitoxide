use bstr::BStr;

use crate::protocol::context::Error;

mod write {
    use bstr::{BStr, BString};

    use crate::protocol::{Context, ContextOptions, context::serde::validate};

    impl Context {
        /// Write ourselves to `out` such that [`from_bytes()`][Self::from_bytes()] can decode it losslessly.
        pub fn write_to(&self, mut out: impl std::io::Write) -> std::io::Result<()> {
            use bstr::ByteSlice;
            fn write_key(out: &mut impl std::io::Write, key: &str, value: &BStr) -> std::io::Result<()> {
                out.write_all(key.as_bytes())?;
                out.write_all(b"=")?;
                out.write_all(value)?;
                out.write_all(b"\n")
            }
            let Context {
                options: ContextOptions { protect_protocol },
                protocol,
                host,
                path,
                username,
                password,
                oauth_refresh_token,
                password_expiry_utc,
                url,
                // We only decode quit and interpret it, but won't get to pass it on as it means to stop the
                // credential helper invocation chain.
                quit: _,
            } = self;
            for (key, value) in [("url", url), ("path", path)] {
                if let Some(value) = value {
                    validate(key, value.as_slice().into(), *protect_protocol).map_err(std::io::Error::other)?;
                    write_key(&mut out, key, value.as_ref()).ok();
                }
            }
            for (key, value) in [
                ("protocol", protocol),
                ("host", host),
                ("username", username),
                ("password", password),
                ("oauth_refresh_token", oauth_refresh_token),
            ] {
                if let Some(value) = value {
                    validate(key, value.as_str().into(), *protect_protocol).map_err(std::io::Error::other)?;
                    write_key(&mut out, key, value.as_bytes().as_bstr()).ok();
                }
            }
            if let Some(value) = password_expiry_utc {
                let key = "password_expiry_utc";
                let value = value.to_string();
                validate(key, value.as_str().into(), *protect_protocol).map_err(std::io::Error::other)?;
                write_key(&mut out, key, value.as_bytes().as_bstr()).ok();
            }
            Ok(())
        }

        /// Like [`write_to()`][Self::write_to()], but writes infallibly into memory.
        pub fn to_bstring(&self) -> BString {
            let mut buf = Vec::<u8>::new();
            self.write_to(&mut buf).expect("infallible");
            buf.into()
        }
    }
}

///
pub mod decode {
    use bstr::{BString, ByteSlice};

    use crate::protocol::{Context, ContextOptions, context, context::serde::validate};

    /// The error returned by [`from_bytes()`][Context::from_bytes()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        IllformedUtf8InValue { key: String, value: BString },
        Encoding(context::Error),
        Syntax { line: BString },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::IllformedUtf8InValue { key, value } => {
                    write!(f, "Illformed UTF-8 in value of key {key:?}: {value:?}")
                }
                Error::Encoding(err) => std::fmt::Display::fmt(err, f),
                Error::Syntax { line } => write!(f, "Invalid format in line {line:?}, expecting key=value"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Encoding(err) => err.source(),
                Error::IllformedUtf8InValue { .. } | Error::Syntax { .. } => None,
            }
        }
    }

    impl From<context::Error> for Error {
        fn from(err: context::Error) -> Self {
            Error::Encoding(err)
        }
    }

    impl Context {
        /// Decode ourselves from `input` which is the format written by [`write_to()`][Self::write_to()].
        /// `options` control what to support during deserialization.
        pub fn from_bytes(input: &[u8], options: ContextOptions) -> Result<Self, Error> {
            let mut ctx = Context {
                options,
                ..Context::default()
            };
            let Context {
                options: _,
                protocol,
                host,
                path,
                username,
                password,
                oauth_refresh_token,
                password_expiry_utc,
                url,
                quit,
            } = &mut ctx;
            for res in input.lines().take_while(|line| !line.is_empty()).map(|line| {
                let mut it = line.splitn(2, |b| *b == b'=');
                match (
                    it.next().and_then(|k| k.to_str().ok()),
                    it.next().map(ByteSlice::as_bstr),
                ) {
                    (Some(key), Some(value)) => validate(key, value, options.protect_protocol)
                        .map(|_| (key, value.to_owned()))
                        .map_err(Into::into),
                    _ => Err(Error::Syntax { line: line.into() }),
                }
            }) {
                let (key, value) = res?;
                match key {
                    "protocol" | "host" | "username" | "password" | "oauth_refresh_token" => {
                        if !value.is_utf8() {
                            return Err(Error::IllformedUtf8InValue { key: key.into(), value });
                        }
                        let value = value.to_string();
                        *match key {
                            "protocol" => &mut *protocol,
                            "host" => host,
                            "username" => username,
                            "password" => password,
                            "oauth_refresh_token" => oauth_refresh_token,
                            _ => unreachable!("checked field names in match above"),
                        } = Some(value);
                    }
                    "password_expiry_utc" => {
                        *password_expiry_utc = value.to_str().ok().and_then(|value| value.parse().ok());
                    }
                    "url" => *url = Some(value),
                    "path" => *path = Some(value),
                    "quit" => {
                        *quit = gix_config_value::Boolean::try_from(value.as_bstr())
                            .ok()
                            .map(Into::into);
                    }
                    _ => {}
                }
            }
            Ok(ctx)
        }
    }
}

fn validate(key: &str, value: &BStr, protect_protocol: bool) -> Result<(), Error> {
    if key.contains('\0')
        || key.contains('\n')
        || key.contains('\r')
        || value.contains(&0)
        || value.contains(&b'\n')
        || (protect_protocol && value.contains(&b'\r'))
    {
        return Err(Error::Encoding {
            key: key.to_owned(),
            value: value.to_owned(),
        });
    }
    Ok(())
}
