use anyhow::Context;
use rusqlite::{OptionalExtension, params};

/// A version to be incremented whenever the database layout is changed, to refresh it automatically.
const VERSION: usize = 1;

pub fn create(
    path: impl AsRef<std::path::Path>,
    shared_repository_permissions: i32,
) -> anyhow::Result<rusqlite::Connection> {
    let path = path.as_ref();
    let open = || -> anyhow::Result<_> {
        let con = rusqlite::Connection::open(path)?;
        // SQLite gives new journals, WALs and shared-memory files the database's permissions.
        gix::fs::set_shared_repository_permissions(path, shared_repository_permissions)
            .with_context(|| format!("Could not set permissions of query database '{}'", path.display()))?;
        Ok(con)
    };
    let mut con = open()?;
    let meta_table = r#"
        CREATE TABLE if not exists meta(
            version int
        )"#;
    con.execute_batch(meta_table)?;
    let version: Option<usize> = con.query_row("SELECT version FROM meta", [], |r| r.get(0)).optional()?;
    match version {
        None => {
            con.execute("INSERT into meta(version) values(?)", params![VERSION])?;
        }
        Some(version) if version != VERSION => match con.close() {
            Ok(()) => {
                std::fs::remove_file(path).with_context(|| {
                    format!(
                        "Failed to remove incompatible database file at {path}",
                        path = path.display()
                    )
                })?;
                con = open()?;
                con.execute_batch(meta_table)?;
                con.execute("INSERT into meta(version) values(?)", params![VERSION])?;
            }
            Err((_, err)) => return Err(err.into()),
        },
        _ => {}
    }
    con.execute_batch(
        r#"
        CREATE TABLE if not exists commits(
            hash blob(20) NOT NULL PRIMARY KEY
        )
        "#,
    )?;
    // Files are stored as paths which also have an id for referencing purposes
    con.execute_batch(
        r#"
        CREATE TABLE if not exists files(
            file_id integer NOT NULL PRIMARY KEY,
            file_path text UNIQUE
        )
        "#,
    )?;
    con.execute_batch(
        r#"
        CREATE TABLE if not exists commit_file(
            hash blob(20),
            file_id text,
            has_diff boolean NOT NULL,
            lines_added integer NOT NULL,
            lines_removed integer NOT NULL,
            lines_before integer NOT NULL,
            lines_after integer NOT NULL,
            mode integer,
            source_file_id integer,
            FOREIGN KEY (hash) REFERENCES commits (hash),
            FOREIGN KEY (file_id) REFERENCES files (file_id),
            PRIMARY KEY (hash, file_id)
        )
        "#,
    )?;

    Ok(con)
}

#[cfg(all(test, unix))]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt};

    #[test]
    fn sharing_applies_to_database_recreation_and_live_journals() -> gix_testtools::Result {
        for (umask, group_mode, default_mode) in [(0o022, 0o664, 0o644), (0o077, 0o660, 0o600)] {
            if !gix_testtools::run_with_umask(umask)? {
                continue;
            }
            let temp = tempfile::TempDir::new()?;
            for (policy, expected_mode) in [(0, default_mode), (0o660, group_mode), (-0o640, 0o640)] {
                let path = temp.path().join(format!("query-{policy}"));
                let mut con = super::create(&path, policy)?;
                assert_eq!(
                    fs::metadata(&path)?.permissions().mode() & 0o777,
                    expected_mode,
                    "new databases honor sharing policy {policy} with umask {umask:o}"
                );

                // An active write transaction keeps the rollback journal available for inspection.
                // Changing the schema version also forces database recreation on the next open.
                let transaction = con.transaction()?;
                transaction.execute("UPDATE meta SET version = version + 1", [])?;
                let journal = path.with_file_name(format!("query-{policy}-journal"));
                assert_eq!(
                    fs::metadata(journal)?.permissions().mode() & 0o777,
                    expected_mode,
                    "SQLite's live journal inherits the database's permissions despite the umask"
                );
                transaction.commit()?;
                con.close().map_err(|(_, err)| err)?;

                let _recreated = super::create(&path, policy)?;
                assert_eq!(
                    fs::metadata(path)?.permissions().mode() & 0o777,
                    expected_mode,
                    "schema replacement applies the same sharing policy as initial creation"
                );
            }
        }
        Ok(())
    }
}
