#[macro_use]
extern crate rocket;

use std::str::FromStr;
use std::time::Duration;
use std::sync::Mutex;
use std::{fmt, thread};

use rocket_sync_db_pools::rusqlite::named_params;
use rocket_sync_db_pools::{database, rusqlite};

use flume::Sender;
use rocket::response::Responder;
use rocket::serde::{json::serde_json, json::Json, Deserialize, Serialize};
use rocket::{response, tokio, Build, Request, Rocket, State};

use rusqlite::ToSql;
use rusty_ulid::Ulid;

use crate::CommandError::ServerError;
use crate::CommandResponse::CommandAccepted;
use rocket::fairing::AdHoc;
use rocket_sync_db_pools::rusqlite::Connection;
use rusqlite::types::Null;

type AccountId = Ulid;
type CommandId = Ulid;

#[database("aba")]
struct DbConn(rusqlite::Connection);

static MIGRATIONS: &[&str] = &[
    "CREATE TABLE version (version INTEGER)",
    "INSERT INTO version VALUES (1)",
    "CREATE TABLE command_log (id TEXT NOT NULL, version INTEGER NOT NULL, command TEXT NOT NULL);",
    "CREATE UNIQUE INDEX idx_command_log_id ON command_log(id);",
    "CREATE TABLE account (id TEXT NOT NULL, number INTEGER NOT NULL, description TEXT NOT NULL, type TEXT NOT NULL, parent_id TEXT, statement TEXT, FOREIGN KEY(parent_id) REFERENCES account(id));",
    "CREATE UNIQUE INDEX idx_account_id ON account(id);",
    "CREATE UNIQUE INDEX idx_account_parent_number ON account(parent_id, number);",
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

pub enum Error {
    Rusqlite(rusqlite::Error),
}

impl std::convert::From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Rusqlite(err)
    }
}

fn process_commands(rx: flume::Receiver<Command>) -> AdHoc {
    AdHoc::on_ignite("Process commands", |rocket| async {
        let conn = DbConn::get_one(&rocket).await.expect("database connection");
//        let command_receiver = rocket
//            .state::<CommandRx>()
//            .expect("Command sender channel").clone();
        
        conn.run(move |c| {
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(15));
                loop {
                    //println!("Processing commands...");
                    //interval.tick().await;
                    let command = rx.recv_async().await.expect("Command");
                    println!("Received command in background: {:?}", command);
                    // TODO handle graceful shutdown with select
                }
            });
        })
        .await;
        rocket
    })
}

fn db_migrations() -> AdHoc {
    AdHoc::on_ignite("DB Migrations", |rocket| async {
        let conn = DbConn::get_one(&rocket).await.expect("database connection");

        conn.run(|c| {
            let version = get_schema_version(&c).expect("Can get schema version");
            let stmts = &MIGRATIONS[(version as usize)..];
            let mut i: i32 = version;

            if version == MIGRATIONS.len() as i32 {
                println!("db up to date, no migration needed");
                return ();
            }

            for stmt in stmts {
                println!("conn.execute: {}", &stmt);
                let res = c.execute(stmt, []);
                if res.is_err() {
                    println!("migration failed on:\n{}\n{:?}", stmt, res);
                    break;
                }

                i += 1;
            }

            set_schema_version(&c, i).expect("Can set schema version");
        })
        .await;
        rocket
    })
}

struct CommandTx(flume::Sender<Command>);
struct CommandRx(flume::Receiver<Command>);

fn command_queue(tx: flume::Sender<Command>, rx: flume::Receiver<Command>) -> AdHoc {
    AdHoc::on_ignite("Command Queue", |rocket| async {
        //let (tx, rx) = flume::bounded(32);
        rocket.manage(CommandTx(tx)).manage(CommandRx(rx))
    })
}

fn get_schema_version(conn: &rusqlite::Connection) -> rusqlite::Result<i32> {
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

fn set_schema_version(conn: &rusqlite::Connection, version: i32) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE version SET version=:version",
        named_params! {":version": version},
    )
}

/// Insert command
fn insert_command(
    conn: &rusqlite::Connection,
    version: i64,
    id: CommandId,
    command: Command,
) -> Result<(), CommandError> {
    let command_json = serde_json::to_string(&command).unwrap();
    let mut statement = conn.prepare_cached(
        "INSERT INTO command_log (id, version, command) VALUES (:id, :version, :command)",
    )?;
    statement.execute(named_params! {
        ":id": id.to_string(),
        ":version": version, // TODO remove version?
        ":command": command_json,
    })?;
    Ok(())
}

/// Insert account
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
#[derive(Serialize, Deserialize, Debug, Clone)]
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

// Commands
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub enum Command {
    AddAccount {
        //#[serde(borrow = "'r")]
        account: Account,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Account {
    id: AccountId,
    number: u64,
    description: String,
    account_type: AccountType,
}

#[derive(Responder, Debug)]
pub enum CommandResponse {
    #[response(status = 202)]
    CommandAccepted(String),
}

#[derive(Responder, Debug)]
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

impl From<rocket_sync_db_pools::rusqlite::Error> for CommandError {
    fn from(rusqlite_error: rocket_sync_db_pools::rusqlite::Error) -> Self {
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
async fn get_ulid() -> String {
    rusty_ulid::generate_ulid_string()
}

//#[get("/accounts")]
//fn get_accounts() -> Vec<Account<'static>> {
//    todo!()
//}

#[post("/commands", data = "<command>")]
async fn post_command(
    conn: DbConn,
    command_tx: &State<CommandTx>,
    command: Json<Command>,
) -> Result<CommandResponse, CommandError> {
    //    conn.run(move |c| {
    //        let id = Ulid::generate();
    //        let command_json = serde_json::to_string(&command.0).unwrap();
    //
    //        let mut statement = c.prepare_cached(
    //            "INSERT INTO command_log (id, version, command) VALUES (:id, :version, :command)",
    //        )?;
    //        statement.execute(named_params! {
    //            ":id": id.to_string(),
    //            ":version": 0, // TODO remove version?
    //            ":command": command_json,
    //        });
    //        Ok(CommandAccepted(id.to_string()))
    //    })
    //    .await
    conn.run(move |c| {
        let id = Ulid::generate();
        insert_command(c, 0, id, command.0.clone())?;

        command_tx.0.send(command.0);
        Ok(CommandAccepted(id.to_string()))
    })
    .await
}

//use rocket::response::stream::{Event, EventStream};
//use rocket::tokio::time::{self, Duration};
//
//#[get("/events")]
//fn get_events() -> EventStream![] {
//    EventStream! {
//        let mut interval = time::interval(Duration::from_secs(1));
//        loop {
//            yield Event::data("ping");
//            interval.tick().await;
//        }
//    }
//}

#[launch]
fn rocket() -> _ {
    //    // Open a new in-memory SQLite database.
    //    let conn_mutex = Mutex::new(Connection::open("test.sqlite").expect("open test_db"));
    //
    //    // Initialize the `entries` table in the in-memory database.
    //    migrate(conn_mutex.try_lock().as_ref().unwrap());
    //
    //    // setup command channels
    //    let (command_sender, command_receiver): (flume::Sender<Command>, flume::Receiver<Command>) =
    //        flume::unbounded();
    //
    //    let command_thread = thread::spawn(move || {
    //        for command in command_receiver.iter() {
    //            match command {
    //                Command::AddAccount { account } => {
    //                    debug!("add account: {:?}", &account);
    //                    match insert_account(conn_mutex.try_lock().as_ref().unwrap(), &account) {
    //                        Ok(id) => {
    //                            println!("Inserted account: {:?}, {}", account, id);
    //                        }
    //                        Err(error) => {
    //                            println!("Failed! inserted account: {:?}, {:?}", account, error);
    //                        }
    //                    }
    //                }
    //            }
    //        }
    //    });

    let (tx, rx) = flume::bounded(32);

    rocket::build()
        .attach(DbConn::fairing())
        .attach(db_migrations())
        .attach(command_queue(tx, rx.clone()))
        .attach(process_commands(rx))
        .mount("/", routes![get_ulid])
        .mount("/", routes![post_command])
    //        .mount("/", routes![get_events])
}
