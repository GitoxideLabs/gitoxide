# gix-reqwest

[![crates.io](https://img.shields.io/crates/v/gix-reqwest.svg)](https://crates.io/crates/gix-reqwest)
[![Documentation](https://docs.rs/gix-reqwest/badge.svg)](https://docs.rs/gix-reqwest)
[![MIT/Apache-2 licensed](https://img.shields.io/crates/l/gix-reqwest.svg)](./LICENSE-APACHE)

A fork of [reqwest](https://github.com/seanmonstar/reqwest), the HTTP client
created by Sean McArthur and its contributors, maintained by the Gitoxide
project. It carries bugfixes and patches needed for Gitoxide to work correctly
until they are available upstream. The original MIT and Apache-2.0 licenses
and copyright notices are preserved.

`gix-reqwest` has its own release versions and Rust crate name, `gix_reqwest`.
Its `Client`, `ClientBuilder`, `Request`, and other types are distinct from
upstream `reqwest` types. Use this crate's types when configuring a consumer
that depends on this fork. Feature names and the experimental
`reqwest_unstable` configuration flag retain their upstream spelling.

The fork starts from upstream commit
[`6b99ec2`](https://github.com/seanmonstar/reqwest/commit/6b99ec2a6a4fa9b601337fff9979541bb27050d2).
`Reqwest-Upstream` commit fields record complete upstream integration
checkpoints; `Reqwest-Patch` identifies separately imported patches and
`Reqwest-PR` their known source PRs. Local provenance precedes the verbatim
original message; inspect the full commit body instead of Git's trailer parser.
Gitoxide can return to upstream `reqwest` once a release includes the required
fixes and passes the fork's and transport's regression tests.

Maintainers: follow the [reintegration skill](https://github.com/GitoxideLabs/gitoxide/blob/main/.agents/skills/reintegrate-reqwest/SKILL.md)
when importing upstream changes. Commit trailers are the source of truth for
the latest integrated revision, including after branch merges.

The upstream client provides:

- Async and blocking `Client`s
- Plain bodies, JSON, urlencoded, multipart
- Customizable redirect policy
- HTTP Proxies
- HTTPS via rustls (or optionally, system-native TLS)
- Cookie Store
- WASM


## Example

This asynchronous example uses [Tokio](https://tokio.rs) and enables some
optional features, so your `Cargo.toml` could look like this:

```toml
[dependencies]
gix-reqwest = { version = "0.1", features = ["json"] }
tokio = { version = "1", features = ["full"] }
```

And then the code:

```rust,no_run
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp = gix_reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    println!("{resp:#?}");
    Ok(())
}
```

## Commercial Support

For advice and support from the upstream author, see reqwest's
[commercial support][sponsor]. Report issues with this fork to
[Gitoxide](https://github.com/GitoxideLabs/gitoxide/issues).

## Requirements

By default, `gix-reqwest` uses [rustls](https://github.com/rustls/rustls), but when the `native-tls` feature is enabled
it will use the operating system TLS framework if available, meaning Windows and macOS.
On Linux, it will use the available OpenSSL (see https://docs.rs/openssl for supported versions and more details)
or fail to build if not found. Alternatively you can enable the `native-tls-vendored` feature to compile a copy of OpenSSL.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

## Sponsors

Support upstream reqwest by becoming a [sponsor][].

[sponsor]: https://seanmonstar.com/sponsor
