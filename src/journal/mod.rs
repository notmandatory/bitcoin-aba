use crate::{ApiVersion, Error};
use db::Db;

use rusty_ulid::Ulid;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

mod db;

pub type JournalEntryId = Ulid;

#[derive(Clone)]
pub struct Journal {
    db: Db,
}

impl Journal {
    pub fn new() -> Journal {
        let db = Db::new().expect("journal db");
        Journal { db }
    }

    pub fn add(&self, entry: JournalEntry) -> Result<(), Error> {
        // TODO do validations
        self.db.insert_entry(entry)
    }
}

/// Journal Entry
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    pub version: ApiVersion,
    pub action: Action,
}

/// Journal Entry Action
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Action {
    AddAccount { account: Account },
}

type AccountId = Ulid;

// *organization -> *category -> *subaccount
// *organization -> *organizationunit -> *category -> *subaccount
// *organization -> *category -> *organizationunit -> *subaccount
// *organization -> *category -> *subaccount -> *organization -> *subaccount
/// AccountType
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

/// Account
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub id: AccountId,
    pub number: u64,
    pub description: String,
    pub account_type: AccountType,
}

///// List accounts
//#[get("/account")]
//pub(crate) async fn list_accounts(
//    pool: web::Data<Pool>,
//) -> Result<HttpResponse, AWError> {
//    let conn = pool.get().unwrap();
//    debug!("journal_entry = {:?}", entry.0);
//    insert_journal_entry(&conn, entry.0).unwrap();
//    Ok(HttpResponse::Ok().finish())
//}

/// Financial Statement
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
