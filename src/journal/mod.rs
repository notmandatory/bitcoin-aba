use crate::{ApiVersion, Error};
use db::Db;

use bdk::descriptor::ExtendedDescriptor;
use bdk::TransactionDetails;
use rust_decimal::Decimal;
use rusty_ulid::Ulid;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use time::{Date, OffsetDateTime};

mod db;

pub type JournalEntryId = Ulid;

#[derive(Clone)]
pub struct Journal {
    db: Db,
}

impl Journal {
    pub fn new() -> Result<Self, Error> {
        let db = Db::new()?;
        Ok(Journal { db })
    }

    pub fn new_mem() -> Result<Journal, Error> {
        let db = Db::new_mem()?;
        Ok(Journal { db })
    }

    pub fn add(&self, entry: JournalEntry) -> Result<(), Error> {
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

    pub fn new(action: Action) -> Self {
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
    AddCurrency { currency: Currency },
    AddEntity { entity: Entity },
    AddTransaction { transaction: Transaction },
}

/// Account id
pub type AccountId = Ulid;

/// Account number
pub type AccountNumber = u32;

/// Entity id
pub type EntityId = Ulid;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum EntityType {
    Individual,
    Organization,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Entity {
    pub id: EntityId,
    pub entity_type: EntityType,
    pub name: String,
    pub address: Option<String>,
}

impl Entity {
    pub fn new(entity_type: EntityType, name: String, address: Option<String>) -> Self {
        let id = Ulid::generate();
        Entity {
            id,
            entity_type,
            name,
            address,
        }
    }
}

// *organization -> *category -> *subaccount
// *organization -> *organizationunit -> *category -> *subaccount
// *organization -> *category -> *organizationunit -> *subaccount
// *organization -> *category -> *subaccount -> *organization -> *subaccount
/// Account type
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum AccountType {
    Organization {
        parent_id: Option<AccountId>,
        entity_id: EntityId,
    },
    OrganizationUnit {
        parent_id: AccountId,
        entity_id: EntityId,
    },
    Category {
        parent_id: AccountId,
        statement: FinancialStatement,
    },
    LedgerAccount {
        parent_id: AccountId,
        currency_id: CurrencyId,
    },
    EquityAccount {
        parent_id: AccountId,
        currency_id: CurrencyId,
        entity_id: EntityId,
    },
    BankAccount {
        parent_id: AccountId,
        currency_id: CurrencyId,
        entity_id: EntityId,
        routing: u32,
        account: u64,
    },
    BitcoinAccount {
        parent_id: AccountId,
        descriptor: ExtendedDescriptor,
        change_descriptor: Option<ExtendedDescriptor>,
    },
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            AccountType::Organization { .. } => "Organization",
            AccountType::OrganizationUnit { .. } => "OrganizationUnit",
            AccountType::Category { .. } => "Category",
            AccountType::LedgerAccount { .. } => "LedgerAccount",
            AccountType::EquityAccount { .. } => "EquityAccount",
            AccountType::BankAccount { .. } => "BankAccount",
            AccountType::BitcoinAccount { .. } => "BitcoinAccount",
        })
    }
}

/// Account
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Account {
    pub id: AccountId,
    pub number: AccountNumber,
    pub description: String,
    pub account_type: AccountType,
}

impl Account {
    pub fn new(number: AccountNumber, description: String, account_type: AccountType) -> Self {
        let id = Ulid::generate();
        Account {
            id,
            number,
            description,
            account_type,
        }
    }
}

/// Currency id ie. ISO 4217, use > 1000 for non-ISO 4217 currencies like BTC
pub type CurrencyId = u32;

/// Currency scale
pub type CurrencyScale = u32;

/// Units for a fiat currency value, ie. USD, EUR
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Currency {
    pub id: CurrencyId,
    pub code: String,
    pub scale: CurrencyScale,
    pub name: String,
}

/// Transaction id
pub type TransactionId = Ulid;

/// debits => cash in to account, credits <= cash out of account
/// Transaction represents a "balanced" set of debit and credit account values
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Transaction {
    pub id: TransactionId,
    pub datetime: OffsetDateTime,
    pub description: String,
    pub debits: Vec<AccountValue>,
    pub credits: Vec<AccountValue>,
}

impl Transaction {
    pub fn new(
        datetime: OffsetDateTime,
        description: String,
        debits: Vec<AccountValue>,
        credits: Vec<AccountValue>,
    ) -> Self {
        let id = Ulid::generate();
        Transaction {
            id,
            datetime,
            description,
            debits,
            credits,
        }
    }
}

pub enum PaymentMethod {
    Bitcoin,
    Ach,
    Check,
    Cash,
}

pub enum PaymentTerms {
    ImmediatePayment,
    PaymentInAdvance,
    NetDays {
        days: u32,
        late_fee_interest: Decimal,
    },
    NetDaysDiscount {
        days: u32,
        discount_days: u32,
        discount: Decimal,
        late_fee_interest: Decimal,
    },
}

pub enum Payment {
    Bitcoin {
        details: TransactionDetails,
    },
    Lightning {
        details: String,
    },
    Ach {
        transaction_id: String,
        datetime: OffsetDateTime,
        currency_id: CurrencyId,
        amount: Decimal,
        memo: String,
    },
    Check {
        check_number: u32,
        check_routing: u32,
        check_account: u32,
        date: Date,
        currency_id: CurrencyId,
        amount: Decimal,
        memo: String,
    },
    Cash {
        date: Date,
        currency_id: CurrencyId,
        amount: Decimal,
    },
}

pub enum TransactionType {
    Invoice {
        payment_method: PaymentMethod,
        payment_terms: PaymentTerms,
        payments: Vec<Payment>,
    },
    LedgerAdjustment,
}

/// Account, currency and value of a debit or credit operation
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct AccountValue {
    pub account_id: AccountId,
    pub currency_id: CurrencyId,
    pub amount: Decimal,
    pub description: Option<String>,
}

/// Financial Statement
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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
    //use log::debug;
    use crate::journal::Action::AddAccount;
    use crate::journal::{
        Account, AccountType, AccountValue, Action, Currency, Entity, EntityType,
        FinancialStatement, Journal, JournalEntry, Transaction,
    };
    use rust_decimal::Decimal;
    use time::macros::datetime;

    #[test]
    fn test_add_view() {
        let journal = Journal::new_mem().expect("journal");
        let test_entries = test_entries();
        for entry in &test_entries {
            journal.add(entry.clone()).unwrap();
        }
        let entries_view = journal.view().unwrap();
        assert_eq!(entries_view.len(), test_entries.len());
        for index in 0..test_entries.len() {
            assert_eq!(
                &entries_view.get(index).unwrap(),
                &test_entries.get(index).unwrap()
            );
        }
    }

    // utility functions

    pub fn test_entries() -> Vec<JournalEntry> {
        // Currencies
        let usd = Currency {
            id: 840,
            code: "USD".to_string(),
            scale: 2,
            name: Some("US Dollars".to_string()),
            description: Some("US Dollar Reserve Notes".to_string()),
        };

        // Entities
        let org1 = Entity::new(EntityType::Organization, "Test Co.".to_string(), None);
        let owner1 = Entity::new(EntityType::Individual, "Owner One".to_string(), None);
        let bank1 = Entity::new(EntityType::Organization, "Test Bank".to_string(), None);

        // COA entries
        let org_acct = Account::new(
            10,
            "Test Organization".to_string(),
            AccountType::Organization {
                parent_id: None,
                entity_id: org1.id,
            },
        );
        let assets_acct = Account::new(
            100,
            "Assets".to_string(),
            AccountType::Category {
                statement: FinancialStatement::BalanceSheet,
                parent_id: org_acct.id,
            },
        );
        let liabilities_acct = Account::new(
            200,
            "Liabilities".to_string(),
            AccountType::Category {
                statement: FinancialStatement::BalanceSheet,
                parent_id: org_acct.id,
            },
        );
        let equity_acct = Account::new(
            300,
            "Equity".to_string(),
            AccountType::Category {
                statement: FinancialStatement::BalanceSheet,
                parent_id: org_acct.id,
            },
        );
        let revenue_acct = Account::new(
            400,
            "Revenue".to_string(),
            AccountType::Category {
                statement: FinancialStatement::IncomeStatement,
                parent_id: org_acct.id,
            },
        );
        let expenses_acct = Account::new(
            500,
            "Expenses".to_string(),
            AccountType::Category {
                statement: FinancialStatement::IncomeStatement,
                parent_id: org_acct.id,
            },
        );
        let owner1_acct = Account::new(
            100,
            "Owner 1".to_string(),
            AccountType::EquityAccount {
                parent_id: equity_acct.id,
                currency_id: usd.id,
                entity_id: owner1.id,
            },
        );
        let bank_checking_acct = Account::new(
            100,
            "Bank Checking".to_string(),
            AccountType::BankAccount {
                parent_id: assets_acct.id,
                currency_id: usd.id,
                entity_id: bank1.id,
                routing: 11111,
                account: 123123123123,
            },
        );
        let office_supp_acct = Account::new(
            100,
            "Office Supplies".to_string(),
            AccountType::LedgerAccount {
                parent_id: expenses_acct.id,
                currency_id: usd.id,
            },
        );
        let accounts = vec![
            org_acct,
            assets_acct,
            liabilities_acct,
            equity_acct,
            revenue_acct,
            expenses_acct,
            owner1_acct.clone(),
            bank_checking_acct.clone(),
            office_supp_acct,
        ];

        let mut entries: Vec<JournalEntry> = accounts
            .iter()
            .map(|a| JournalEntry::new(AddAccount { account: a.clone() }))
            .collect();

        // Test transaction entry

        let debits = vec![AccountValue {
            account_id: bank_checking_acct.id.clone(),
            currency_id: usd.id,
            amount: Decimal::new(10_000_00, usd.scale), // USD 10,000.00
            description: Some("Owner funds deposited to bank".to_string()),
        }];

        let credits = vec![AccountValue {
            account_id: owner1_acct.id.clone(),
            currency_id: usd.id,
            amount: Decimal::new(10_000_00, usd.scale), // USD 10,000.00
            description: Some("Equity credited to owner".to_string()),
        }];

        let datetime = datetime!(2022-01-03 09:00 UTC);
        let funding_tx = Transaction::new(
            datetime,
            "Owner's initial funding".to_string(),
            debits,
            credits,
        );

        let usd_entry = JournalEntry::new(Action::AddCurrency { currency: usd });
        entries.push(usd_entry);

        let transaction_entry = JournalEntry::new(Action::AddTransaction {
            transaction: funding_tx,
        });
        entries.push(transaction_entry);

        // sort entries by journal entry ids
        entries.sort_by(|je1, je2| je1.id.cmp(&je2.id));
        entries
    }
}
