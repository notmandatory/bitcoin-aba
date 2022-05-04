use log::debug;
use std::fmt::{Display, Formatter};
use std::io;
use std::sync::Mutex;

use actix_web::{
    get, middleware, post, web, App, Error as AWError, HttpResponse, HttpServer, Responder,
    ResponseError,
};

use crate::journal::{Journal, JournalEntry};
use crate::ledger::Ledger;

mod journal;
mod ledger;

type ApiVersion = u16;

#[derive(Debug, Clone)]
pub enum Error {
    Rusqlite(String),
    R2d2(String),
    SerdeJson(String),
    UlidDecoding(rusty_ulid::DecodingError),
    MissingAccount(journal::AccountId),
    MissingCurrency(journal::CurrencyId),
    MissingEntity(journal::EntityId),
    MissingTransaction(journal::TransactionId),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rusqlite(s) => write!(f, "rusqlite: {}", s),
            Self::R2d2(s) => write!(f, "r2d2: {}", s),
            Self::SerdeJson(s) => write!(f, "serde json: {}", s),
            Self::UlidDecoding(d) => write!(f, "ulid decode: {}", d),
            Self::MissingAccount(a) => write!(f, "missing account: {}", a),
            Self::MissingCurrency(c) => write!(f, "missing currency: {}", c),
            Self::MissingEntity(e) => write!(f, "missing entity: {}", e),
            Self::MissingTransaction(t) => write!(f, "missing transaction: {}", t),
        }
    }
}

impl ResponseError for Error {}

#[actix_web::main]
async fn main() -> io::Result<()> {
    //std::env::set_var("RUST_LOG", "actix_web=info");

    // access logs are printed with the INFO level so ensure it is enabled by default
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let journal = journal::Journal::new().expect("new journal");
    let mut ledger = ledger::Ledger::new();
    ledger.load_journal(&journal).expect("loaded journal");

    let journal_data_mutex = web::Data::new(Mutex::new(journal));
    let ledger_data_mutex = web::Data::new(Mutex::new(ledger));

    // Start http server
    HttpServer::new(move || {
        App::new()
            // store journal db as Data object
            .app_data(journal_data_mutex.clone())
            .app_data(ledger_data_mutex.clone())
            .wrap(middleware::Logger::default())
            .service(generate_ulid)
            .service(add_journal_entry)
            .service(view_journal_entries)
            .service(view_ledger_accounts)
            .service(view_ledger_currencies)
            .service(view_ledger_entities)
            .service(view_ledger_transactions)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

/// Generate a new ulid
#[get("/ulid")]
pub(crate) async fn generate_ulid() -> Result<HttpResponse, AWError> {
    let ulid = rusty_ulid::generate_ulid_string();
    Ok(HttpResponse::Ok().body(ulid))
}

/// Create a journal entry
#[post("/journal")]
async fn add_journal_entry(
    journal: web::Data<Mutex<Journal>>,
    ledger: web::Data<Mutex<Ledger>>,
    entry: web::Json<JournalEntry>,
) -> Result<impl Responder, AWError> {
    debug!("update ledger");
    ledger.lock().unwrap().add_journal_entry(entry.0.clone())?;
    debug!("add new journal entry = {:?}", entry.0);
    journal.lock().unwrap().add(entry.0).unwrap();
    Ok(web::HttpResponse::Ok())
}

#[get("/journal")]
async fn view_journal_entries(
    journal: web::Data<Mutex<Journal>>,
) -> Result<impl Responder, AWError> {
    debug!("view journal before DB");
    let journal_view = journal.lock().unwrap().view()?;
    debug!("view journal entries: {:?}", journal_view);
    Ok(web::Json(journal_view))
}

#[get("/ledger/accounts")]
async fn view_ledger_accounts(ledger: web::Data<Mutex<Ledger>>) -> Result<impl Responder, AWError> {
    let accounts_view = ledger.lock().unwrap().accounts();
    Ok(web::Json(accounts_view))
}

#[get("/ledger/currencies")]
async fn view_ledger_currencies(
    ledger: web::Data<Mutex<Ledger>>,
) -> Result<impl Responder, AWError> {
    let currencies_view = ledger.lock().unwrap().currencies();
    Ok(web::Json(currencies_view))
}

#[get("/ledger/entities")]
async fn view_ledger_entities(ledger: web::Data<Mutex<Ledger>>) -> Result<impl Responder, AWError> {
    let entities_view = ledger.lock().unwrap().entities();
    Ok(web::Json(entities_view))
}

#[get("/ledger/transactions")]
async fn view_ledger_transactions(
    ledger: web::Data<Mutex<Ledger>>,
) -> Result<impl Responder, AWError> {
    let transactions_view = ledger.lock().unwrap().transactions();
    Ok(web::Json(transactions_view))
}
