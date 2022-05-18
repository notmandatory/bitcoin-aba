use crate::journal::Action::{
    AddAccount, AddContact, AddCurrency, AddOrganization, AddTransaction,
};
use crate::journal::CurrencyCode::{BTC, USD};
use rust_decimal::Decimal;
use rusty_ulid::Ulid;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt;
use std::fmt::{Display, Formatter};
use time::macros::datetime;
use time::{Date, OffsetDateTime};

#[cfg(feature = "server")]
pub mod sqlite;

#[derive(Debug, Clone)]
pub enum Error {
    Db(String),
    UlidDecoding(String),
    SerdeJson(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db(a) => write!(f, "database: {}", a),
            Self::UlidDecoding(a) => write!(f, "ulid decoding: {}", a),
            Self::SerdeJson(a) => write!(f, "serde json: {}", a),
        }
    }
}

/// DB

pub trait Db {
    // Insert entry
    fn insert_entry(&mut self, entry: JournalEntry) -> Result<(), Error>;

    // Select entries
    fn select_entries(&self) -> Result<Vec<JournalEntry>, Error>;
}

pub struct VecDb {
    db: Vec<JournalEntry>,
}

impl VecDb {
    pub fn new() -> Self {
        Self { db: Vec::new() }
    }
}

impl Db for VecDb {
    fn insert_entry(&mut self, entry: JournalEntry) -> Result<(), Error> {
        Ok(self.db.push(entry))
    }

    fn select_entries(&self) -> Result<Vec<JournalEntry>, Error> {
        let entries = self.db.iter().cloned().collect();
        Ok(entries)
    }
}

/// Journal

#[derive(Clone)]
pub struct Journal<D>
where
    D: Db,
{
    db: RefCell<D>,
}

impl<D> Journal<D>
where
    D: Db,
{
    pub fn new(db: D) -> Self {
        //let db = Db::new()?;
        Journal {
            db: RefCell::new(db),
        }
    }

    pub fn add(&self, entry: JournalEntry) -> Result<(), Error> {
        self.db.borrow_mut().insert_entry(entry)
    }

    pub fn view(&self) -> Result<Vec<JournalEntry>, Error> {
        self.db.borrow().select_entries()
    }
}

/// Journal Entry

pub type JournalEntryId = Ulid;
pub type ApiVersion = u16;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    pub version: ApiVersion,
    pub organization_id: OrganizationId,
    pub action: Action,
}

impl JournalEntry {
    pub const DEFAULT_VERSION: ApiVersion = 1;

    pub fn new(id: JournalEntryId, organization_id: OrganizationId, action: Action) -> Self {
        let version = JournalEntry::DEFAULT_VERSION;
        JournalEntry {
            id,
            version,
            organization_id,
            action,
        }
    }

    pub fn new_gen_id(organization_id: OrganizationId, action: Action) -> Self {
        let id = Ulid::generate();
        JournalEntry::new(id, organization_id, action)
    }

    pub fn new_after_id(
        previous_id: JournalEntryId,
        organization_id: OrganizationId,
        action: Action,
    ) -> Self {
        let id = Ulid::next_monotonic(previous_id);
        JournalEntry::new(id, organization_id, action)
    }
}

/// Journal Entry Action
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum Action {
    AddOrganization {
        contact: Contact,
        organization: Organization,
    },
    AddCurrency {
        currency: Currency,
    },
    AddContact {
        contact: Contact,
    },
    AddAccount {
        account: Account,
    },
    AddTransaction {
        transaction: Transaction,
        ledger_entries: Vec<LedgerEntry>,
    },
}

/// Organization id
pub type OrganizationId = Ulid;

/// Account id
pub type AccountId = Ulid;

/// Account number
pub type AccountNumber = u32;

/// Contact id
pub type ContactId = Ulid;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Organization {
    pub id: OrganizationId,
    pub contact_id: ContactId,
}
impl Organization {
    pub fn new(contact_id: &ContactId) -> Self {
        let id = Ulid::generate();
        let contact_id = contact_id.clone();
        Organization { id, contact_id }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ContactType {
    Individual,
    Organization,
    OrganizationUnit,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Contact {
    pub id: ContactId,
    pub contact_type: ContactType,
    pub name: String,
    pub address: Option<String>,
}

impl Contact {
    pub fn new(contact_type: ContactType, name: String, address: Option<String>) -> Self {
        let id = Ulid::generate();
        Contact {
            id,
            contact_type,
            name,
            address,
        }
    }
}

/// Account type
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum AccountType {
    LedgerAccount,
    ContactAccount {
        contact_id: ContactId,
    },
    BankAccount {
        currency_id: CurrencyId,
        routing: u32,
        account: u64,
    },
    BitcoinAccount {
        descriptor: String,
        change_descriptor: Option<String>,
    },
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            AccountType::LedgerAccount { .. } => "LedgerAccount",
            AccountType::ContactAccount { .. } => "ContactAccount",
            AccountType::BankAccount { .. } => "BankAccount",
            AccountType::BitcoinAccount { .. } => "BitcoinAccount",
        })
    }
}

/// Account
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Account {
    pub id: AccountId,
    pub parent_id: Option<AccountId>,
    pub number: AccountNumber,
    pub description: String,
    pub account_type: AccountType,
    pub account_category: AccountCategory,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum BalanceSheetCategory {
    Asset,
    Liability,
    Equity,
}

impl fmt::Display for BalanceSheetCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            BalanceSheetCategory::Asset => "Asset",
            BalanceSheetCategory::Liability => "Liability",
            BalanceSheetCategory::Equity => "Equity",
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum IncomeStatementCategory {
    OperatingRevenue,
    OperatingExpense,
    NonOperatingRevenue,
    NonOperatingExpense,
}

impl fmt::Display for IncomeStatementCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            IncomeStatementCategory::OperatingRevenue => "OperatingRevenue",
            IncomeStatementCategory::OperatingExpense => "OperatingExpense",
            IncomeStatementCategory::NonOperatingRevenue => "NonOperatingRevenue",
            IncomeStatementCategory::NonOperatingExpense => "NonOperatingExpense",
        })
    }
}

/// Financial Statement
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum AccountCategory {
    BalanceSheet(BalanceSheetCategory),
    IncomeStatement(IncomeStatementCategory),
}

impl fmt::Display for AccountCategory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            AccountCategory::BalanceSheet(_) => "BalanceSheet",
            AccountCategory::IncomeStatement(_) => "IncomeStatement",
        })
    }
}

impl Account {
    pub fn new(
        parent_id: Option<&AccountId>,
        number: AccountNumber,
        description: String,
        account_type: AccountType,
        account_category: AccountCategory,
    ) -> Self {
        let id = Ulid::generate();
        let parent_id = parent_id.cloned();
        Account {
            id,
            parent_id,
            number,
            description,
            account_type,
            account_category,
        }
    }
}

/// Currency id ie. ISO 4217, use > 2000 for non-ISO 4217 currencies like BTC
pub type CurrencyId = u32;

pub enum CurrencyCode {
    USD = 840,
    BTC = 2009,
}

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

/// Ledger entry types
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum EntryType {
    Debit,
    Credit,
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
    pub transaction_type: TransactionType,
}

impl Transaction {
    pub fn new(
        datetime: OffsetDateTime,
        description: String,
        transaction_type: TransactionType,
    ) -> Self {
        let id = Ulid::generate();
        Transaction {
            id,
            datetime,
            description,
            transaction_type,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum PaymentMethod {
    Bitcoin {
        address: String,
    },
    Ach {
        contact_id: ContactId,
        currency_id: CurrencyId,
        routing: u32,
        account: u64,
    },
    Check {
        contact_id: ContactId,
        currency_id: CurrencyId,
    },
    Cash,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum Payment {
    Bitcoin {
        details: String,
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

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum TransactionType {
    Invoice {
        payment_method: PaymentMethod,
        payment_terms: PaymentTerms,
        payments: Vec<Payment>,
    },
    LedgerAdjustment,
}

/// Account and currency amount of a debit or credit ledger entry
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct LedgerEntry {
    pub transaction_id: TransactionId,
    pub entry_type: EntryType,
    pub account_id: AccountId,
    pub currency_amount: CurrencyAmount,
    pub description: Option<String>,
}

impl LedgerEntry {
    pub fn new(
        transaction_id: &TransactionId,
        entry_type: EntryType,
        account_id: &AccountId,
        currency_amount: CurrencyAmount,
        description: Option<String>,
    ) -> Self {
        let transaction_id = transaction_id.clone();
        let account_id = account_id.clone();
        LedgerEntry {
            transaction_id,
            entry_type,
            account_id,
            currency_amount,
            description,
        }
    }
}

/// Currency and amount of a debit or credit
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct CurrencyAmount {
    pub currency_id: CurrencyId,
    pub amount: Decimal,
}

impl CurrencyAmount {
    pub fn new(currency_id: &CurrencyId, amount: Decimal) -> Self {
        let currency_id = currency_id.clone();
        CurrencyAmount {
            currency_id,
            amount,
        }
    }
}

// utility functions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestEntries {
    pub organization_contact: Contact,
    pub organization: Organization,
    pub contacts: Vec<Contact>,
    pub currencies: Vec<Currency>,
    pub accounts: Vec<Account>,
    pub transactions: Vec<(Transaction, Vec<LedgerEntry>)>,
    pub journal_entries: Vec<JournalEntry>,
}

impl TestEntries {
    fn new(
        organization_contact: Contact,
        organization: Organization,
        contacts: Vec<Contact>,
        currencies: Vec<Currency>,
        accounts: Vec<Account>,
        transactions: Vec<(Transaction, Vec<LedgerEntry>)>,
    ) -> Self {
        let mut journal_entries: Vec<JournalEntry> = Vec::new();

        let action = AddOrganization {
            contact: organization_contact.clone(),
            organization: organization.clone(),
        };
        TestEntries::add_journal_entry(&mut journal_entries, &organization.id, action);

        for contact in contacts.clone() {
            let action = AddContact {
                contact: contact.clone(),
            };
            TestEntries::add_journal_entry(&mut journal_entries, &organization.id, action)
        }

        for currency in currencies.clone() {
            let action = AddCurrency {
                currency: currency.clone(),
            };
            TestEntries::add_journal_entry(&mut journal_entries, &organization.id, action)
        }
        for account in accounts.clone() {
            let action = AddAccount {
                account: account.clone(),
            };
            TestEntries::add_journal_entry(&mut journal_entries, &organization.id, action)
        }
        for transaction in transactions.iter().cloned() {
            let action = AddTransaction {
                transaction: transaction.0.clone(),
                ledger_entries: transaction.1.clone(),
            };
            TestEntries::add_journal_entry(&mut journal_entries, &organization.id, action)
        }
        TestEntries {
            organization_contact,
            organization,
            contacts,
            currencies,
            accounts,
            transactions,
            journal_entries,
        }
    }

    fn add_journal_entry(
        journal_entries: &mut Vec<JournalEntry>,
        organization_id: &OrganizationId,
        action: Action,
    ) {
        let previous_id = journal_entries
            .last()
            .map(|je| je.id)
            .unwrap_or(JournalEntryId::generate());
        let je = JournalEntry::new_after_id(previous_id, organization_id.clone(), action);
        journal_entries.push(je);
    }
}

pub fn test_entries() -> TestEntries {
    // Contacts
    let organization_contact =
        Contact::new(ContactType::Organization, "Test Company".to_string(), None);
    let owner = Contact::new(ContactType::Individual, "Test Owner".to_string(), None);
    let bank1 = Contact::new(ContactType::Organization, "Test Bank".to_string(), None);

    let contacts = vec![owner.clone(), bank1.clone()];

    // Currencies
    let usd = Currency {
        id: USD as u32,
        code: "USD".to_string(),
        scale: 2,
        name: "US Dollars".to_string(),
    };

    let btc = Currency {
        id: BTC as u32,
        code: "BTC".to_string(),
        scale: 8,
        name: "Bitcoin".to_string(),
    };

    let currencies = vec![usd.clone(), btc.clone()];

    // Organization
    let organization = Organization::new(&organization_contact.id);

    // COA entries
    let assets_acct = Account::new(
        None,
        100,
        "Assets".to_string(),
        AccountType::LedgerAccount,
        AccountCategory::BalanceSheet(BalanceSheetCategory::Asset),
    );

    let liabilities_acct = Account::new(
        None,
        200,
        "Liabilities".to_string(),
        AccountType::LedgerAccount,
        AccountCategory::BalanceSheet(BalanceSheetCategory::Liability),
    );

    let equity_acct = Account::new(
        None,
        300,
        "Equity".to_string(),
        AccountType::LedgerAccount,
        AccountCategory::BalanceSheet(BalanceSheetCategory::Equity),
    );

    let revenue_acct = Account::new(
        None,
        400,
        "Revenue".to_string(),
        AccountType::LedgerAccount,
        AccountCategory::IncomeStatement(IncomeStatementCategory::OperatingRevenue),
    );

    let expenses_acct = Account::new(
        None,
        500,
        "Expenses".to_string(),
        AccountType::LedgerAccount,
        AccountCategory::IncomeStatement(IncomeStatementCategory::OperatingExpense),
    );

    let owner1_acct = Account::new(
        Some(&equity_acct.id),
        100,
        "Owner 1".to_string(),
        AccountType::ContactAccount {
            contact_id: owner.id,
        },
        AccountCategory::BalanceSheet(BalanceSheetCategory::Equity),
    );

    let bank_checking_acct = Account::new(
        Some(&assets_acct.id),
        100,
        "Bank Checking".to_string(),
        AccountType::BankAccount {
            currency_id: usd.id,
            routing: 11111,
            account: 123123123123,
        },
        AccountCategory::BalanceSheet(BalanceSheetCategory::Asset),
    );

    let office_supp_acct = Account::new(
        Some(&expenses_acct.id),
        100,
        "Office Supplies".to_string(),
        AccountType::LedgerAccount,
        AccountCategory::IncomeStatement(IncomeStatementCategory::OperatingExpense),
    );

    let consult_income_acct = Account::new(
        Some(&revenue_acct.id),
        100,
        "Consulting Income".to_string(),
        AccountType::LedgerAccount,
        AccountCategory::IncomeStatement(IncomeStatementCategory::OperatingRevenue),
    );

    // Test funding transaction
    let datetime = datetime!(2022-01-03 09:00 UTC);
    let funding_tx = Transaction::new(
        datetime,
        "Owner's initial funding".to_string(),
        TransactionType::Invoice {
            payment_method: PaymentMethod::Cash,
            payment_terms: PaymentTerms::ImmediatePayment,
            payments: vec![],
        },
    );

    let funding_debits = vec![LedgerEntry::new(
        &funding_tx.id,
        EntryType::Debit,
        &bank_checking_acct.id,
        CurrencyAmount::new(
            &usd.id,
            Decimal::new(10_000_00, usd.scale), // USD 10,000.00
        ),
        Some("Owner funds deposited to bank".to_string()),
    )];

    let funding_credits = vec![LedgerEntry::new(
        &funding_tx.id,
        EntryType::Credit,
        &owner1_acct.id,
        CurrencyAmount::new(
            &usd.id,
            Decimal::new(10_000_00, usd.scale), // USD 10,000.00
        ),
        Some("Equity credited to owner".to_string()),
    )];

    // Test income transaction
    let datetime = datetime!(2022-02-03 09:00 UTC);
    let income_tx = Transaction::new(
        datetime,
        "Consulting income".to_string(),
        TransactionType::Invoice {
            payment_method: PaymentMethod::Check {
                contact_id: organization_contact.id.clone(),
                currency_id: USD as u32,
            },
            payment_terms: PaymentTerms::ImmediatePayment,
            payments: vec![],
        },
    );

    let income_debits = vec![LedgerEntry::new(
        &income_tx.id,
        EntryType::Debit,
        &bank_checking_acct.id,
        CurrencyAmount::new(
            &usd.id,
            Decimal::new(800000, usd.scale), // USD 8,000.00
        ),
        Some("Consulting fee deposit".to_string()),
    )];

    let income_credits = vec![LedgerEntry::new(
        &income_tx.id,
        EntryType::Credit,
        &consult_income_acct.id,
        CurrencyAmount::new(
            &usd.id,
            Decimal::new(800000, usd.scale), // USD 8,000.00
        ),
        Some("Consulting services".to_string()),
    )];

    let accounts = vec![
        assets_acct,
        liabilities_acct,
        equity_acct,
        revenue_acct,
        expenses_acct,
        owner1_acct,
        bank_checking_acct,
        office_supp_acct,
        consult_income_acct,
    ];

    let transactions: Vec<(Transaction, Vec<LedgerEntry>)> = vec![
        (
            funding_tx.clone(),
            [funding_debits, funding_credits].concat(),
        ),
        (income_tx.clone(), [income_debits, income_credits].concat()),
    ];

    TestEntries::new(
        organization_contact,
        organization,
        contacts,
        currencies,
        accounts,
        transactions,
    )
}

#[cfg(test)]
pub(crate) mod test {
    use crate::journal::{test_entries, Journal, JournalEntry, VecDb};

    #[test]
    fn test_add_view() {
        let db = VecDb::new();
        let journal = Journal::new(db);
        let test_entries = test_entries();
        for entry in &test_entries.journal_entries {
            journal.add(entry.clone()).unwrap();
        }
        let journal_view = journal.view().unwrap();
        let test_journal: Vec<JournalEntry> = test_entries.journal_entries;
        assert_eq!(journal_view.len(), test_journal.len());
        for index in 0..test_journal.len() {
            assert_eq!(
                &journal_view.get(index).unwrap(),
                &test_journal.get(index).unwrap()
            );
        }
    }
}
