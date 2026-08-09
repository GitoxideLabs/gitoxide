/// A scheme or protocol for use in a [`Url`][crate::Url].
///
/// It defines how to talk to a given repository.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Scheme {
    /// A local resource that is accessible on the current host.
    File,
    /// A git daemon, like `File` over TCP/IP.
    Git,
    /// Launch `git-upload-pack` through an `ssh` tunnel.
    Ssh,
    /// Use the HTTP protocol to talk to git servers.
    Http,
    /// Use the HTTPS protocol to talk to git servers.
    Https,
    /// Any other protocol or transport that isn't known at compile time.
    ///
    /// It's used to support plug-in transports, and carries the helper name of locations in the
    /// `<helper>::<address>` form of `gitremote-helpers(7)`. Note that such a name may be one of the
    /// built-in ones above, as `ssh::address` names the `git-remote-ssh` program rather than the
    /// built-in SSH transport.
    Ext(String),
}

impl<'a> From<&'a str> for Scheme {
    fn from(value: &'a str) -> Self {
        match value {
            // "ssh+git" and "git+ssh" are legacy, but Git still allows them and so should we
            "ssh" | "ssh+git" | "git+ssh" => Scheme::Ssh,
            "file" => Scheme::File,
            "git" => Scheme::Git,
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            unknown => Scheme::Ext(unknown.into()),
        }
    }
}

impl Scheme {
    /// Return the canonical textual name of this scheme.
    ///
    /// Legacy `ssh+git` and `git+ssh` inputs are represented as [`Scheme::Ssh`] and therefore return `ssh`.
    pub fn as_str(&self) -> &str {
        use Scheme::*;
        match self {
            File => "file",
            Git => "git",
            Ssh => "ssh",
            Http => "http",
            Https => "https",
            Ext(name) => name.as_str(),
        }
    }

    /// Return the default port for this scheme, or `None` if it is not known.
    pub fn default_port(&self) -> Option<u16> {
        match self {
            Scheme::Http => Some(80),
            Scheme::Https => Some(443),
            Scheme::Ssh => Some(22),
            Scheme::Git => Some(9418),
            _ => None,
        }
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
