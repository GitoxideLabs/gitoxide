use crate::{
    basics::HashId,
    constants,
    error::Error,
    pq::{MergedIterPQueue, PqEntry},
    record::Record,
    table::{Table, TableIter},
};

/// A merged view over multiple tables, typically oldest to newest.
#[derive(Debug, Clone)]
pub struct MergedTable {
    /// Source tables.
    pub tables: Vec<Table>,
    /// Hash in use by all tables.
    pub hash_id: HashId,
    /// Smallest update index across all tables.
    pub min_update_index: u64,
    /// Largest update index across all tables.
    pub max_update_index: u64,
    /// Whether deletions should be filtered while iterating.
    pub suppress_deletions: bool,
}

impl MergedTable {
    /// Create a merged table from `tables`.
    pub fn new(tables: Vec<Table>) -> Result<Self, Error> {
        let mut hash_id = HashId::Sha1;
        let mut min_update_index = 0;
        let mut max_update_index = 0;

        for (idx, table) in tables.iter().enumerate() {
            if idx == 0 {
                hash_id = table.hash_id;
                min_update_index = table.min_update_index;
                max_update_index = table.max_update_index;
            } else {
                if table.hash_id != hash_id {
                    return Err(Error::Malformed("all merged tables must share hash id"));
                }
                min_update_index = min_update_index.min(table.min_update_index);
                max_update_index = max_update_index.max(table.max_update_index);
            }
        }

        Ok(Self {
            tables,
            hash_id,
            min_update_index,
            max_update_index,
            suppress_deletions: false,
        })
    }

    /// Create an iterator over merged refs.
    pub fn ref_iter(&self) -> Result<MergedIter<'_>, Error> {
        self.iter(constants::BLOCK_TYPE_REF)
    }

    /// Create an iterator over merged logs.
    pub fn log_iter(&self) -> Result<MergedIter<'_>, Error> {
        self.iter(constants::BLOCK_TYPE_LOG)
    }

    /// Create an iterator over records of the given block type.
    pub fn iter(&self, typ: u8) -> Result<MergedIter<'_>, Error> {
        MergedIter::new(self, typ)
    }
}

/// Iterator over merged table records.
pub struct MergedIter<'a> {
    subiters: Vec<TableIter<'a>>,
    pq: MergedIterPQueue,
    suppress_deletions: bool,
}

impl<'a> MergedIter<'a> {
    fn new(table: &'a MergedTable, typ: u8) -> Result<Self, Error> {
        let mut subiters = Vec::with_capacity(table.tables.len());
        for t in &table.tables {
            subiters.push(t.iter(typ)?);
        }

        let mut out = Self {
            subiters,
            pq: MergedIterPQueue::default(),
            suppress_deletions: table.suppress_deletions,
        };
        out.rebuild_pq()?;
        Ok(out)
    }

    fn rebuild_pq(&mut self) -> Result<(), Error> {
        self.pq.clear();
        for idx in 0..self.subiters.len() {
            self.advance_subiter(idx)?;
        }
        Ok(())
    }

    fn advance_subiter(&mut self, idx: usize) -> Result<(), Error> {
        if let Some(record) = self.subiters[idx].next_record()? {
            self.pq.push(PqEntry { index: idx, record });
        }
        Ok(())
    }

    /// Seek all subiterators to `key`.
    pub fn seek_key(&mut self, key: &[u8]) -> Result<(), Error> {
        for subiter in &mut self.subiters {
            subiter.seek_key(key)?;
        }
        self.rebuild_pq()
    }

    /// Return the next merged record.
    pub fn next_record(&mut self) -> Result<Option<Record>, Error> {
        loop {
            let Some(entry) = self.pq.pop() else {
                return Ok(None);
            };

            self.advance_subiter(entry.index)?;

            while let Some(top) = self.pq.peek() {
                if top.record.cmp_key(&entry.record)? != std::cmp::Ordering::Equal {
                    break;
                }
                let dup = self.pq.pop().expect("just peeked");
                self.advance_subiter(dup.index)?;
            }

            if self.suppress_deletions && entry.record.is_deletion() {
                continue;
            }
            return Ok(Some(entry.record));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        fs,
        path::{Path, PathBuf},
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{constants, record::Record};

    use super::{MergedTable, Table};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("valid time")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("gix-reftable-merged-{stamp}"));
            fs::create_dir_all(&path).expect("temp dir");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn run(cwd: &Path, args: &[&str]) {
        let status = Command::new(args[0])
            .args(&args[1..])
            .current_dir(cwd)
            .status()
            .expect("command executed");
        assert!(status.success(), "command failed: {args:?}");
    }

    fn create_table() -> Option<(TempDir, Table)> {
        let tmp = TempDir::new();
        let source = tmp.path.join("source");
        let clone = tmp.path.join("clone");
        fs::create_dir_all(&source).expect("source dir");

        run(&source, &["git", "init", "-q"]);
        run(&source, &["git", "config", "user.name", "committer"]);
        run(&source, &["git", "config", "user.email", "committer@example.com"]);
        run(&source, &["git", "config", "commit.gpgSign", "false"]);
        fs::write(source.join("file"), "hello\n").expect("write file");
        run(&source, &["git", "add", "file"]);
        run(&source, &["git", "commit", "-q", "-m", "c1"]);
        let clone_status = Command::new("git")
            .args(["clone", "--ref-format=reftable"])
            .arg(source.to_str().expect("utf-8 path"))
            .arg(clone.to_str().expect("utf-8 path"))
            .current_dir(&tmp.path)
            .status()
            .ok()?;
        if !clone_status.success() {
            return None;
        }

        let list = fs::read_to_string(clone.join(".git/reftable/tables.list")).ok()?;
        let table_name = list.lines().next()?;
        let table = Table::open(clone.join(".git/reftable").join(table_name)).ok()?;
        Some((tmp, table))
    }

    #[test]
    fn merged_iterator_deduplicates_by_key_with_recency_preference() {
        let Some((_tmp, table)) = create_table() else {
            return;
        };
        let merged = MergedTable::new(vec![table.clone(), table]).expect("merged");

        let mut iter = merged.ref_iter().expect("iter");
        let mut ref_names = Vec::new();
        while let Some(rec) = iter.next_record().expect("next") {
            let Record::Ref(rec) = rec else {
                panic!("expected ref");
            };
            ref_names.push(rec.refname);
        }

        let unique_names = ref_names.iter().collect::<BTreeSet<_>>();
        assert_eq!(unique_names.len(), ref_names.len());
        assert!(ref_names.iter().any(|name| name == "HEAD"));
        assert!(ref_names.iter().any(|name| name.starts_with("refs/heads/")));
    }

    #[test]
    fn merged_seek_key() {
        let Some((_tmp, table)) = create_table() else {
            return;
        };
        let merged = MergedTable::new(vec![table.clone(), table]).expect("merged");

        let mut iter = merged.iter(constants::BLOCK_TYPE_REF).expect("iter");
        iter.seek_key(b"refs/heads/").expect("seek");
        let rec = iter.next_record().expect("next").expect("record");
        let Record::Ref(rec) = rec else {
            panic!("expected ref");
        };
        assert!(rec.refname.starts_with("refs/heads/"));
    }
}
