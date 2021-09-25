#[macro_use]
extern crate rocket;

use std::sync::Mutex;

use rocket::response::Responder;
use rocket::serde::{json::serde_json, json::Json, Deserialize, Serialize};
use rocket::{response, Request, State};
use rusqlite::{named_params, Connection};
use rusty_ulid::Ulid;
use rusqlite::ToSql;

use crate::CommandError::ServerError;
use std::fmt;
use std::str::FromStr;
use rusqlite::types::Null;

type AccountId = Ulid;

static MIGRATIONS: &[&str] = &[
    "CREATE TABLE version (version INTEGER)",
    "INSERT INTO version VALUES (1)",
    "CREATE TABLE command (seq INTEGER PRIMARY KEY AUTOINCREMENT, version INTEGER NOT NULL, command TEXT NOT NULL);",
    "CREATE INDEX idx_command_seq ON command(seq);",
    "CREATE TABLE account (id TEXT UNIQUE NOT NULL, number INTEGER NOT NULL, description TEXT NOT NULL, type TEXT NOT NULL, parent_id TEXT, statement TEXT);",
    "CREATE INDEX idx_account_id ON account(id);",
    // "CREATE TABLE utxos (value INTEGER, keychain TEXT, vout INTEGER, txid BLOB, script BLOB);",
    // "CREATE INDEX idx_txid_vout ON utxos(txid, vout);",
    // "CREATE TABLE transactions (txid BLOB, raw_tx BLOB);",
    // "CREATE INDEX idx_txid ON transactions(txid);",
    // "CREATE TABLE transaction_details (txid BLOB, timestamp INTEGER, received INTEGER, sent INTEGER, fee INTEGER, height INTEGER, verified INTEGER DEFAULT 0);",
    // "CREATE INDEX idx_txdetails_txid ON transaction_details(txid);",
    // "CREATE TABLE last_derivation_indices (keychain TEXT, value INTEGER);",
    // "CREATE UNIQUE INDEX idx_indices_keychain ON last_derivation_indices(keychain);",
    // "CREATE TABLE checksums (keychain TEXT, checksum BLOB);",
    // "CREATE INDEX idx_checksums_keychain ON checksums(keychain);",
];

type DbConn = Mutex<Connection>;

pub enum Error {
    Rusqlite(rusqlite::Error),
}

impl std::convert::From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Rusqlite(err)
    }
}

pub fn get_connection(path: &str) -> Result<Connection, Error> {
    let connection = Connection::open(path)?;
    migrate(&connection)?;
    Ok(connection)
}

pub fn get_schema_version(conn: &Connection) -> rusqlite::Result<i32> {
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
            let mut rows = stmt.query([])?;
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

pub fn set_schema_version(conn: &Connection, version: i32) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE version SET version=:version",
        named_params! {":version": version},
    )
}

/// Insert command
pub fn insert_command(
    connection: &Connection,
    version: i64,
    command: &Command,
) -> Result<i64, CommandError> {
    let mut statement = connection
        .prepare_cached("INSERT INTO command (version, command) VALUES (:version, :command)")?;
    statement.execute(named_params! {
        ":version": version,
        ":command": serde_json::to_string(&command)?,
    })?;

    Ok(connection.last_insert_rowid())
}

/// Insert command
pub fn insert_account(connection: &Connection, account: &Account) -> Result<String, CommandError> {
    let mut sql_statement = connection.prepare_cached("INSERT INTO account (id, number, description, type, parent_id, statement) VALUES (:id, :number, :description, :type, :parent_id, :statement)")?;
    match account.account_type {
        AccountType::Organization { parent_id } => {
            sql_statement.execute(named_params! {
                ":id": account.id.to_string(),
                ":number": account.number,
                ":description": account.description,
                ":type": account.account_type.to_string(),
                ":parent_id": parent_id.map(|ulid| ulid.to_string()).to_sql()?,
                ":statement": Null,
            })?;
        }
        AccountType::OrganizationUnit { parent_id } => {
            sql_statement.execute(named_params! {
                ":id": account.id.to_string(),
                ":number": account.number,
                ":description": account.description,
                ":type": account.account_type.to_string(),
                ":parent_id": parent_id.to_string(),
                ":statement": Null,
            })?;
        }
        AccountType::Category {
            parent_id,
            statement,
        } => {
            sql_statement.execute(named_params! {
                ":id": account.id.to_string(),
                ":number": account.number,
                ":description": account.description,
                ":type": account.account_type.to_string(),
                ":parent_id": parent_id.to_string(),
                ":statement": statement.to_string(),
            })?;
        }
        AccountType::SubAccount { parent_id } => {
            sql_statement.execute(named_params! {
                ":id": account.id.to_string(),
                ":number": account.number,
                ":description": account.description,
                ":type": account.account_type.to_string(),
                ":parent_id": parent_id.to_string(),
                ":statement": Null,
            })?;
        }
    }
    Ok(account.id.to_string())
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version = get_schema_version(conn)?;
    let stmts = &MIGRATIONS[(version as usize)..];
    let mut i: i32 = version;

    if version == MIGRATIONS.len() as i32 {
        println!("db up to date, no migration needed");
        return Ok(());
    }

    for stmt in stmts {
        println!("conn.execute: {}", &stmt);
        let res = conn.execute(stmt, []);
        if res.is_err() {
            println!("migration failed on:\n{}\n{:?}", stmt, res);
            break;
        }

        i += 1;
    }

    set_schema_version(conn, i)?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(crate = "rocket::serde")]
pub enum FinancialStatement {
    BalanceSheet,
    IncomeStatement,
}

impl fmt::Display for FinancialStatement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            FinancialStatement::BalanceSheet => "BalanceSheet",
            FinancialStatement::IncomeStatement => "IncomeStatement",
        })
    }
}

impl FromStr for FinancialStatement {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BalanceSheet" => Ok(FinancialStatement::BalanceSheet),
            "IncomeStatement" => Ok(FinancialStatement::IncomeStatement),
            _ => Err(()),
        }
    }
}

/// *organization -> *category -> *subaccount
/// *organization -> *organizationunit -> *category -> *subaccount
/// *organization -> *category -> *organizationunit -> *subaccount
/// *organization -> *category -> *subaccount -> *organization -> *subaccount
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(crate = "rocket::serde")]
pub enum AccountType {
    Organization {
        parent_id: Option<AccountId>,
    },
    OrganizationUnit {
        parent_id: AccountId,
    },
    Category {
        parent_id: AccountId,
        statement: FinancialStatement,
    },
    SubAccount {
        parent_id: AccountId,
    },
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            AccountType::Organization { .. } => "Organization",
            AccountType::OrganizationUnit { .. } => "OrganizationUnit",
            AccountType::Category { .. } => "Category",
            AccountType::SubAccount { .. } => "SubAccount",
        })
    }
}

// TODO add command ULID and version?
// Commands
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub enum Command<'r> {
    AddAccount {
        #[serde(borrow = "'r")]
        account: Account<'r>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Account<'r> {
    id: AccountId,
    number: u64,
    description: &'r str,
    account_type: AccountType,
}

#[derive(Responder)]
pub enum CommandResponse {
    #[response(status = 201, content_type = "text")]
    CreatedAccount(String),
}

#[derive(Responder)]
pub enum CommandError {
    #[response(status = 500)]
    ServerError(String),
    //    #[response(status = 404)]
    //    NotFound(String),
}

impl From<serde_json::Error> for CommandError {
    fn from(json_error: serde_json::Error) -> Self {
        CommandError::ServerError(json_error.to_string())
    }
}

impl From<rusqlite::Error> for CommandError {
    fn from(rusqlite_error: rusqlite::Error) -> Self {
        CommandError::ServerError(rusqlite_error.to_string())
    }
}

impl From<&std::sync::TryLockError<std::sync::MutexGuard<'_, rusqlite::Connection>>>
    for CommandError
{
    fn from(
        try_lock_error: &std::sync::TryLockError<std::sync::MutexGuard<'_, rusqlite::Connection>>,
    ) -> Self {
        CommandError::ServerError(try_lock_error.to_string())
    }
}

#[get("/ulid")]
fn get_ulid() -> String {
    rusty_ulid::generate_ulid_string()
}

//#[get("/accounts")]
//fn get_accounts() -> Vec<Account<'static>> {
//    todo!()
//}

#[post("/commands", data = "<command>")]
fn post_command(
    connection: &State<Mutex<Connection>>,
    command: Json<Command>,
) -> Result<CommandResponse, CommandError> {
    insert_command(connection.try_lock().as_ref()?, 0, &command.0)?;

    match command.0 {
        Command::AddAccount { account } => {
            debug!("add account: {:?}", &account);
            insert_account(connection.try_lock().as_ref()?, &account);
            Ok(CommandResponse::CreatedAccount(account.id.to_string()))
        }
    }
}

#[launch]
fn rocket() -> _ {
    // Open a new in-memory SQLite database.
    let conn = Connection::open("test.sqlite").expect("open test_db");

    // Initialize the `entries` table in the in-memory database.
    migrate(&conn);

    rocket::build()
        .manage(Mutex::new(conn))
        .mount("/", routes![get_ulid])
        .mount("/", routes![post_command])
}
