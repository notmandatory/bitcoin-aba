use std::{io, fmt};

use actix_web::{middleware, web, App, Error as AWError, HttpResponse, HttpServer};
use r2d2_sqlite::{self, SqliteConnectionManager};
use rusty_ulid::Ulid;
use serde::{Deserialize, Serialize};

mod db;
use db::Pool;
use std::str::FromStr;

type AccountId = Ulid;
type JournalEntryId = Ulid;

#[derive(Debug)]
pub enum Error {
    Rusqlite(rusqlite::Error),
}

//// Insert command
//fn insert_command(
//    conn: &rusqlite::Connection,
//    version: i64,
//    id: CommandId,
//    command: Command,
//) -> Result<(), CommandError> {
//    let command_json = serde_json::to_string(&command).unwrap();
//    let mut statement = conn.prepare_cached(
//        "INSERT INTO command_log (id, version, command) VALUES (:id, :version, :command)",
//    )?;
//    statement.execute(named_params! {
//        ":id": id.to_string(),
//        ":version": version, // TODO remove version?
//        ":command": command_json,
//    })?;
//    Ok(())
//}
//
///// Insert account
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
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

// *organization -> *category -> *subaccount
// *organization -> *organizationunit -> *category -> *subaccount
// *organization -> *category -> *organizationunit -> *subaccount
// *organization -> *category -> *subaccount -> *organization -> *subaccount
#[derive(Serialize, Deserialize, Debug, Clone)]
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

// Journal Entry
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum JournalEntry {
    AddAccount {
        //#[serde(borrow = "'r")]
        account: Account,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    id: AccountId,
    number: u64,
    description: String,
    account_type: AccountType,
}

//#[derive(Responder, Debug)]
//pub enum CommandResponse {
//    #[response(status = 202)]
//    CommandAccepted(String),
//}
//
//#[derive(Responder, Debug)]
//pub enum CommandError {
//    #[response(status = 500)]
//    ServerError(String),
//    //    #[response(status = 404)]
//    //    NotFound(String),
//}

//impl From<serde_json::Error> for CommandError {
//    fn from(json_error: serde_json::Error) -> Self {
//        CommandError::ServerError(json_error.to_string())
//    }
//}

//impl From<rocket_sync_db_pools::rusqlite::Error> for CommandError {
//    fn from(rusqlite_error: rocket_sync_db_pools::rusqlite::Error) -> Self {
//        CommandError::ServerError(rusqlite_error.to_string())
//    }
//}
//
//impl From<&std::sync::TryLockError<std::sync::MutexGuard<'_, rusqlite::Connection>>>
//    for CommandError
//{
//    fn from(
//        try_lock_error: &std::sync::TryLockError<std::sync::MutexGuard<'_, rusqlite::Connection>>,
//    ) -> Self {
//        CommandError::ServerError(try_lock_error.to_string())
//    }
//}
//
//#[get("/ulid")]
//async fn get_ulid() -> String {
//    rusty_ulid::generate_ulid_string()
//}
//
////#[get("/accounts")]
////fn get_accounts() -> Vec<Account<'static>> {
////    todo!()
////}
//
//#[post("/commands", data = "<command>")]
//async fn post_command(
//    conn: DbConn,
//    command_tx: &State<CommandTx>,
//    command: Json<Command>,
//) -> Result<CommandResponse, CommandError> {
//    //    conn.run(move |c| {
//    //        let id = Ulid::generate();
//    //        let command_json = serde_json::to_string(&command.0).unwrap();
//    //
//    //        let mut statement = c.prepare_cached(
//    //            "INSERT INTO command_log (id, version, command) VALUES (:id, :version, :command)",
//    //        )?;
//    //        statement.execute(named_params! {
//    //            ":id": id.to_string(),
//    //            ":version": 0, // TODO remove version?
//    //            ":command": command_json,
//    //        });
//    //        Ok(CommandAccepted(id.to_string()))
//    //    })
//    //    .await
//    conn.run(move |c| {
//        let id = Ulid::generate();
//        insert_command(c, 0, id, command.0.clone())?;
//
//        command_tx.0.send(command.0);
//        Ok(CommandAccepted(id.to_string()))
//    })
//    .await
//}
//
////use rocket::response::stream::{Event, EventStream};
////use rocket::tokio::time::{self, Duration};
////
////#[get("/events")]
////fn get_events() -> EventStream![] {
////    EventStream! {
////        let mut interval = time::interval(Duration::from_secs(1));
////        loop {
////            yield Event::data("ping");
////            interval.tick().await;
////        }
////    }
////}

//#[launch]
//fn rocket() -> _ {
//    //    // Open a new in-memory SQLite database.
//    //    let conn_mutex = Mutex::new(Connection::open("test.sqlite").expect("open test_db"));
//    //
//    //    // Initialize the `entries` table in the in-memory database.
//    //    migrate(conn_mutex.try_lock().as_ref().unwrap());
//    //
//    //    // setup command channels
//    //    let (command_sender, command_receiver): (flume::Sender<Command>, flume::Receiver<Command>) =
//    //        flume::unbounded();
//    //
//    //    let command_thread = thread::spawn(move || {
//    //        for command in command_receiver.iter() {
//    //            match command {
//    //                Command::AddAccount { account } => {
//    //                    debug!("add account: {:?}", &account);
//    //                    match insert_account(conn_mutex.try_lock().as_ref().unwrap(), &account) {
//    //                        Ok(id) => {
//    //                            println!("Inserted account: {:?}, {}", account, id);
//    //                        }
//    //                        Err(error) => {
//    //                            println!("Failed! inserted account: {:?}, {:?}", account, error);
//    //                        }
//    //                    }
//    //                }
//    //            }
//    //        }
//    //    });
//
//    let (tx, rx) = flume::bounded(32);
//
//    rocket::build()
//        .attach(DbConn::fairing())
//        .attach(db_migrations())
//        .attach(command_queue(tx, rx.clone()))
//        .attach(process_commands(rx))
//        .mount("/", routes![get_ulid])
//        .mount("/", routes![post_command])
//    //        .mount("/", routes![get_events])
//}

// generate ulid
async fn generate_ulid() -> Result<HttpResponse, AWError> {
    let ulid = rusty_ulid::generate_ulid_string();
    Ok(HttpResponse::Ok().body(ulid))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    //std::env::set_var("RUST_LOG", "actix_web=info");

    // access logs are printed with the INFO level so ensure it is enabled by default
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Start N db executor actors (N = number of cores avail)
    let manager = SqliteConnectionManager::file("bitcoin-aba.sqlite");
    let pool = Pool::new(manager).unwrap();
    db::migrations(pool.get().unwrap()).unwrap();

    // Start http server
    HttpServer::new(move || {
        App::new()
            // store db pool as Data object
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/ulid").route(web::get().to(generate_ulid)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
