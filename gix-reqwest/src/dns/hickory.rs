//! DNS resolution via the [hickory-resolver](https://github.com/hickory-dns/hickory-dns) crate

use hickory_resolver::{
    config::{LookupIpStrategy, NameServerConfig, ResolverConfig, GOOGLE},
    net::{runtime::TokioRuntimeProvider, NetError},
    TokioResolver,
};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, OnceLock};

use super::{Addrs, Name, Resolve, Resolving};

/// Wrapper around an `AsyncResolver`, which implements the `Resolve` trait.
#[derive(Debug, Default, Clone)]
pub(crate) struct HickoryDnsResolver {
    /// Since we might not have been called in the context of a
    /// Tokio Runtime in initialization, so we must delay the actual
    /// construction of the resolver.
    state: Arc<OnceLock<TokioResolver>>,
}

struct SocketAddrs {
    iter: std::vec::IntoIter<IpAddr>,
}

impl Resolve for HickoryDnsResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.clone();
        Box::pin(async move {
            let state = &resolver.state;
            let resolver = state
                .get()
                .map_or_else(|| new_resolver().map(|new| state.get_or_init(|| new)), Ok)?;

            let lookup = resolver.lookup_ip(name.as_str()).await?;
            let addrs: Addrs = Box::new(SocketAddrs {
                iter: lookup.iter().collect::<Vec<_>>().into_iter(),
            });
            Ok(addrs)
        })
    }
}

impl Iterator for SocketAddrs {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ip_addr| SocketAddr::new(ip_addr, 0))
    }
}

/// Interleave IPv4 and IPv6 addresses so the first two nameservers
/// (the default `num_concurrent_reqs`) cover both families.
///
/// `ResolverConfig::udp_and_tcp(&GOOGLE)` lists both IPv4 addresses first.
/// On an IPv6-only host those queries time out (~5s) before IPv6 Google
/// DNS is tried.
fn interleave_ip_families(ips: &[IpAddr]) -> Vec<IpAddr> {
    let mut v4 = ips.iter().copied().filter(IpAddr::is_ipv4);
    let mut v6 = ips.iter().copied().filter(IpAddr::is_ipv6);
    let mut out = Vec::with_capacity(ips.len());
    loop {
        match (v4.next(), v6.next()) {
            (Some(a), Some(b)) => {
                out.push(a);
                out.push(b);
            }
            (Some(a), None) => {
                out.push(a);
                out.extend(v4);
                break;
            }
            (None, Some(b)) => {
                out.push(b);
                out.extend(v6);
                break;
            }
            (None, None) => break,
        }
    }
    out
}

fn google_fallback_config() -> ResolverConfig {
    let name_servers = interleave_ip_families(GOOGLE.ips)
        .into_iter()
        .map(NameServerConfig::udp_and_tcp)
        .collect();
    ResolverConfig::from_parts(None, Vec::new(), name_servers)
}

/// Create a new resolver with the default configuration,
/// which reads from `/etc/resolve.conf`. If reading `/etc/resolv.conf` fails,
/// it fallbacks to Google Public DNS, with IPv4 and IPv6 nameservers
/// interleaved so a single-family host does not wait on the other family.
/// The options are overridden to look up for both IPv4 and IPv6 addresses
/// to work with "happy eyeballs" algorithm.
fn new_resolver() -> Result<TokioResolver, NetError> {
    let mut builder = TokioResolver::builder_tokio().unwrap_or_else(|err| {
        log::debug!(
            "hickory-dns: failed to load system DNS configuration; falling back to Google DNS: {:?}",
            err
        );
        TokioResolver::builder_with_config(
            google_fallback_config(),
            TokioRuntimeProvider::default(),
        )
    });
    builder.options_mut().ip_strategy = LookupIpStrategy::Ipv6AndIpv4;
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_fallback_nameservers_interleave_families() {
        let ips = interleave_ip_families(GOOGLE.ips);
        assert_eq!(ips.len(), GOOGLE.ips.len());
        assert!(ips[0].is_ipv4(), "first nameserver should stay IPv4");
        assert!(ips[1].is_ipv6(), "second nameserver should be IPv6");
        let mut expected = GOOGLE.ips.to_vec();
        let mut actual = ips.clone();
        expected.sort();
        actual.sort();
        assert_eq!(actual, expected);
    }
}
