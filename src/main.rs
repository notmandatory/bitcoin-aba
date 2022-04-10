use std::fmt::{Display, Formatter};
use log::debug;
use std::io;

use actix_web::{get, middleware, post, web, App, Error as AWError, HttpResponse, HttpServer, Responder, ResponseError};
use actix_web::http::StatusCode;

use crate::journal::{Journal, JournalEntry};

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
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rusqlite(s) => write!(f, "rusqlite: {}", s),
            Self::R2d2(s) => write!(f, "r2d2: {}", s),
            Self::SerdeJson(s) => write!(f, "serde json: {}", s),
            Self::UlidDecoding(d) => write!(f, "ulid decode: {}", d.to_string()),
            Self::MissingAccount(a) => write!(f, "missing account: {}", a.to_string()),
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

    // Start http server
    HttpServer::new(move || {
        App::new()
            // store journal db as Data object
            .app_data(web::Data::new(journal.clone()))
            .wrap(middleware::Logger::default())
            .service(generate_ulid)
            .service(add_journal_entry)
            .service(view_journal_entries)
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
    journal: web::Data<Journal>,
    entry: web::Json<JournalEntry>,
) -> Result<impl Responder, AWError> {
    debug!("add new journal entry = {:?}", entry.0);
    journal.add(entry.0).unwrap();
    Ok(web::HttpResponse::Ok())
}

#[get("/journal")]
async fn view_journal_entries(
    journal: web::Data<Journal>,
) -> Result<impl Responder, AWError> {
    debug!("view journal before DB");
    let journal_view = journal.view()?;
    debug!("view journal entries: {:?}", journal_view);
    Ok(web::Json(journal_view))
}
