use log::debug;
use std::fmt::{Display, Formatter};
use std::io;
use std::sync::Mutex;

use actix_web::{
    get, middleware, post, web, App, Error as AWError, HttpResponse, HttpServer, Responder,
    ResponseError,
};

use aba::journal::{test_entries, Journal, JournalEntry};
use aba::ledger::Ledger;
use aba::rusty_ulid;

use aba::journal::sqlite::SqliteDb;

#[cfg(feature = "web-files")]
use actix_web_static_files::ResourceFiles;

#[derive(Debug, Clone)]
pub enum Error {
    Rusqlite(String),
    R2d2(String),
    SerdeJson(String),
    UlidDecoding(rusty_ulid::DecodingError),
    Ledger(aba::ledger::Error),
    Journal(aba::journal::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rusqlite(s) => write!(f, "rusqlite: {}", s),
            Self::R2d2(s) => write!(f, "r2d2: {}", s),
            Self::SerdeJson(s) => write!(f, "serde json: {}", s),
            Self::UlidDecoding(d) => write!(f, "ulid decode: {}", d),
            Self::Ledger(l) => write!(f, "ledger error: {}", l),
            Self::Journal(l) => write!(f, "journal error: {}", l),
        }
    }
}

impl From<aba::ledger::Error> for Error {
    fn from(e: aba::ledger::Error) -> Self {
        Error::Ledger(e)
    }
}

impl ResponseError for Error {}

#[cfg(feature = "web-files")]
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> io::Result<()> {
    //std::env::set_var("RUST_LOG", "actix_web=info");

    // access logs are printed with the INFO level so ensure it is enabled by default
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let db = SqliteDb::new().unwrap();
    let journal = Journal::new(db).expect("new journal");
    let mut ledger = Ledger::new();
    let journal_entries = journal.view().expect("journal entries");
    ledger
        .add_journal_entries(journal_entries)
        .expect("ledger loaded");
    //ledger.load_journal(&journal).expect("loaded journal");

    let journal_data_mutex = web::Data::new(Mutex::new(journal));
    let ledger_data_mutex = web::Data::new(Mutex::new(ledger));

    // Start http server
    HttpServer::new(move || {
        let app = App::new().service(
            web::scope("/api")
                // store journal db as Data object
                .app_data(journal_data_mutex.clone())
                .app_data(ledger_data_mutex.clone())
                .wrap(middleware::Logger::default())
                .service(generate_ulid)
                .service(load_test_journal_entries)
                .service(add_journal_entry)
                .service(view_journal_entries)
                .service(view_ledger_accounts)
                .service(view_ledger_currencies)
                .service(view_ledger_entities)
                .service(view_ledger_transactions),
        );
        #[cfg(feature = "web-files")]
        let app = app.service(ResourceFiles::new("/", generate()));
        app
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
}

/// Generate a new ulid
#[get("/api/ulid")]
pub(crate) async fn generate_ulid() -> Result<HttpResponse, AWError> {
    let ulid = rusty_ulid::generate_ulid_string();
    Ok(HttpResponse::Ok().body(ulid))
}

/// Load test journal entry
#[post("/journal/test")]
async fn load_test_journal_entries(
    journal: web::Data<Mutex<Journal<SqliteDb>>>,
    ledger: web::Data<Mutex<Ledger>>,
) -> Result<impl Responder, AWError> {
    debug!("add test entries to ledger");
    let test_entries = test_entries();
    ledger
        .lock()
        .unwrap()
        .add_journal_entries(test_entries.journal_entries.clone())
        .map_err(|e| Error::from(e))?;
    debug!("add test entries to journal");
    for entry in test_entries.journal_entries {
        journal.lock().unwrap().add(entry).unwrap();
    }
    Ok(HttpResponse::Ok())
}

/// Create a journal entry
#[post("/journal")]
async fn add_journal_entry(
    journal: web::Data<Mutex<Journal<SqliteDb>>>,
    ledger: web::Data<Mutex<Ledger>>,
    entry: web::Json<JournalEntry>,
) -> Result<impl Responder, AWError> {
    debug!("update ledger");
    ledger
        .lock()
        .unwrap()
        .add_journal_entry(entry.0.clone())
        .map_err(|e| Error::from(e))?;
    debug!("add new journal entry = {:?}", entry.0);
    journal.lock().unwrap().add(entry.0).unwrap();
    Ok(HttpResponse::Ok())
}

#[get("/journal")]
async fn view_journal_entries(
    journal: web::Data<Mutex<Journal<SqliteDb>>>,
) -> Result<impl Responder, AWError> {
    debug!("view journal before DB");
    let journal_view = journal
        .lock()
        .unwrap()
        .view()
        .map_err(|e| Error::Journal(e))?;
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
