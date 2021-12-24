use log::{debug, error, info};
use super::Error;
use rusqlite::params;
use rusqlite::NO_PARAMS;

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

impl std::convert::From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Rusqlite(err)
    }
}

static MIGRATIONS: &[&str] = &[
    "CREATE TABLE version (version INTEGER)",
    "INSERT INTO version VALUES (1)",
    "CREATE TABLE journal_entry (id TEXT NOT NULL, version INTEGER NOT NULL, entry TEXT NOT NULL);",
    "CREATE UNIQUE INDEX idx_journal_entry_id ON journal_entry(id);",
    "CREATE TABLE account (id TEXT NOT NULL, number INTEGER NOT NULL, description TEXT NOT NULL, type TEXT NOT NULL, parent_id TEXT, statement TEXT, FOREIGN KEY(parent_id) REFERENCES account(id));",
    "CREATE UNIQUE INDEX idx_account_id ON account(id);",
    "CREATE UNIQUE INDEX idx_account_parent_number ON account(parent_id, number);",
];

pub(super) fn migrations(conn: Connection) -> Result<(), Error> {
    let version = get_schema_version(&conn)?;
    info!("At version {}", version);

    if version == MIGRATIONS.len() as i32 {
        info!("Up to date, no migration needed");
        return Ok(());
    }

    let stmts = &MIGRATIONS[(version as usize)..];
    let mut i: i32 = version;
    for stmt in stmts {
        debug!("Conn.execute: {}", &stmt);
        let res = conn.execute(stmt, NO_PARAMS);
        if res.is_err() {
            error!("Migration failed on:\n{}\n{:?}", stmt, res);
            break;
        }

        i += 1;
    }

    set_schema_version(&conn, i)?;
    Ok(())
}

fn get_schema_version(conn: &Connection) -> rusqlite::Result<i32> {
    let statement = conn.prepare_cached("SELECT version FROM version");
    match statement {
        Err(rusqlite::Error::SqliteFailure(e, Some(msg))) => {
            if msg == "no such table: version" {
                Ok(0)
            } else {
                Err(rusqlite::Error::SqliteFailure(e, Some(msg)))
            }
        }
        Ok(mut stmt) => {
            let mut rows = stmt.query(NO_PARAMS)?;
            match rows.next()? {
                Some(row) => {
                    let version: i32 = row.get(0)?;
                    Ok(version)
                }
                None => Ok(0),
            }
        }
        _ => Ok(0),
    }
}

fn set_schema_version(conn: &Connection, version: i32) -> rusqlite::Result<usize> {
    conn.execute("UPDATE version SET version=:version", params![version])
}
