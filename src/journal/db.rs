use super::JournalEntry;
use crate::Error;
use log::{debug, error, info};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::NO_PARAMS;
use rusqlite::{named_params, params};

type SchemaVersion = u32;

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

#[derive(Clone)]
pub struct Db {
    pool: Pool,
}

impl std::convert::From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Rusqlite(err)
    }
}

impl std::convert::From<r2d2::Error> for Error {
    fn from(err: r2d2::Error) -> Self {
        Error::R2d2(err)
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
        let manager = SqliteConnectionManager::file("bitcoin-aba.sqlite");
        let pool = Pool::new(manager)?;
        Db::exec_migrations(&pool.get().expect("connection"))?;
        Ok(Db { pool })
    }

    fn exec_migrations(conn: &Connection) -> Result<(), Error> {
        let version: SchemaVersion = Db::select_version(&conn)?;
        info!("At version {}", version);

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

        Db::update_version(&conn, i)?;
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

    //// Select accounts
    //pub fn select_accounts(
    //    conn: &Connection,
    //) -> rusqlite::Result<usize> {
    //    conn.execute_named(
    //        "SELECT FROM journal_entry max(id)",
    //        named_params![":id": &entry.id.to_string(), ":version": entry.version, ":action": serde_json::to_string(&entry.action).unwrap()],
    //    )
    //}

    //// Insert account
    //pub fn insert_account(connection: &Connection, account: &Account) -> Result<String, CommandError> {
    //    let mut sql_statement = connection.prepare_cached("INSERT INTO account (id, number, description, type, parent_id, statement) VALUES (:id, :number, :description, :type, :parent_id, :statement)")?;
    //    match account.account_type {
    //        AccountType::Organization { parent_id } => {
    //            sql_statement.execute(named_params! {
    //                ":id": account.id.to_string(),
    //                ":number": account.number,
    //                ":description": account.description,
    //                ":type": account.account_type.to_string(),
    //                ":parent_id": parent_id.map(|ulid| ulid.to_string()).to_sql()?,
    //                ":statement": Null,
    //            })?;
    //        }
    //        AccountType::OrganizationUnit { parent_id } => {
    //            sql_statement.execute(named_params! {
    //                ":id": account.id.to_string(),
    //                ":number": account.number,
    //                ":description": account.description,
    //                ":type": account.account_type.to_string(),
    //                ":parent_id": parent_id.to_string(),
    //                ":statement": Null,
    //            })?;
    //        }
    //        AccountType::Category {
    //            parent_id,
    //            statement,
    //        } => {
    //            sql_statement.execute(named_params! {
    //                ":id": account.id.to_string(),
    //                ":number": account.number,
    //                ":description": account.description,
    //                ":type": account.account_type.to_string(),
    //                ":parent_id": parent_id.to_string(),
    //                ":statement": statement.to_string(),
    //            })?;
    //        }
    //        AccountType::SubAccount { parent_id } => {
    //            sql_statement.execute(named_params! {
    //                ":id": account.id.to_string(),
    //                ":number": account.number,
    //                ":description": account.description,
    //                ":type": account.account_type.to_string(),
    //                ":parent_id": parent_id.to_string(),
    //                ":statement": Null,
    //            })?;
    //        }
    //    }
    //    Ok(account.id.to_string())
    //}
}
