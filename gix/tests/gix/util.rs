#![allow(clippy::result_large_err)]
use gix::{Repository, ThreadSafeRepository, open};
pub use gix_testtools::Result;
use gix_testtools::tempfile;
use std::collections::HashMap;

static SHA1_TO_SHA256_HASHES: std::sync::LazyLock<HashMap<&str, &str>> = std::sync::LazyLock::new(|| {
    [
        (
            "04fea06420ca60892f73becee3614f6d023a4b7f",
            "8df3dab4ddfa6eb2a34065cda27d95af2709d4d2658e1b5fbd145822acf42b28",
        ),
        (
            "05dc291f5376cde200316cb0b74b00cfebc79ea4",
            "56ad633ef5fcc8698612ab68dddbba7a03cd2c57e5c12492f6eecc32e899d775",
        ),
        (
            "0f35190769db39bc70f60b6fbec9156370ce2f83",
            "d0b9b041a563042e3bae9499e6f0188a69eb4edf454eeb9c839a01ec23d6c4b5",
        ),
        (
            "134385f6d781b7e97062102c6a483440bfda2a03",
            "5c4c31e0551f0d1fb410b7b9366604b050ea3388b96885063f10ba4c3e2dedd0",
        ),
        (
            "1a010b1c0f081b2e8901d55307a15c29ff30af0e",
            "6040432db6355a4a4e6f31c83c3c207bfec72357767f4b42e58ac1086be19851",
        ),
        (
            "21d3ba9a26b790a4858d67754ae05d04dfce4d0c",
            "95997c02e30a106c5413e7a68e7758c6b3c70e951f7471ee48d75c06edc7d234",
        ),
        (
            "27e71576a6335294aa6073ab767f8b36bdba81d0",
            "c2eec0d4d46a9d91b6c306fa0a82c993cb244b38fb63696a93f29145ee287684",
        ),
        (
            "288e509293165cb5630d08f4185bdf2445bf6170",
            "10d977a4ec5ad49aff689ea886dd430f5cf2f618811521fa0df2ccb8c69811ae",
        ),
        (
            "2d9d136fb0765f2e24c44a0f91984318d580d03b",
            "1e485b4edcc2040ffdc450396bf67232498ada3abb6c60fc1e35966538b9144f",
        ),
        (
            "30887839de28edf7ab66c860e5c58b4d445f6b12",
            "731565bc9052421d78a23136b86c0aa5c0eea176b004dd14374342c99f96c19f",
        ),
        (
            "317e9677c3bcffd006f9fc84bbb0a54ef1676197",
            "1f5807555942aa1bf20804aec2ac2b57ee28543d4c885f6bbc1f574798e6be22",
        ),
        (
            "3189cd3cb0af8586c39a838aa3e54fd72a872a41",
            "735ec3eb1e74b0815da6d8aeca80ffbffdca25a2b624cc54d5d34caca9bc4dec",
        ),
        (
            "3a774843723a713a8d361b4d4d98ad4092ef05bd",
            "f94ec43f5a88d139270047a2517ca02b9e73c79d5da45ede9e370e14f7eae720",
        ),
        (
            "362cb5539acbd3c8ca355471f97c6a68d3db0da7",
            "5f0ae0c252472dba8c416420b90ce2aead95561489c1f3d46cc1f8c201a8a7e4",
        ),
        (
            "4b825dc642cb6eb9a060e54bf8d69288fbee4904",
            "6ef19b41225c5369f1c104d45d8d85efa9b057b53b14b4b9b939dd74decc5321",
        ),
        (
            "4c3f4cce493d7beb45012e478021b5f65295e5a3",
            "2c309d047b92197ef711ba55ab652c42d36750d6571a3e024a7325e324be3033",
        ),
        (
            "82024b2ef7858273337471cbd1ca1cedbdfd5616",
            "125ce6c0ed8fe2d20ba96bb2dd9c15a9ef63fcecdee79728f171dc73881aabdd",
        ),
        (
            "95d09f2b10159347eece71399a7e2e907ea3df4f",
            "fee53a18d32820613c0527aa79be5cb30173c823a9b448fa4817767cc84c6f03",
        ),
        (
            "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7",
            "bbaf9640a7404a15394dae2606c5090cb44a722be2167d9d78485779aaf4e065",
        ),
        (
            "a047f8183ba2bb7eb00ef89e60050c5fde740483",
            "6753719202da03ce58e0631cf2f1fd2726c0085a4997cacbaf420e758113cc78",
        ),
        (
            "a9128c283485202893f5af379dd9beccb6e79486",
            "42ff94ad066877cd43a3633b6834e2541f987271d4a6acc048e8501a66e3a5bd",
        ),
        (
            "b51277f2b2ea77676dd6fa877b5eb5ba2f7094d9",
            "9e0929adee3bcd10a6f37c45101712d819f951102707b70baee48d4868d45b0d",
        ),
        (
            "b5152869aedeb21e55696bb81de71ea1bb880c85",
            "a5d87b4776ac59907b8a994b23c0ae71cc8bfa3673737e4baf3bb502915300c6",
        ),
        (
            "bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac",
            "3e87837a4730030fb4a6bf72d121d4e6305df46e7fd93d1f4f3f1b22845112c7",
        ),
        (
            "ce013625030ba8dba906f756967f9e9ca394464a",
            "2cf8d83d9ee29543b34a87727421fdecb7e3f3a183d337639025de576db9ebb4",
        ),
        (
            "cfa5e6f7872c2f4fed7bd8c3f2732a37536d6912",
            "864c497e9bbd16208a87f7595aff2caa52d96e73436fc008810516ad1d84c29c",
        ),
        (
            "d07c527cf14e524a8494ce6d5d08e28079f5c6ea",
            "b07ba512b235bfe2b9b143ac1bd891f82efdaa565e711c98f5e592e6dadaaf89",
        ),
        (
            "d8523dfd5a7aa16562fa1c3e1d3b4a4494f97876",
            "07dee378fa15974a711e2353b3a0046b5a0fdeb8ec460ec837e5a868e51baa25",
        ),
        (
            "d95f3ad14dee633a758d2e331151e950dd13e4ed",
            "7c490ebf9db90b84753749c721ef2bedfeb85c1da94160f2619df1249c64bdda",
        ),
        (
            "de303ef102bd5705a40a0c42ae2972eb1a668455",
            "d60b1da3fb52545d0a8e70f970ffdec226ab0fc4ec870148058ab1c621330db0",
        ),
        (
            "dfd0954dabef3b64f458321ef15571cc1a46d552",
            "94c0c58a38f244279fcfc39f909bbb898eb0cecb754965d441fe144059e7207a",
        ),
        (
            "e046f3e51d955840619fc7d01fbd9a469663de22",
            "edd23e5e4014be6162b5ae5d10ee2cf709dfbcdb9227f3a3dce6e221927bf2e0",
        ),
        (
            "e1412f169e0812eb260601bdab3854ca0f1a7b33",
            "0e0844f01f6ee495e23f7c5ca0431205e4b547347905df2e4dbd3d6812cb79da",
        ),
        (
            "e7c7273539cfc1a52802fa9d61aa578f6ccebcb4",
            "10d26eeb1167a0cefa12fa0738d36eb4e55adb43e8837dfa3d38e7f8b019140e",
        ),
        (
            "edc8cc8a25e64e73aacea469fc765564dd2c3f65",
            "c23b49bd2cee82a036e7359dc45101f6c1ba5fb06db2818c646d764c24a2cf05",
        ),
        (
            "f99771fe6a1b535783af3163eba95a927aae21d5",
            "e36613f63a483f296d306a8c41dc6ad8ecb2f178ab8d0e9c82a130917ae53e65",
        ),
        (
            "fafd9d08a839d99db60b222cd58e2e0bfaf1f7b2",
            "7d8d5a719510afb480e790ecab4c2de8d0aaca041cb2b4b7e7ceb412e77d1cb7",
        ),
    ]
    .into()
});

/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
///
/// This takes `GIX_TEST_FIXTURE_HASH` into account, so it maps SHA-1 hashes to their
/// corresponding SHA-256 hashes.
pub fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
    match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex"),
        gix_hash::Kind::Sha256 => gix_hash::ObjectId::from_hex(
            SHA1_TO_SHA256_HASHES
                .get(hex)
                .unwrap_or_else(|| panic!("SHA-1 {hex} wasn't mapped to SHA-256 yet"))
                .as_bytes(),
        )
        .expect("64 bytes hex"),
        _ => unimplemented!(),
    }
}

/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
///
/// This does *not* take `GIX_TEST_FIXTURE_HASH` into account, so it does not map SHA-1 hashes to
/// SHA-256 hashes. Use this for tests that don't need a mapping.
pub fn hex_to_id_sha1_only(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

pub fn freeze_time() -> gix_testtools::Env<'static> {
    let frozen_time = "42 +0030";
    gix_testtools::Env::new()
        .unset("GIT_AUTHOR_NAME")
        .unset("GIT_AUTHOR_EMAIL")
        .set("GIT_AUTHOR_DATE", frozen_time)
        .unset("GIT_COMMITTER_NAME")
        .unset("GIT_COMMITTER_EMAIL")
        .set("GIT_COMMITTER_DATE", frozen_time)
}
pub fn repo(name: &str) -> Result<ThreadSafeRepository> {
    let repo_path = gix_testtools::scripted_fixture_read_only(name)?;
    Ok(ThreadSafeRepository::open_opts(repo_path, restricted())?)
}

pub fn repo_opts(name: &str, opts: open::Options) -> std::result::Result<ThreadSafeRepository, open::Error> {
    let repo_path = gix_testtools::scripted_fixture_read_only(name).unwrap();
    ThreadSafeRepository::open_opts(repo_path, opts)
}

pub fn named_repo(name: &str) -> Result<Repository> {
    let repo_path = gix_testtools::scripted_fixture_read_only(name)?;
    Ok(ThreadSafeRepository::open_opts(repo_path, restricted())?.to_thread_local())
}

pub fn named_subrepo_opts(
    fixture: &str,
    name: &str,
    opts: open::Options,
) -> std::result::Result<Repository, gix::open::Error> {
    let repo_path = gix_testtools::scripted_fixture_read_only(fixture)
        .map_err(|err| gix::open::Error::Io(std::io::Error::other(err)))?
        .join(name);
    Ok(ThreadSafeRepository::open_opts(repo_path, opts)?.to_thread_local())
}

pub fn restricted() -> open::Options {
    open::Options::isolated().config_overrides(["user.name=gitoxide", "user.email=gitoxide@localhost"])
}

pub fn restricted_and_git() -> open::Options {
    let mut opts = restricted();
    opts.permissions.env.git_prefix = gix_sec::Permission::Allow;
    opts.permissions.env.identity = gix_sec::Permission::Allow;
    opts
}

pub fn repo_rw(name: &str) -> Result<(Repository, tempfile::TempDir)> {
    repo_rw_opts(name, restricted())
}

pub fn repo_rw_opts(name: &str, opts: gix::open::Options) -> Result<(Repository, tempfile::TempDir)> {
    let repo_path = gix_testtools::scripted_fixture_writable(name)?;
    Ok((
        ThreadSafeRepository::discover_opts(
            repo_path.path(),
            Default::default(),
            gix_sec::trust::Mapping {
                full: opts.clone(),
                reduced: opts,
            },
        )?
        .to_thread_local(),
        repo_path,
    ))
}

pub fn basic_repo() -> Result<Repository> {
    repo("make_basic_repo.sh").map(|r| r.to_thread_local())
}

pub fn basic_rw_repo() -> Result<(Repository, tempfile::TempDir)> {
    repo_rw("make_basic_repo.sh")
}
