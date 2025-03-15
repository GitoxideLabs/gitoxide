use gix_hash::ObjectId;
use gix_object::Exists;
use std::ops::DerefMut;

impl Clone for crate::Repository {
    fn clone(&self) -> Self {
        let mut new = crate::Repository::from_refs_and_objects(
            self.refs.clone(),
            self.objects.clone(),
            self.work_tree.clone(),
            self.common_dir.clone(),
            self.config.clone(),
            self.options.clone(),
            #[cfg(feature = "index")]
            self.index.clone(),
            self.shallow_commits.clone(),
            #[cfg(feature = "attributes")]
            self.modules.clone(),
        );

        if self.bufs.is_none() {
            new.bufs.take();
        }

        new
    }
}

impl std::fmt::Debug for crate::Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Repository")
            .field("kind", &self.kind())
            .field("git_dir", &self.git_dir())
            .field("workdir", &self.workdir())
            .finish()
    }
}

impl PartialEq<crate::Repository> for crate::Repository {
    fn eq(&self, other: &crate::Repository) -> bool {
        self.git_dir().canonicalize().ok() == other.git_dir().canonicalize().ok()
            && self.work_tree.as_deref().and_then(|wt| wt.canonicalize().ok())
                == other.work_tree.as_deref().and_then(|wt| wt.canonicalize().ok())
    }
}

impl From<&crate::ThreadSafeRepository> for crate::Repository {
    fn from(repo: &crate::ThreadSafeRepository) -> Self {
        crate::Repository::from_refs_and_objects(
            repo.refs.clone(),
            gix_odb::memory::Proxy::from(gix_odb::Cache::from(repo.objects.to_handle())).with_write_passthrough(),
            repo.work_tree.clone(),
            repo.common_dir.clone(),
            repo.config.clone(),
            repo.linked_worktree_options.clone(),
            #[cfg(feature = "index")]
            repo.index.clone(),
            repo.shallow_commits.clone(),
            #[cfg(feature = "attributes")]
            repo.modules.clone(),
        )
    }
}

impl From<crate::ThreadSafeRepository> for crate::Repository {
    fn from(repo: crate::ThreadSafeRepository) -> Self {
        crate::Repository::from_refs_and_objects(
            repo.refs,
            gix_odb::memory::Proxy::from(gix_odb::Cache::from(repo.objects.to_handle())).with_write_passthrough(),
            repo.work_tree,
            repo.common_dir,
            repo.config,
            repo.linked_worktree_options,
            #[cfg(feature = "index")]
            repo.index,
            repo.shallow_commits,
            #[cfg(feature = "attributes")]
            repo.modules.clone(),
        )
    }
}

impl From<crate::Repository> for crate::ThreadSafeRepository {
    fn from(r: crate::Repository) -> Self {
        crate::ThreadSafeRepository {
            refs: r.refs,
            objects: r.objects.into_inner().store(),
            work_tree: r.work_tree,
            common_dir: r.common_dir,
            config: r.config,
            linked_worktree_options: r.options,
            #[cfg(feature = "index")]
            index: r.index,
            #[cfg(feature = "attributes")]
            modules: r.modules,
            shallow_commits: r.shallow_commits,
        }
    }
}

impl gix_object::Write for crate::Repository {
    fn write(&self, object: &dyn gix_object::WriteTo) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut buf = self.empty_reusable_buffer();
        object.write_to(buf.deref_mut())?;
        self.write_buf(object.kind(), &buf)
    }

    fn write_buf(&self, object: gix_object::Kind, from: &[u8]) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let oid = gix_object::compute_hash(self.object_hash(), object, from);
        if self.objects.exists(&oid) {
            return Ok(oid);
        }
        self.objects.write_buf(object, from)
    }

    fn write_stream(
        &self,
        kind: gix_object::Kind,
        size: u64,
        from: &mut dyn std::io::Read,
    ) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut buf = self.empty_reusable_buffer();
        let bytes = std::io::copy(from, buf.deref_mut())?;
        if size != bytes {
            return Err(format!("Found {bytes} bytes in stream, but had {size} bytes declared").into());
        }
        self.write_buf(kind, &buf)
    }
}

impl gix_object::FindHeader for crate::Repository {
    fn try_header(&self, id: &gix_hash::oid) -> Result<Option<gix_object::Header>, gix_object::find::Error> {
        if id == ObjectId::empty_tree(self.object_hash()) {
            return Ok(Some(gix_object::Header {
                kind: gix_object::Kind::Tree,
                size: 0,
            }));
        }
        self.objects.try_header(id)
    }
}

impl gix_object::Find for crate::Repository {
    fn try_find<'a>(
        &self,
        id: &gix_hash::oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<gix_object::Data<'a>>, gix_object::find::Error> {
        if id == ObjectId::empty_tree(self.object_hash()) {
            buffer.clear();
            return Ok(Some(gix_object::Data {
                kind: gix_object::Kind::Tree,
                data: &[],
            }));
        }
        self.objects.try_find(id, buffer)
    }
}

impl gix_object::Exists for crate::Repository {
    fn exists(&self, id: &gix_hash::oid) -> bool {
        if id == ObjectId::empty_tree(self.object_hash()) {
            return true;
        }
        self.objects.exists(id)
    }
}
