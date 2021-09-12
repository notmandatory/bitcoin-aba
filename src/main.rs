#[macro_use]
extern crate rocket;

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::serde::uuid::Uuid;
use rocket::response::Responder;
use rocket::{Request, response};
use crate::CommandError::NotFound;

// // The type to represent the ID of a message.
// type Id = usize;
//
// // We're going to store all of the messages here. No need for a DB.
// type MessageList = Mutex<Vec<String>>;
// type Messages<'r> = &'r State<MessageList>;

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
enum Command<'r>{
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
    uuid: Uuid
}

impl <'r>Responder<'r, 'static> for ResourceUuid {
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
        Command::AddAccount { number, description, account_type} => {
            let account = Account {
                uuid: Uuid::new_v4(),
                number,
                description,
                account_type,};

            println!("add account: {:?}", account);
            Ok(CommandResponse::Created(ResourceUuid { uuid: account.uuid }))
        },
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![post_command])
}
