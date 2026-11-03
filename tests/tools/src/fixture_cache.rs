use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::Hasher,
    io::Write,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use gix_error::{Error, Result, ResultExt, bail};

use crate::{FixtureState, PostResult};

/// A handle to one entry in a caller-owned cache of generated fixtures.
/// Equivalent requests select the same entry, even through separate handles. Archives never populate it.
pub(super) struct Cache {
    /// Absolute entry directory selected by the identity hash, containing `identity` and `completed` files
    /// alongside `fixture-*` generation-attempt directories, rather than the fixture payload directly.
    /// For example, `/tmp/qa-run-123/fixture-cache/<identity-hash>`, with the payload in a child
    /// directory such as `fixture-AbCd12/`. A successful attempt's random name is recorded in `completed`
    /// and reused by later requests; a new suffix is chosen only when starting a generation attempt.
    directory: PathBuf,
    /// Serialized cache-key inputs, hashed to select `directory` and persisted there in full.
    /// Comparing these bytes before reuse detects hash collisions instead of accepting the wrong fixture.
    identity: Vec<u8>,
}

impl Cache {
    /// Whether either cache environment variable is set, requesting shared caching.
    /// Partial configuration counts as a request so [`Self::from_env()`] can report it as invalid.
    pub(super) fn is_requested_from_env() -> bool {
        env::var_os("GIX_TEST_FIXTURE_CACHE").is_some() || env::var_os("GIX_TEST_FIXTURE_SOURCE_ROOT").is_some()
    }

    pub(super) fn from_env(
        script: &Path,
        args: &[String],
        object_hash: gix_hash::Kind,
        script_identity: u32,
        post_version: Option<u32>,
    ) -> Result<Option<Self>> {
        let Some((directory, source_root)) = Self::paths_from_env()? else {
            return Ok(None);
        };
        let Some(mut identity) = source_identity(script, &source_root, object_hash)? else {
            return Ok(None);
        };
        fs::create_dir_all(&directory).or_error()?;
        let directory = fixture_realpath(&directory)?;
        append_invocation_identity(&mut identity, args, script_identity, post_version);
        let mut hash = DefaultHasher::new();
        hash.write(&identity);
        Ok(Some(Self {
            directory: directory.join(format!("{:016x}", hash.finish())),
            identity,
        }))
    }

    pub(super) fn obtain<T>(
        self,
        make_fixture: impl FnOnce(FixtureState<'_>) -> PostResult<T>,
    ) -> Result<(PathBuf, T)> {
        let _lock = self.lock_entry()?;
        self.create_entry_and_verify_identity()?;
        if let Some(path) = self.read_completed_fixture()? {
            let value = make_fixture(FixtureState::Fresh(&path)).map_err(Error::from_boxed)?;
            return Ok((path, value));
        }

        let path = self.create_generation_directory()?;
        let value = make_fixture(FixtureState::Uninitialized(&path))
            .inspect_err(|_| {
                eprintln!("Retained incomplete generated fixture at {}", path.display());
            })
            .map_err(Error::from_boxed)?;
        self.publish_completed_fixture(&path)?;
        Ok((path, value))
    }

    fn paths_from_env() -> Result<Option<(PathBuf, PathBuf)>> {
        let (directory, source_root) = match (
            env::var_os("GIX_TEST_FIXTURE_CACHE"),
            env::var_os("GIX_TEST_FIXTURE_SOURCE_ROOT"),
        ) {
            (None, None) => return Ok(None),
            (Some(directory), Some(source_root)) => (PathBuf::from(directory), PathBuf::from(source_root)),
            _ => bail!("GIX_TEST_FIXTURE_CACHE and GIX_TEST_FIXTURE_SOURCE_ROOT must be set together"),
        };
        if !directory.is_absolute() || !source_root.is_absolute() {
            bail!("GIX_TEST_FIXTURE_CACHE and GIX_TEST_FIXTURE_SOURCE_ROOT must be absolute paths");
        }
        Ok(Some((directory, source_root)))
    }

    fn lock_entry(&self) -> Result<gix_lock::Marker> {
        fs::create_dir_all(self.directory.parent().expect("cache entries have a parent")).or_error()?;
        gix_lock::Marker::acquire_to_hold_resource(
            &self.directory,
            gix_lock::acquire::Fail::AfterDurationWithBackoff(Duration::from_secs(6 * 60)),
            None,
            0,
        )
        .or_error()
    }

    fn create_entry_and_verify_identity(&self) -> Result<()> {
        fs::create_dir_all(&self.directory).or_error()?;
        let identity_path = self.directory.join("identity");
        match fs::read(&identity_path) {
            Ok(identity) if identity == self.identity => {}
            Ok(_) => bail!("Fixture cache identity mismatch at {}", self.directory.display()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                fs::write(identity_path, &self.identity).or_error()?;
            }
            Err(err) => bail!(err),
        }
        Ok(())
    }

    fn read_completed_fixture(&self) -> Result<Option<PathBuf>> {
        let completed = self.directory.join("completed");
        let name = match fs::read_to_string(&completed) {
            Ok(name) => name,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => bail!(err),
        };
        let name_path = Path::new(&name);
        if !name.starts_with("fixture-")
            || !matches!(name_path.components().next(), Some(Component::Normal(_)))
            || name_path.components().count() != 1
        {
            bail!("Invalid fixture cache completion record at {}", completed.display());
        }
        let path = self.directory.join(name_path);
        if !fs::symlink_metadata(&path).or_error()?.is_dir() {
            bail!("Completed fixture cache is not a directory: {}", path.display());
        }
        Ok(Some(path))
    }

    fn create_generation_directory(&self) -> Result<PathBuf> {
        // Generate at the final path: fixtures may embed it in alternates, remotes or linked worktrees.
        // Keep incomplete attempts for inspection, and never treat directory existence as success.
        Ok(tempfile::Builder::new()
            .prefix("fixture-")
            .tempdir_in(&self.directory)
            .or_error()?
            .keep())
    }

    fn publish_completed_fixture(&self, path: &Path) -> Result<()> {
        let mut publication = tempfile::NamedTempFile::new_in(&self.directory).or_error()?;
        publication
            .write_all(
                path.file_name()
                    .expect("fixture directory has a name")
                    .as_encoded_bytes(),
            )
            .or_error()?;
        publication
            .persist_noclobber(self.directory.join("completed"))
            .or_error()?;
        Ok(())
    }
}

/// Resolve aliases before generation so Git, the shell and consumers use the same path spelling.
fn fixture_realpath(path: &Path) -> Result<PathBuf> {
    let path = gix_path::realpath(path)?;
    #[cfg(windows)]
    {
        // `realpath()` resolves symlinks but leaves DOS short names like RUNNER~1 intact.
        // Expand them, then remove the verbatim prefix that `canonicalize()` introduces.
        let canonical = path.canonicalize().or_error()?;
        let mut components = canonical.components();
        let mut path = match components.next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                std::path::Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
                std::path::Prefix::VerbatimUNC(server, share) => Path::new(r"\\").join(server).join(share),
                _ => return Ok(canonical),
            },
            _ => return Ok(canonical),
        };
        path.extend(components);
        Ok(path)
    }
    #[cfg(not(windows))]
    Ok(path)
}

fn source_identity(script: &Path, source_root: &Path, object_hash: gix_hash::Kind) -> Result<Option<Vec<u8>>> {
    let canonical_source_root = fixture_realpath(source_root)?;
    // Preserve invocation aliases, which can affect BASH_SOURCE, without embedding the snapshot root.
    let invocation_path = script
        .strip_prefix(source_root)
        .or_else(|_| script.strip_prefix(&canonical_source_root))
        .unwrap_or(script)
        .to_owned();
    let cwd = fixture_realpath(&env::current_dir().or_error()?)?;
    let script = fixture_realpath(&cwd.join(script))?;
    let (Ok(crate_path), Ok(script_path)) = (
        cwd.strip_prefix(&canonical_source_root),
        script.strip_prefix(&canonical_source_root),
    ) else {
        // Tests may create independent source repositories outside the opted-in snapshot.
        return Ok(None);
    };

    let mut identity = b"gix-testtools-generated-fixture-v1".to_vec();
    let hash_name = object_hash.to_string();
    for value in [
        crate_path.as_os_str().as_encoded_bytes(),
        script_path.as_os_str().as_encoded_bytes(),
        invocation_path.as_os_str().as_encoded_bytes(),
        hash_name.as_bytes(),
        crate::family_name().as_bytes(),
    ] {
        append_identity_field(&mut identity, value);
    }
    // Keep exact script contents as well as the legacy archive CRC added by `append_invocation_identity()`.
    append_identity_field(&mut identity, &fs::read(script).or_error()?);
    Ok(Some(identity))
}

fn append_invocation_identity(
    identity: &mut Vec<u8>,
    args: &[String],
    script_identity: u32,
    post_version: Option<u32>,
) {
    identity.extend(script_identity.to_le_bytes());
    identity.extend((args.len() as u64).to_le_bytes());
    for arg in args {
        append_identity_field(identity, arg.as_bytes());
    }
    identity.push(u8::from(post_version.is_some()));
    if let Some(version) = post_version {
        identity.extend(version.to_le_bytes());
    }
}

fn append_identity_field(identity: &mut Vec<u8>, value: &[u8]) {
    identity.extend((value.len() as u64).to_le_bytes());
    identity.extend(value);
}
