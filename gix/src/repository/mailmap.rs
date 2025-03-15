use crate::config::tree::Mailmap;
use crate::Id;

impl crate::Repository {
    // TODO: tests
    /// Similar to [`open_mailmap_into()`][crate::Repository::open_mailmap_into()], but ignores all errors and returns at worst
    /// an empty mailmap, e.g. if there is no mailmap or if there were errors loading them.
    ///
    /// This represents typical usage within git, which also works with what's there without considering a populated mailmap
    /// a reason to abort an operation, considering it optional.
    pub fn open_mailmap(&self) -> gix_mailmap::Snapshot {
        let mut out = gix_mailmap::Snapshot::default();
        self.open_mailmap_into(&mut out).ok();
        out
    }

    // TODO: tests
    /// Try to merge mailmaps from the following locations into `target`:
    ///
    /// - read the `.mailmap` file without following symlinks from the working tree, if present
    /// - OR read `HEAD:.mailmap` if this repository is bare (i.e. has no working tree), if the `mailmap.blob` is not set.
    /// - read the mailmap as configured in `mailmap.blob`, if set.
    /// - read the file as configured by `mailmap.file`, following symlinks, if set.
    ///
    /// Only the first error will be reported, and as many source mailmaps will be merged into `target` as possible.
    /// Parsing errors will be ignored.
    pub fn open_mailmap_into(&self, target: &mut gix_mailmap::Snapshot) -> Result<(), crate::mailmap::load::Error> {
        let mut err = None::<crate::mailmap::load::Error>;
        let mut buf = Vec::new();
        let mut blob_id = self.config.resolved.string(Mailmap::BLOB).and_then(|spec| {
            self.rev_parse_single(spec.as_ref())
                .map_err(|e| err.get_or_insert(e.into()))
                .map(Id::detach)
                .ok()
        });
        match self.workdir() {
            None => {
                blob_id = blob_id.or_else(|| {
                    self.head().ok().and_then(|mut head| {
                        let commit = head.peel_to_commit_in_place().ok()?;
                        let tree = commit.tree().ok()?;
                        tree.find_entry(".mailmap").map(|e| e.object_id())
                    })
                });
            }
            Some(root) => {
                if let Ok(mut file) = gix_features::fs::open_options_no_follow()
                    .read(true)
                    .open(root.join(".mailmap"))
                    .map_err(|e| {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            err.get_or_insert(e.into());
                        }
                    })
                {
                    buf.clear();
                    std::io::copy(&mut file, &mut buf)
                        .map_err(|e| err.get_or_insert(e.into()))
                        .ok();
                    target.merge(gix_mailmap::parse_ignore_errors(&buf));
                }
            }
        }

        if let Some(blob) = blob_id.and_then(|id| self.find_object(id).map_err(|e| err.get_or_insert(e.into())).ok()) {
            target.merge(gix_mailmap::parse_ignore_errors(&blob.data));
        }

        let configured_path = self
            .config_snapshot()
            .trusted_path(&Mailmap::FILE)
            .and_then(|res| res.map_err(|e| err.get_or_insert(e.into())).ok());

        if let Some(mut file) =
            configured_path.and_then(|path| std::fs::File::open(path).map_err(|e| err.get_or_insert(e.into())).ok())
        {
            buf.clear();
            std::io::copy(&mut file, &mut buf)
                .map_err(|e| err.get_or_insert(e.into()))
                .ok();
            target.merge(gix_mailmap::parse_ignore_errors(&buf));
        }

        err.map_or(Ok(()), Err)
    }
}
