use crate::{
    Result, Submodule,
    bstr::{BStr, BString, ByteSlice},
    error::{ResultExt, bail, message, validation},
};

impl Submodule<'_> {
    /// Return a URL suitable for cloning or fetching this submodule, honoring configuration overrides.
    ///
    /// Resolve leading `./` and `../` components from `.gitmodules` against the superproject's default remote URL as Git does:
    /// use the current branch's `branch.<name>.remote`, otherwise the only configured remote, otherwise `origin`.
    /// If its URL is missing, use the superproject itself as the upstream.
    /// Local paths are made absolute relative to the superproject's worktree, or its Git directory if bare,
    /// so the result can be used independently of the caller's current directory.
    /// URLs from configuration overrides are treated as initialized and are not resolved against the remote again.
    /// Other URLs are returned as configured; URL rewrite rules and transport permissions apply when connecting.
    /// An error is returned if traversal exhausts the upstream URL's components or the result cannot be parsed.
    pub fn resolved_url(&self) -> Result<gix_url::Url> {
        let mut url = self.url()?;
        let repo = self.state.repo();
        let root = repo.workdir().unwrap_or_else(|| repo.git_dir());
        let root = gix_path::realpath_opts(root, repo.current_dir(), gix_path::realpath::MAX_SYMLINKS)
            .or_raise(|| message("Could not resolve the superproject's directory"))?;
        let configured_url = url.to_bstring();
        let modules = self.state.modules.config();
        // The modules configuration includes trusted overrides, each retaining its origin metadata.
        // Only relative URLs originating in `.gitmodules` are resolved against the superproject's remote;
        // overrides are treated as initialized URLs, so relative overrides remain local paths.
        let mut from_modules_file = false;
        modules.string_filter_by("submodule", Some(self.name()), "url", |meta| {
            from_modules_file = std::ptr::eq(meta, modules.meta());
            true
        });
        if from_modules_file && (starts_with_dot_slash(&configured_url) || starts_with_dot_dot_slash(&configured_url)) {
            let remote_name = repo
                .head()?
                .referent_name()
                .and_then(|branch| {
                    repo.config.resolved.string_filter_by(
                        "branch",
                        Some(branch.shorten()),
                        "remote",
                        repo.filter_config_section(),
                    )
                })
                .or_else(|| repo.remote_default_name(crate::remote::Direction::Fetch))
                .unwrap_or_else(|| "origin".into());
            let remote_url = repo.config.resolved.string_filter_by(
                "remote",
                Some(remote_name.as_bstr()),
                "url",
                repo.filter_config_section(),
            );
            let own_upstream = remote_url.is_none();
            let fallback;
            let remote_url = match remote_url.as_deref() {
                Some(url) => url.as_bstr(),
                None => {
                    fallback = gix_path::into_bstr(&root)?;
                    fallback.as_ref()
                }
            };
            let resolved = resolve_relative_url(remote_url, &configured_url)?;
            if own_upstream {
                // The fallback is a known local path, including Windows verbatim prefixes that aren't URL syntax.
                url.path = gix_path::into_bstr(gix_path::to_native_path_on_windows(resolved.as_bstr())?)?.into_owned();
            } else {
                url = gix_url::parse(&resolved).or_raise(|| {
                    validation(format!(
                        "The resolved URL of submodule '{}' could not be parsed",
                        self.name()
                    ))
                    .with_input(resolved)
                })?;
            }
        }
        url.canonicalize(&root)
            .or_raise(|| message("Could not make the submodule URL independent of the current directory"))?;
        Ok(url)
    }
}

fn is_dir_sep(byte: u8) -> bool {
    byte == b'/' || cfg!(windows) && byte == b'\\'
}

fn starts_with_dot_slash(url: &[u8]) -> bool {
    url.len() >= 2 && url[0] == b'.' && is_dir_sep(url[1])
}

fn starts_with_dot_dot_slash(url: &[u8]) -> bool {
    url.len() >= 3 && url.starts_with(b"..") && is_dir_sep(url[2])
}

/// Port of Git's `relative_url()` in `remote.c`, operating on bytes to preserve local paths.
/// The caller supplies a relative submodule URL starting with `./` or `../`.
/// Only leading relative components are resolved; the result is not parsed or canonicalized here.
///
/// Examples in the form `(remote_url, url) -> result`:
///
/// * (`ssh://host/org/super.git`, `./sub`) -> `ssh://host/org/super.git/sub`
/// * (`ssh://host/org/super.git`, `../sub`) -> `ssh://host/org/sub`
/// * (`https://host/org/super.git/`, `././sub/`) -> `https://host/org/super.git/sub`
/// * (`git@host:org/super.git`, `../../sub`) -> `git@host:sub` (preserves the scp-style separator)
/// * (`/srv/git/super`, `../sub`) -> `/srv/git/sub`
/// * (`foo/bar`, `../sub`) -> `foo/sub` (local relative paths remain relative)
/// * (`ssh://host/org/super.git`, `./sub/../nested`) -> `ssh://host/org/super.git/sub/../nested`
/// * (`foo`, `../../sub`) -> error (exhausts the relative upstream's components)
/// * (``, `../sub`) -> error (empty upstream)
fn resolve_relative_url(remote_url: &BStr, mut url: &[u8]) -> Result<BString> {
    if remote_url.is_empty() {
        bail!(validation(
            "Cannot resolve a relative submodule URL against an empty remote URL"
        ));
    }
    let mut remote = remote_url.to_vec();
    if remote.last().is_some_and(|byte| is_dir_sep(*byte)) {
        remote.pop();
    }
    let has_drive = cfg!(windows) && gix_path::windows_drive_prefix(remote.as_bstr()).is_some();
    let local = has_drive
        || remote
            .find_byte(b':')
            .is_none_or(|colon| remote.find_byte(b'/').is_some_and(|slash| slash < colon));
    let relative = local && !has_drive && !remote.first().is_some_and(|byte| is_dir_sep(*byte));
    if relative && !starts_with_dot_slash(&remote) && !starts_with_dot_dot_slash(&remote) {
        remote.splice(..0, *b"./");
    }
    let mut colonsep = false;
    loop {
        if starts_with_dot_dot_slash(url) {
            url = &url[3..];
            if let Some(separator) = remote.iter().rposition(|byte| is_dir_sep(*byte)) {
                remote.truncate(separator);
            } else if let Some(colon) = remote.rfind_byte(b':') {
                remote.truncate(colon);
                colonsep = true;
            } else if relative || remote == b"." {
                bail!(validation("Cannot strip one component off the submodule's remote URL").with_input(remote));
            } else {
                remote.clear();
                remote.push(b'.');
            }
        } else if starts_with_dot_slash(url) {
            url = &url[2..];
        } else {
            break;
        }
    }
    remote.push(if colonsep { b':' } else { b'/' });
    remote.extend_from_slice(url);
    if url.ends_with(b"/") {
        remote.pop();
    }
    if starts_with_dot_slash(&remote) {
        remote.drain(..2);
    }
    Ok(remote.into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn virtual_drive_upstreams_remain_local() -> gix_testtools::TestResult {
        for drive in ["1", "ä", "֍", "😀"] {
            for (remote, module, expected) in [
                (
                    format!("{drive}:/repos/super.git"),
                    "../sub",
                    format!("{drive}:/repos/sub"),
                ),
                (
                    format!(r"{drive}:\repos\super.git"),
                    r"..\sub",
                    format!(r"{drive}:\repos/sub"),
                ),
            ] {
                let resolved = resolve_relative_url(remote.as_bytes().as_bstr(), module.as_bytes())?;
                assert_eq!(resolved, expected, "resolution preserves virtual drive prefixes");
                let mut url = gix_url::parse(&resolved)?;
                assert_eq!(
                    url.scheme,
                    gix_url::Scheme::File,
                    "virtual drive URLs use local transport"
                );
                url.canonicalize(std::path::Path::new(r"C:\gix-submodule-test-base"))?;
                assert_eq!(
                    url.path,
                    format!(r"{drive}:\repos\sub"),
                    "resolved virtual drive URLs stay independent of the superproject directory"
                );
            }
        }
        Ok(())
    }
}
