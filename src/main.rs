use log::debug;
use std::io;

use actix_web::{get, middleware, post, web, App, Error as AWError, HttpResponse, HttpServer};

use crate::journal::{Journal, JournalEntry};

mod journal;

type ApiVersion = u16;

#[derive(Debug, Clone)]
pub enum Error {
    Rusqlite(String),
    R2d2(String),
    SerdeJson(String),
    UlidDecoding(rusty_ulid::DecodingError),
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    //std::env::set_var("RUST_LOG", "actix_web=info");

    // access logs are printed with the INFO level so ensure it is enabled by default
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let journal = journal::Journal::new();

    // Start http server
    HttpServer::new(move || {
        App::new()
            // store journal db as Data object
            .data(journal.clone())
            .wrap(middleware::Logger::default())
            .service(generate_ulid)
            .service(add_journal_entry)
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
) -> Result<HttpResponse, AWError> {
    debug!("journal_entry = {:?}", entry.0);
    journal.add(entry.0).unwrap();
    Ok(HttpResponse::Ok().finish())
}
