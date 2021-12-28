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
    pub fn new() -> Result<Journal, Error> {
        let db = Db::new()?;
        Ok(Journal { db })
    }

    pub fn new_mem() -> Result<Journal, Error> {
        let db = Db::new_mem()?;
        Ok(Journal { db })
    }

    pub fn add(&self, entry: JournalEntry) -> Result<(), Error> {
        // TODO do validations
        self.db.insert_entry(entry)
    }

    pub fn view(&self) -> Result<Vec<JournalEntry>, Error> {
        self.db.select_entries()
    }
}

/// Journal Entry
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    pub version: ApiVersion,
    pub action: Action,
}

impl JournalEntry {
    const DEFAULT_VERSION: ApiVersion = 1;

    fn new(action: Action) -> JournalEntry {
        let id = Ulid::generate();
        let version = JournalEntry::DEFAULT_VERSION;
        JournalEntry {
            id,
            version,
            action,
        }
    }
}

/// Journal Entry Action
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum Action {
    AddAccount { account: Account },
}

type AccountId = Ulid;

// *organization -> *category -> *subaccount
// *organization -> *organizationunit -> *category -> *subaccount
// *organization -> *category -> *organizationunit -> *subaccount
// *organization -> *category -> *subaccount -> *organization -> *subaccount
/// AccountType
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Account {
    pub id: AccountId,
    pub number: u64,
    pub description: String,
    pub account_type: AccountType,
}

impl Account {
    fn new(number: u64, description: String, account_type: AccountType) -> Account {
        let id = Ulid::generate();
        Account {
            id,
            number,
            description,
            account_type,
        }
    }
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
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
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

#[cfg(test)]
mod test {
    use crate::journal::{Account, AccountType, Action, Journal, JournalEntry};

    #[test]
    fn test_add_find() {
        let journal = Journal::new_mem().expect("journal");

        let entry = JournalEntry::new(Action::AddAccount {
            account: Account::new(
                100,
                "Test account".to_string(),
                AccountType::Organization { parent_id: None },
            ),
        });
        journal.add(entry.clone()).unwrap();
        let entries = journal.view().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries.get(0).unwrap(), &entry);
    }
}
