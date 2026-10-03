use gix::{
    Result,
    error::{ErrorExt, ResultExt, message},
};
use rusqlite::{OptionalExtension, params};

/// A version to be incremented whenever the database layout is changed, to refresh it automatically.
const VERSION: usize = 1;

pub fn create(path: impl AsRef<std::path::Path>) -> Result<rusqlite::Connection> {
    let path = path.as_ref();
    let mut con = rusqlite::Connection::open(path).or_error()?;
    let meta_table = r#"
        CREATE TABLE if not exists meta(
            version int
        )"#;
    con.execute_batch(meta_table).or_error()?;
    let version: Option<usize> = con
        .query_row("SELECT version FROM meta", [], |r| r.get(0))
        .optional()
        .or_error()?;
    match version {
        None => {
            con.execute("INSERT into meta(version) values(?)", params![VERSION])
                .or_error()?;
        }
        Some(version) if version != VERSION => match con.close() {
            Ok(()) => {
                std::fs::remove_file(path).or_raise(|| {
                    message!(
                        "Failed to remove incompatible database file at {path}",
                        path = path.display()
                    )
                })?;
                con = rusqlite::Connection::open(path).or_error()?;
                con.execute_batch(meta_table).or_error()?;
                con.execute("INSERT into meta(version) values(?)", params![VERSION])
                    .or_error()?;
            }
            Err((_, err)) => return Err(err.raise()),
        },
        _ => {}
    }
    con.execute_batch(
        r#"
        CREATE TABLE if not exists commits(
            hash blob(20) NOT NULL PRIMARY KEY
        )
        "#,
    )
    .or_error()?;
    // Files are stored as paths which also have an id for referencing purposes
    con.execute_batch(
        r#"
        CREATE TABLE if not exists files(
            file_id integer NOT NULL PRIMARY KEY,
            file_path text UNIQUE
        )
        "#,
    )
    .or_error()?;
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
    )
    .or_error()?;

    Ok(con)
}
