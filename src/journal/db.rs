use super::JournalEntry;
use crate::{ApiVersion, Error};
use log::{debug, error, info};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::NO_PARAMS;
use rusqlite::{named_params, params, Row};
use rusty_ulid::Ulid;
use std::str::FromStr;

type SchemaVersion = u32;

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

#[derive(Clone)]
pub struct Db {
    pool: Pool,
}

impl std::convert::From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Rusqlite(err.to_string())
    }
}

impl std::convert::From<r2d2::Error> for Error {
    fn from(err: r2d2::Error) -> Self {
        Error::R2d2(err.to_string())
    }
}

impl std::convert::From<rusty_ulid::DecodingError> for Error {
    fn from(err: rusty_ulid::DecodingError) -> Self {
        Error::UlidDecoding(err)
    }
}

impl std::convert::From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerdeJson(err.to_string())
    }
}

static MIGRATIONS: &[&str] = &[
    "CREATE TABLE schema_version (version INTEGER NOT NULL)",
    "INSERT INTO schema_version VALUES (1)",
    "CREATE TABLE table_je_id (name TEXT NOT NULL, je_id TEXT NOT NULL)",
    "CREATE TABLE journal_entry (id TEXT NOT NULL, version INTEGER NOT NULL, action TEXT NOT NULL);",
    "CREATE UNIQUE INDEX idx_journal_entry_id ON journal_entry(id);",
    "CREATE TABLE account (id TEXT NOT NULL, number INTEGER NOT NULL, description TEXT NOT NULL, type TEXT NOT NULL, parent_id TEXT, statement TEXT, FOREIGN KEY(parent_id) REFERENCES account(id));",
    "CREATE UNIQUE INDEX idx_account_id ON account(id);",
    "CREATE UNIQUE INDEX idx_account_parent_number ON account(parent_id, number);",
];

impl Db {
    pub fn new() -> Result<Db, Error> {
        // Start N db executor actors (N = number of cores avail)
        let manager = SqliteConnectionManager::file("bitcoin-aba.db");
        let pool = Pool::new(manager)?;
        Db::exec_migrations(&pool.get().expect("connection"))?;
        Ok(Db { pool })
    }

    pub fn new_mem() -> Result<Db, Error> {
        // Start N db executor actors (N = number of cores avail)
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::new(manager)?;
        Db::exec_migrations(&pool.get().expect("connection"))?;
        Ok(Db { pool })
    }

    fn exec_migrations(conn: &Connection) -> Result<(), Error> {
        let version: SchemaVersion = Db::select_version(conn)?;
        if version == MIGRATIONS.len() as SchemaVersion {
            info!("Up to date, no migration needed");
            return Ok(());
        }

        let stmts = &MIGRATIONS[(version as usize)..];
        let mut i: SchemaVersion = version;
        for stmt in stmts {
            debug!("Conn.execute: {}", &stmt);
            let res = conn.execute(stmt, NO_PARAMS);
            if res.is_err() {
                error!("Migration failed on:\n{}\n{:?}", stmt, res);
                break;
            }

            i += 1;
        }

        Db::update_version(conn, i)?;
        Ok(())
    }

    fn select_version(conn: &Connection) -> rusqlite::Result<SchemaVersion> {
        let statement = conn.prepare_cached("SELECT version FROM schema_version");
        match statement {
            Err(rusqlite::Error::SqliteFailure(e, Some(msg))) => {
                if msg == "no such table: schema_version" {
                    Ok(0)
                } else {
                    Err(rusqlite::Error::SqliteFailure(e, Some(msg)))
                }
            }
            Ok(mut stmt) => {
                let mut rows = stmt.query(NO_PARAMS)?;
                match rows.next()? {
                    Some(row) => {
                        let version: SchemaVersion = row.get(0)?;
                        Ok(version)
                    }
                    None => Ok(0),
                }
            }
            _ => Ok(0),
        }
    }

    fn update_version(conn: &Connection, version: SchemaVersion) -> rusqlite::Result<usize> {
        conn.execute(
            "UPDATE schema_version SET version=:version",
            params![&version],
        )
    }

    pub fn insert_entry(&self, entry: JournalEntry) -> Result<(), Error> {
        // rusqlite::Result<usize> {
        let conn = self.pool.get().expect("connection");
        conn.execute_named(
            "INSERT INTO journal_entry (id, version, action) VALUES (:id, :version, :action)",
            named_params![":id": &entry.id.to_string(), ":version": entry.version, ":action": serde_json::to_string(&entry.action).unwrap()],
        ).map_err(|e| e.into()).map(|_s| ())
        // TODO error if result size isn't 1
    }

    fn convert_row_entry(row: &Row) -> Result<JournalEntry, Error> {
        let id = Ulid::from_str(row.get::<_, String>(0)?.as_str())?; //.map_err(|e| Error::from(e))?;
        let version = row.get::<_, ApiVersion>(1)?;
        let action = serde_json::from_str(row.get::<_, String>(2)?.as_str())?; //.map_err(|e| Error::from(e))?;
        Ok(JournalEntry {
            id,
            version,
            action,
        })
    }

    // Select entries
    pub fn select_entries(&self) -> Result<Vec<JournalEntry>, Error> {
        let conn = self.pool.get().expect("connection");
        let mut stmt = conn
            .prepare("SELECT * FROM journal_entry ORDER BY id")
            .map_err(Error::from)?;

        let entity_rows = stmt
            .query_and_then(NO_PARAMS, Db::convert_row_entry)
            .map_err(Error::from)?;

        let mut result = Vec::new();
        for entry in entity_rows {
            //debug!("entry: {:?}", &entry?);
            result.push(entry?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use crate::journal::db::Db;
    use crate::journal::{Account, AccountType, Action, Entity, EntityType, JournalEntry};

    #[test]
    pub fn test_insert_select() {
        let db = Db::new_mem().unwrap();

        let org = Entity::new(EntityType::Organization, "Test Org".to_string(), None);
        let account = Account::new(
            100,
            "Test account".to_string(),
            AccountType::Organization {
                parent_id: None,
                entity_id: org.id,
            },
        );
        let entry = JournalEntry::new_gen_id(Action::AddAccount { account });

        db.insert_entry(entry.clone()).unwrap();
        let entries = db.select_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries.get(0).unwrap(), &entry);
    }
}
