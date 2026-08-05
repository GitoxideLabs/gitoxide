use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    basics::HashId,
    error::Error,
    merged::MergedTable,
    record::{LogRecord, RefRecord},
    table::Table,
    writer::{WriteOptions, Writer},
};

/// Options controlling stack behavior.
#[derive(Debug, Clone)]
pub struct StackOptions {
    /// Hash used when creating new tables.
    pub hash_id: HashId,
    /// Disable automatic compaction after commits.
    pub disable_auto_compact: bool,
    /// Minimum number of tables required before compaction.
    pub auto_compaction_factor: usize,
    /// Write options used for emitted tables.
    pub write_options: WriteOptions,
}

impl Default for StackOptions {
    fn default() -> Self {
        let write_options = WriteOptions::default();
        Self {
            hash_id: write_options.hash_id,
            disable_auto_compact: false,
            auto_compaction_factor: 2,
            write_options,
        }
    }
}

/// A stack of reftable files controlled by `tables.list`.
#[derive(Debug, Clone)]
pub struct Stack {
    dir: PathBuf,
    opts: StackOptions,
    table_names: Vec<String>,
    tables: Vec<Table>,
    merged: MergedTable,
}

impl Stack {
    /// Open or initialize a stack at `dir`.
    pub fn open(dir: impl AsRef<Path>, mut opts: StackOptions) -> Result<Self, Error> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        opts.write_options.hash_id = opts.hash_id;

        let mut out = Self {
            dir,
            opts,
            table_names: Vec::new(),
            tables: Vec::new(),
            merged: MergedTable::new(Vec::new())?,
        };
        out.ensure_tables_list()?;
        out.reload()?;
        Ok(out)
    }

    /// Return the stack directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Return loaded table names in stack order.
    pub fn table_names(&self) -> &[String] {
        &self.table_names
    }

    /// Return loaded table handles in stack order.
    pub fn tables(&self) -> &[Table] {
        &self.tables
    }

    /// Return merged view of all tables.
    pub fn merged(&self) -> &MergedTable {
        &self.merged
    }

    /// Return the next update index.
    pub fn next_update_index(&self) -> u64 {
        self.merged.max_update_index.saturating_add(1).max(1)
    }

    /// Reload stack metadata and all tables from disk.
    pub fn reload(&mut self) -> Result<(), Error> {
        self.ensure_tables_list()?;
        let list = fs::read_to_string(self.tables_list_path())?;

        self.table_names = list
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(ToOwned::to_owned)
            .collect();

        self.tables.clear();
        for name in &self.table_names {
            let table = Table::open(self.dir.join(name))?;
            self.tables.push(table);
        }

        self.merged = MergedTable::new(self.tables.clone())?;
        Ok(())
    }

    /// Run basic consistency checks on all tables and merged iteration.
    pub fn fsck(&self) -> Result<(), Error> {
        let mut prev_max = None;
        for (idx, table) in self.tables.iter().enumerate() {
            let table_name = &self.table_names[idx];
            if !is_valid_table_name(table_name) {
                return Err(Error::Api("invalid reftable table name"));
            }
            if table.hash_id != self.opts.hash_id {
                return Err(Error::Api("table hash id does not match stack hash id"));
            }
            if table.min_update_index > table.max_update_index {
                return Err(Error::Api("table has invalid update-index range"));
            }
            if let Some(prev) = prev_max {
                if table.min_update_index <= prev {
                    return Err(Error::Api("table update-index ranges must be strictly increasing"));
                }
            }
            prev_max = Some(table.max_update_index);

            let path = self.dir.join(table_name);
            if !path.is_file() {
                return Err(Error::Api("table listed in tables.list is missing"));
            }
        }

        let mut refs = self.merged.ref_iter()?;
        while refs.next_record()?.is_some() {}
        let mut logs = self.merged.log_iter()?;
        while logs.next_record()?.is_some() {}

        Ok(())
    }

    /// Create a mutable transaction.
    pub fn transaction(&mut self) -> Transaction<'_> {
        Transaction {
            stack: self,
            refs: Vec::new(),
            logs: Vec::new(),
        }
    }

    fn ensure_tables_list(&self) -> Result<(), Error> {
        let path = self.tables_list_path();
        if !path.exists() {
            fs::write(path, "")?;
        }
        Ok(())
    }

    fn tables_list_path(&self) -> PathBuf {
        self.dir.join("tables.list")
    }

    fn write_tables_list(&self, names: &[String]) -> Result<(), Error> {
        let path = self.tables_list_path();
        let tmp = path.with_extension("list.lock");
        let mut content = String::new();
        for name in names {
            content.push_str(name);
            content.push('\n');
        }
        fs::write(&tmp, content)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    fn write_table_bytes(&self, min: u64, max: u64, bytes: &[u8]) -> Result<String, Error> {
        let suffix = crc32fast::hash(bytes);
        let name = format!("0x{min:012x}-0x{max:012x}-{suffix:08x}.ref");
        let path = self.dir.join(&name);
        let tmp = path.with_extension("lock");
        fs::write(&tmp, bytes)?;
        fs::rename(tmp, path)?;
        Ok(name)
    }

    /// Compact all tables into one when threshold conditions are met.
    pub fn maybe_auto_compact(&mut self) -> Result<(), Error> {
        if self.opts.disable_auto_compact {
            return Ok(());
        }
        if self.tables.len() < self.opts.auto_compaction_factor {
            return Ok(());
        }

        let mut refs = Vec::<RefRecord>::new();
        let mut ref_iter = self.merged.ref_iter()?;
        while let Some(rec) = ref_iter.next_record()? {
            if let crate::record::Record::Ref(r) = rec {
                refs.push(r);
            }
        }

        let mut logs = Vec::<LogRecord>::new();
        let mut log_iter = self.merged.log_iter()?;
        while let Some(rec) = log_iter.next_record()? {
            if let crate::record::Record::Log(l) = rec {
                logs.push(l);
            }
        }

        if refs.is_empty() && logs.is_empty() {
            return Ok(());
        }

        let min = self.merged.min_update_index;
        let max = self.merged.max_update_index;
        let mut writer = Writer::new(self.opts.write_options.clone());
        writer.set_limits(min, max)?;
        for r in refs {
            writer.add_ref(r)?;
        }
        for l in logs {
            writer.add_log(l)?;
        }
        let bytes = writer.finish()?;
        let compacted = self.write_table_bytes(min, max, &bytes)?;

        let old_names = self.table_names.clone();
        self.write_tables_list(&[compacted])?;
        for old in old_names {
            let _ = fs::remove_file(self.dir.join(old));
        }

        self.reload()
    }
}

fn is_valid_table_name(name: &str) -> bool {
    let Some(base) = name.strip_suffix(".ref") else {
        return false;
    };
    let mut parts = base.split('-');
    let Some(min) = parts.next() else {
        return false;
    };
    let Some(max) = parts.next() else {
        return false;
    };
    let Some(hash) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    valid_hex_component(min, 12)
        && valid_hex_component(max, 12)
        && hash.len() == 8
        && hash.chars().all(|c| c.is_ascii_hexdigit())
}

fn valid_hex_component(value: &str, width: usize) -> bool {
    let Some(hex) = value.strip_prefix("0x") else {
        return false;
    };
    hex.len() == width && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Mutable stack transaction.
pub struct Transaction<'a> {
    stack: &'a mut Stack,
    refs: Vec<RefRecord>,
    logs: Vec<LogRecord>,
}

impl Transaction<'_> {
    /// Add a ref update to this transaction.
    pub fn add_ref(&mut self, rec: RefRecord) {
        self.refs.push(rec);
    }

    /// Add a log update to this transaction.
    pub fn add_log(&mut self, rec: LogRecord) {
        self.logs.push(rec);
    }

    /// Commit this transaction, persisting a new table and reloading the stack.
    pub fn commit(mut self) -> Result<(), Error> {
        let update_index = self.stack.next_update_index();
        let mut writer = Writer::new(self.stack.opts.write_options.clone());
        writer.set_limits(update_index, update_index)?;

        for mut r in self.refs.drain(..) {
            r.update_index = update_index;
            writer.add_ref(r)?;
        }
        for mut l in self.logs.drain(..) {
            l.update_index = update_index;
            writer.add_log(l)?;
        }

        let bytes = writer.finish()?;
        let name = self.stack.write_table_bytes(update_index, update_index, &bytes)?;

        let mut names = self.stack.table_names.clone();
        names.push(name);
        self.stack.write_tables_list(&names)?;
        self.stack.reload()?;
        self.stack.maybe_auto_compact()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{error::Error, record::RefRecord, record::RefValue, writer::WriteOptions};

    use super::{Stack, StackOptions};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("valid time")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("gix-reftable-stack-{stamp}"));
            fs::create_dir_all(&path).expect("temp dir");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn transaction_commit_and_reload() {
        let tmp = TempDir::new();
        let mut stack = Stack::open(
            &tmp.path,
            StackOptions {
                disable_auto_compact: true,
                write_options: WriteOptions {
                    block_size: 128,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("stack");

        let mut tx = stack.transaction();
        tx.add_ref(RefRecord {
            refname: "HEAD".into(),
            update_index: 0,
            value: RefValue::Symref("refs/heads/main".into()),
        });
        tx.add_ref(RefRecord {
            refname: "refs/heads/main".into(),
            update_index: 0,
            value: RefValue::Val1(vec![1; 20]),
        });
        tx.commit().expect("commit");

        assert_eq!(stack.tables().len(), 1);
        stack.reload().expect("reload");
        assert_eq!(stack.tables().len(), 1);
        stack.fsck().expect("fsck");
    }

    #[test]
    fn auto_compaction_reduces_table_count() {
        let tmp = TempDir::new();
        let mut stack = Stack::open(
            &tmp.path,
            StackOptions {
                auto_compaction_factor: 2,
                write_options: WriteOptions {
                    block_size: 96,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("stack");

        for idx in 0..3u8 {
            let mut tx = stack.transaction();
            tx.add_ref(RefRecord {
                refname: format!("refs/heads/{idx}"),
                update_index: 0,
                value: RefValue::Val1(vec![idx; 20]),
            });
            tx.commit().expect("commit");
        }

        assert!(stack.tables().len() <= 2, "compaction should reduce table fan-out");
    }

    #[test]
    fn fsck_detects_missing_tables() {
        let tmp = TempDir::new();
        let mut stack = Stack::open(&tmp.path, StackOptions::default()).expect("stack");

        let mut tx = stack.transaction();
        tx.add_ref(RefRecord {
            refname: "refs/heads/main".into(),
            update_index: 0,
            value: RefValue::Val1(vec![1; 20]),
        });
        tx.commit().expect("commit");

        let table = stack.table_names()[0].clone();
        fs::remove_file(tmp.path.join(table)).expect("remove table");
        let err = stack.fsck().expect_err("must fail");
        assert!(matches!(err, Error::Api(_)));
    }
}
