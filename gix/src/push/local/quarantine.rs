use std::{path::Path, sync::atomic::AtomicBool};

use gix_object::{Exists, FindExt, Write};

use crate::{Repository, Result, error::ResultExt, push};

pub(super) struct Quarantine {
    directory: tempfile::TempDir,
    pub objects: gix_odb::loose::Store,
}

impl Quarantine {
    pub fn new(repo: &Repository) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("incoming-")
            .tempdir_in(repo.objects.store_ref().path())
            .or_error()?;
        let objects = gix_odb::loose::Store::at(directory.path(), repo.object_hash());
        Ok(Self { directory, objects })
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn migrate(&self, destination: &Repository, interrupt: &AtomicBool) -> Result<()> {
        // Include objects written by the pre-receive hook, whether loose or packed. The temporary
        // alternate allows hooks' delta packs to refer to objects already held by the receiver.
        std::fs::create_dir_all(self.path().join("info")).or_error()?;
        let alternate =
            super::hooks::quoted_path(&destination.current_dir().join(destination.objects.store_ref().path()))?;
        let mut alternate = gix_path::into_bstr(Path::new(&alternate))?.into_owned();
        alternate.push(b'\n');
        std::fs::write(self.path().join("info/alternates"), &alternate).or_error()?;
        let objects = gix_odb::at(self.path(), destination.object_hash()).or_error()?;
        let mut buffer = Vec::new();
        for object_id in objects.iter()? {
            push::plan::check_interrupt(interrupt)?;
            let object_id = object_id.or_error()?;
            if destination.objects.exists(&object_id) {
                continue;
            }
            let object = objects.find(&object_id, &mut buffer)?;
            object.verify_checksum(&object_id)?;
            destination
                .objects
                .write_buf_with_known_id(object.kind, object.data, object_id)?;
        }
        Ok(())
    }
}
