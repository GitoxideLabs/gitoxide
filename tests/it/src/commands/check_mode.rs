pub(super) mod function {
    use anyhow::{bail, Context};
    use gix::bstr::ByteSlice;

    use regex::bytes::Regex;
    use std::ffi::{OsStr, OsString};
    use std::io::{BufRead, BufReader, Read};
    use std::process::{Command, Stdio};

    pub fn check_mode() -> anyhow::Result<()> {
        let root = find_root()?;
        let mut any_mismatch = false;

        let mut child = git_on(&root)
            .args(["ls-files", "-sz", "--", "*.sh"])
            .stdout(Stdio::piped())
            .spawn()
            .context("Can't start `git` subprocess to list index")?;

        let stdout = child.stdout.take().expect("should have captured stdout");
        for result in BufReader::new(stdout).split(0) {
            let record = result.context(r"Can't read '\0'-terminated record")?;
            any_mismatch |= check_for_mismatch(&root, &record)?;
        }

        let status = child.wait().context("Failure running `git` subprocess to list index")?;
        if !status.success() {
            bail!("`git` subprocess to list index did not complete successfully");
        }
        if any_mismatch {
            bail!("Mismatch found - scan completed, finding at least one `#!` vs. `+x` mismatch");
        }
        Ok(())
    }

    /// Find the top-level directory of the current repository working tree.
    fn find_root() -> anyhow::Result<OsString> {
        let output = Command::new(gix::path::env::exe_invocation())
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("Can't run `git` to find worktree root")?;

        if !output.status.success() {
            bail!("`git` failed to find worktree root");
        }

        let root = output
            .stdout
            .strip_suffix(b"\n")
            .context("Can't parse worktree root")?
            .to_os_str()?
            .to_owned();
        Ok(root)
    }

    /// Prepare a `git` command, passing `root` as an operand to `-C`.
    ///
    /// This is suitable when `git` gave us the path `root`. Then it should already be in a form
    /// where `git -C` will be able to use it, without alteration, regardless of the platform.
    /// (Otherwise, it may be preferable to set `root` as the `cwd` of the `git` process instead.)
    fn git_on(root: &OsStr) -> Command {
        let mut cmd = Command::new(gix::path::env::exe_invocation());
        cmd.arg("-C").arg(root);
        cmd
    }

    /// On mismatch, report it and return `Some(true)`.
    fn check_for_mismatch(root: &OsStr, record: &[u8]) -> anyhow::Result<bool> {
        static RECORD_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            let pattern = r"(?-u)\A([0-7]+) ([[:xdigit:]]+) [[:digit:]]+\t(.+)\z";
            Regex::new(pattern).expect("regex should be valid")
        });

        let fields = RECORD_REGEX.captures(record).context("Malformed record from `git`")?;
        let mode = fields.get(1).expect("match should get mode").as_bytes();
        let oid = fields
            .get(2)
            .expect("match should get oid")
            .as_bytes()
            .to_os_str()
            .expect("oid field verified as hex digits, should be valid OsStr");
        let path = fields.get(3).expect("match should get path").as_bytes().as_bstr();

        match mode {
            b"100644" if blob_has_shebang(root, oid)? => {
                println!("mode -x but has shebang: {path:?}");
                Ok(true)
            }
            b"100755" if !blob_has_shebang(root, oid)? => {
                println!("mode +x but no shebang: {path:?}");
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn blob_has_shebang(root: &OsStr, oid: &OsStr) -> anyhow::Result<bool> {
        let mut buf = [0u8; 2];

        let mut child = git_on(root)
            .args(["cat-file", "blob"])
            .arg(oid)
            .stdout(Stdio::piped())
            .spawn()
            .context("Can't start `git` subprocess to read blob")?;

        let mut stdout = child.stdout.take().expect("should have captured stdout");
        let count = stdout.read(&mut buf).context("Error reading data from blob")?;
        drop(stdout); // Let the pipe break rather than waiting for the rest of the blob.

        // TODO: Maybe check status? On Unix, it should be 0 or SIGPIPE. Not sure about Windows.
        _ = child.wait().context("Failure running `git` subprocess to read blob")?;

        Ok(&buf[..count] == b"#!")
    }
}
