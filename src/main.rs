#[macro_use]
extern crate rocket;

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::serde::uuid::Uuid;
use rocket::response::Responder;
use rocket::{Request, response};
use crate::CommandError::NotFound;
use std::sync::Mutex;
use rusqlite::{named_params, Connection};

static MIGRATIONS: &[&str] = &[
    "CREATE TABLE version (version INTEGER)",
    "INSERT INTO version VALUES (1)",
    // "CREATE TABLE script_pubkeys (keychain TEXT, child INTEGER, script BLOB);",
    // "CREATE INDEX idx_keychain_child ON script_pubkeys(keychain, child);",
    // "CREATE INDEX idx_script ON script_pubkeys(script);",
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

// fn init_database(conn: &Connection) {
//     conn.execute("CREATE TABLE entries (
//                   id              INTEGER PRIMARY KEY,
//                   name            TEXT NOT NULL
//                   )", &[])
//         .expect("create entries table");
//
//     conn.execute("INSERT INTO entries (id, name) VALUES ($1, $2)",
//                  &[&0, &"Rocketeer"])
//         .expect("insert single entry into entries table");
// }

pub enum Error {
    Rusqlite(rusqlite::Error)
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

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version = get_schema_version(conn)?;
    let stmts = &MIGRATIONS[(version as usize)..];
    let mut i: i32 = version;

    if version == MIGRATIONS.len() as i32 {
        println!("db up to date, no migration needed");
        return Ok(());
    }

    for stmt in stmts {
        println!("conn.execute({}", &stmt);
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
enum FinancialStatement {
    BalanceSheet,
    IncomeStatement,
}

/// *organization -> *category -> *subaccount
/// *organization -> *organizationunit -> *category -> *subaccount
/// *organization -> *category -> *organizationunit -> *subaccount
/// *organization -> *category -> *subaccount -> *organization -> *subaccount
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(crate = "rocket::serde")]
enum AccountType {
    Organization {
        parent_account: Option<u64>,
    },
    OrganizationUnit {
        parent_account: u64,
    },
    Category {
        statement: FinancialStatement,
        parent_account: u64,
    },
    SubAccount {
        parent_account: u64,
    },
}

// Commands
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
enum Command<'r> {
    AddAccount {
        number: u64,
        description: &'r str,
        account_type: AccountType,
    }
}

#[derive(Serialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Account<'r> {
    uuid: Uuid,
    number: u64,
    description: &'r str,
    account_type: AccountType,
}

#[derive(Serialize, Debug)]
#[serde(crate = "rocket::serde")]
struct ResourceUuid {
    uuid: Uuid,
}

impl<'r> Responder<'r, 'static> for ResourceUuid {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        Json(self).respond_to(req)
    }
}

#[derive(Responder)]
enum CommandResponse {
    #[response(status = 201, content_type = "json")]
    Created(ResourceUuid)
}

#[derive(Responder)]
enum CommandError<'r> {
    #[response(status = 500)]
    Server(&'r str),
    #[response(status = 404)]
    NotFound(&'r str),
}

#[post("/commands", data = "<command>")]
fn post_command(command: Json<Command<'_>>) -> Result<CommandResponse, CommandError> {
    match command.0 {
        Command::AddAccount { number, description, account_type } => {
            let account = Account {
                uuid: Uuid::new_v4(),
                number,
                description,
                account_type,
            };

            println!("add account: {:?}", account);
            Ok(CommandResponse::Created(ResourceUuid { uuid: account.uuid }))
        }
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    // Open a new in-memory SQLite database.
    let conn = Connection::open("test_db").expect("open test_db");

    // Initialize the `entries` table in the in-memory database.
    migrate(&conn);

    rocket::build()
        .manage(Mutex::new(conn))
        .mount("/", routes![index])
        .mount("/", routes![post_command])
}
