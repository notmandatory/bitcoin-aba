use crate::journal::Action::{AddAccount, AddContact, AddCurrency, AddTransaction};
use crate::journal::{
    Account, AccountId, AccountNumber, AccountType, Contact, ContactId, Currency, CurrencyId,
    JournalEntry, LedgerEntry, Transaction, TransactionId,
};

use log::error;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub mod report;

#[derive(Debug, Clone)]
pub enum Error {
    MissingAccount(AccountId),
    MissingCurrency(CurrencyId),
    MissingContact(ContactId),
    MissingTransaction(TransactionId),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAccount(a) => write!(f, "missing account: {}", a),
            Self::MissingCurrency(c) => write!(f, "missing currency: {}", c),
            Self::MissingContact(e) => write!(f, "missing contact: {}", e),
            Self::MissingTransaction(t) => write!(f, "missing transaction: {}", t),
        }
    }
}

#[derive(Clone)]
pub struct Ledger {
    account_map: BTreeMap<AccountId, Arc<Account>>,
    currency_map: BTreeMap<CurrencyId, Arc<Currency>>,
    contact_map: BTreeMap<ContactId, Arc<Contact>>,
    transaction_map: BTreeMap<TransactionId, Arc<Transaction>>,
    transaction_entries_map: BTreeMap<TransactionId, Vec<Arc<LedgerEntry>>>,
    account_entries_map: BTreeMap<AccountId, Vec<Arc<LedgerEntry>>>,
}

impl Ledger {
    pub fn new() -> Ledger {
        let account_map: BTreeMap<AccountId, Arc<Account>> = BTreeMap::new();
        let currency_map: BTreeMap<CurrencyId, Arc<Currency>> = BTreeMap::new();
        let contact_map: BTreeMap<ContactId, Arc<Contact>> = BTreeMap::new();
        let transaction_map: BTreeMap<TransactionId, Arc<Transaction>> = BTreeMap::new();
        let transaction_entries_map: BTreeMap<TransactionId, Vec<Arc<LedgerEntry>>> =
            BTreeMap::new();
        let account_entries_map: BTreeMap<AccountId, Vec<Arc<LedgerEntry>>> = BTreeMap::new();
        Ledger {
            account_map,
            currency_map,
            contact_map,
            transaction_map,
            transaction_entries_map,
            account_entries_map,
        }
    }

    pub fn account_exists(&self, account_id: &AccountId) -> Result<(), Error> {
        if !self.account_map.contains_key(&account_id) {
            return Err(Error::MissingAccount(account_id.clone()));
        }
        Ok(())
    }

    pub fn currency_exists(&self, currency_id: &CurrencyId) -> Result<(), Error> {
        if !self.currency_map.contains_key(&currency_id) {
            return Err(Error::MissingCurrency(currency_id.clone()));
        }
        Ok(())
    }

    pub fn contact_exists(&self, contact_id: &ContactId) -> Result<(), Error> {
        if !self.contact_map.contains_key(&contact_id) {
            return Err(Error::MissingContact(contact_id.clone()));
        }
        Ok(())
    }

    pub fn transaction_exists(&self, transaction_id: &TransactionId) -> Result<(), Error> {
        if !self.transaction_map.contains_key(&transaction_id) {
            return Err(Error::MissingTransaction(transaction_id.clone()));
        }
        Ok(())
    }

    pub fn account_type_valid(&self, account_type: &AccountType) -> Result<(), Error> {
        match account_type {
            AccountType::ContactAccount { contact_id } => self.contact_exists(contact_id),
            AccountType::BankAccount { currency_id, .. } => self.currency_exists(currency_id),
            _ => Ok(()),
        }
    }

    // add journal entries to ledger collections
    pub fn add_journal_entries(&mut self, journal_entries: Vec<JournalEntry>) -> Result<(), Error> {
        for je in journal_entries {
            if let Err(error) = self.add_journal_entry(je) {
                error!("{}", &error);
                return Err(error);
            }
        }
        Ok(())
    }

    // add single journal entry to ledger collections
    pub fn add_journal_entry(&mut self, journal_entry: JournalEntry) -> Result<(), Error> {
        match journal_entry {
            JournalEntry {
                id: _,
                version: _,
                action: AddAccount { account },
            } => {
                //debug!("insert account: {}", serde_json::to_string(&account)?);
                self.account_type_valid(&account.account_type)?;
                self.account_map.insert(account.id, Arc::new(account));
            }
            JournalEntry {
                id: _,
                version: _,
                action: AddCurrency { currency },
            } => {
                //debug!("insert currency: {}", serde_json::to_string(&currency)?);
                self.currency_map.insert(currency.id, Arc::new(currency));
            }
            JournalEntry {
                id: _,
                version: _,
                action: AddContact { contact },
            } => match self
                .contact_map
                .insert(contact.id, Arc::new(contact.clone()))
            {
                None => {
                    //debug!("insert new contact: {}", serde_json::to_string(&contact)?);
                }
                Some(_old) => {
                    // debug!(
                    //     "replace contact old {} with new: {}",
                    //     serde_json::to_string(&old)?,
                    //     serde_json::to_string(&contact)?
                    // );
                }
            },
            JournalEntry {
                id: _,
                version: _,
                action:
                    AddTransaction {
                        transaction,
                        ledger_entries,
                    },
            } => {
                // debug!(
                //     "insert transaction: {} with entries {}",
                //     serde_json::to_string(&transaction)?,
                //     serde_json::to_string(&ledger_entries)?
                // );
                let transaction_id = transaction.id;
                self.transaction_map
                    .insert(transaction_id.clone(), Arc::new(transaction));
                let transaction_entries: Vec<Arc<LedgerEntry>> =
                    ledger_entries.iter().map(|e| Arc::new(e.clone())).collect();
                let account_entries: Vec<Arc<LedgerEntry>> =
                    transaction_entries.iter().cloned().collect();
                // TODO verify transaction doesn't already exist
                self.transaction_entries_map
                    .insert(transaction_id, transaction_entries);
                self.insert_account_entries(account_entries);
            }
        }
        Ok(())
    }

    pub fn parent<'a>(&'a self, account: &'a Account) -> Result<Option<&Account>, Error> {
        let parent_id: Option<AccountId> = account.parent_id;
        match parent_id {
            Some(id) => {
                if let Some(account) = self.account_map.get(&id) {
                    Ok(Some(account))
                } else {
                    Err(Error::MissingAccount(id))
                }
            }
            None => Ok(None),
        }
    }

    pub fn children<'a>(&'a self, account: &'a Arc<Account>) -> Vec<Arc<Account>> {
        self.account_map
            .values()
            .filter(|a| {
                if let Some(parent_id) = a.parent_id {
                    parent_id == account.id
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    pub fn child_ids<'a>(&'a self, account: &'a Arc<Account>) -> Vec<AccountId> {
        self.children(account).iter().map(|c| c.id).collect()
    }

    pub fn full_number(&self, account: &Account) -> Result<Vec<AccountNumber>, Error> {
        let parent_opt = self.parent(account)?;
        match parent_opt {
            Some(parent) => {
                let mut full_number = self.full_number(parent)?;
                full_number.push(account.number);
                Ok(full_number)
            }
            None => Ok(vec![account.number]),
        }
    }

    pub fn get_account(&self, id: &AccountId) -> Option<Arc<Account>> {
        self.account_map.get(id).cloned()
    }

    pub fn accounts(&self) -> Vec<Arc<Account>> {
        self.account_map.values().cloned().collect()
    }

    pub fn get_currency(&self, id: &CurrencyId) -> Option<Arc<Currency>> {
        self.currency_map.get(id).cloned()
    }

    pub fn currencies(&self) -> Vec<Arc<Currency>> {
        self.currency_map.values().cloned().collect()
    }

    pub fn get_contact(&self, id: &ContactId) -> Option<Arc<Contact>> {
        self.contact_map.get(id).cloned()
    }

    pub fn contacts(&self) -> Vec<Arc<Contact>> {
        self.contact_map.values().cloned().collect()
    }

    pub fn get_transaction(&self, id: &TransactionId) -> Option<Arc<Transaction>> {
        self.transaction_map.get(id).cloned()
    }

    pub fn transactions(&self) -> Vec<Arc<Transaction>> {
        self.transaction_map.values().cloned().collect()
    }

    pub fn get_account_entries(&self, account_id: &AccountId) -> Option<Vec<Arc<LedgerEntry>>> {
        self.account_entries_map.get(account_id).cloned()
    }

    pub fn insert_account_entries(&mut self, entries: Vec<Arc<LedgerEntry>>) {
        for entry in entries.iter().cloned() {
            let account_id = &entry.account_id;
            match self.account_entries_map.get_mut(account_id) {
                None => {
                    self.account_entries_map
                        .insert(entry.account_id, vec![entry]);
                }
                Some(account_entries) => {
                    account_entries.push(entry);
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::journal::test_entries;
    use crate::journal::Action::{AddAccount, AddContact};
    use crate::journal::{
        Account, AccountType, Contact, ContactType, EntryType, JournalEntry, LedgerEntry,
    };
    use crate::ledger::Ledger;
    use log::debug;
    use std::sync::Arc;

    use crate::journal::FinancialStatement::{BalanceSheet, CashFlow, IncomeStatement};
    use rusty_ulid::Ulid;
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn setup() {
        INIT.call_once(|| {
            env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"))
        });
    }

    #[test]
    fn test_new_get() {
        setup();
        let test_entries = test_entries();
        let mut ledger = Ledger::new();
        ledger
            .add_journal_entries(test_entries.journal_entries)
            .expect("loaded journal");

        assert_eq!(ledger.account_map.len(), 10);
        assert_eq!(ledger.currency_map.len(), 2);
        assert_eq!(ledger.transaction_map.len(), 2);

        let transaction1 = ledger.transaction_map.values().next().expect("transaction");

        let transaction_entries_len = ledger.transaction_entries_map.len();
        assert_eq!(transaction_entries_len, test_entries.transactions.len());

        let transactions_len = ledger.transaction_map.len();
        assert_eq!(transactions_len, test_entries.transactions.len());

        let account_entries_keys_len = ledger.account_entries_map.keys().len();
        assert_eq!(account_entries_keys_len, 3);
        let transaction_entries_keys_len = ledger.transaction_entries_map.keys().len();
        assert_eq!(transaction_entries_keys_len, 2);

        let account_entries_values_len = ledger
            .account_entries_map
            .values()
            .flatten()
            .collect::<Vec<&Arc<LedgerEntry>>>()
            .len();
        assert_eq!(account_entries_values_len, 4);

        let transaction_entries_values_len = ledger
            .transaction_entries_map
            .values()
            .flatten()
            .collect::<Vec<&Arc<LedgerEntry>>>()
            .len();
        assert_eq!(transaction_entries_values_len, 4);

        let transaction1_entries = ledger
            .transaction_entries_map
            .get(&transaction1.id)
            .expect("transaction1 entries");
        let transaction1_credits: Vec<&Arc<LedgerEntry>> = transaction1_entries
            .iter()
            .filter(|&e| e.entry_type == EntryType::Credit)
            .collect();
        assert_eq!(transaction1_credits.len(), 1);
        let transaction1_debits: Vec<&Arc<LedgerEntry>> = transaction1_entries
            .iter()
            .filter(|&e| e.entry_type == EntryType::Debit)
            .collect();
        assert_eq!(transaction1_debits.len(), 1);

        // let debit = *transaction1_debits.get(0).unwrap();
        // let debit_account = ledger.account_map.get(&debit.account_id).unwrap();
        // assert_eq!(debit_account.number, 100);
        // if let Some(parent_id) = debit_account.parent_id {
        //     let parent_account = ledger.account_map.get(&parent_id).unwrap();
        //     assert_eq!(parent_account.number, 100);
        // } else {
        //     panic!()
        // }
        //
        // let credit = *transaction1_credits.get(0).unwrap();
        // let credit_account = ledger.account_map.get(&credit.account_id).unwrap();
        // assert_eq!(credit_account.number, 100);
        // if let Some(parent_id) = credit_account.parent_id {
        //     let parent_account = ledger.account_map.get(&parent_id).unwrap();
        //     assert_eq!(parent_account.number, 300);
        // } else {
        //     panic!()
        // }
    }

    #[test]
    fn test_get_parent() {
        setup();
        let test_entries = test_entries();
        let mut ledger = Ledger::new();
        ledger
            .add_journal_entries(test_entries.journal_entries)
            .expect("loaded journal");

        for account in &test_entries.accounts {
            debug!(
                "account: {:?}, parent: {:?}",
                account,
                ledger.parent(account).unwrap()
            );
        }
    }

    #[test]
    fn test_get_full_number() {
        setup();
        let test_entries = test_entries();
        let mut ledger = Ledger::new();
        ledger
            .add_journal_entries(test_entries.journal_entries)
            .expect("loaded journal");

        for account in &test_entries.accounts {
            debug!(
                "account: {:?}, number: {:?}",
                account,
                ledger.full_number(account).unwrap()
            );
        }
    }

    #[test]
    fn test_get_children() {
        setup();
        let test_entries = test_entries();
        let mut ledger = Ledger::new();
        ledger
            .add_journal_entries(test_entries.journal_entries)
            .expect("loaded journal");

        for account in &test_entries.accounts {
            debug!(
                "account: {:?}, children: {:?}",
                account,
                ledger.children(&Arc::new(account.clone()))
            );
        }
    }

    #[test]
    fn test_get_child_ids() {
        setup();
        let test_entries = test_entries();
        let mut ledger = Ledger::new();
        ledger
            .add_journal_entries(test_entries.journal_entries)
            .expect("loaded journal");

        for account in &test_entries.accounts {
            debug!(
                "account: {:?}, children: {:?}",
                account,
                ledger.child_ids(&Arc::new(account.clone()))
            );
        }
    }

    #[test]
    fn test_add_contact_account() {
        setup();

        let test_contact = Contact {
            id: Ulid::generate(),
            contact_type: ContactType::Individual,
            name: "Tester".to_string(),
            address: None,
        };

        let test_account = Account {
            id: Ulid::generate(),
            parent_id: Some(test_contact.id.clone()),
            number: 10,
            description: "Valid".to_string(),
            account_type: AccountType::LedgerAccount,
            statements: vec![BalanceSheet, IncomeStatement, CashFlow],
        };

        let mut ledger = Ledger::new();

        let result = ledger.add_journal_entry(JournalEntry {
            id: Ulid::generate(),
            version: 0,
            action: AddContact {
                contact: test_contact.clone(),
            },
        });
        assert!(result.is_ok());

        let result = ledger.add_journal_entry(JournalEntry {
            id: Ulid::generate(),
            version: 0,
            action: AddAccount {
                account: test_account.clone(),
            },
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_account() {
        setup();

        let test_account = Account {
            id: Ulid::generate(),
            parent_id: Some(Ulid::generate()),
            number: 10,
            description: "Invalid".to_string(),
            account_type: AccountType::ContactAccount {
                contact_id: Ulid::generate(),
            },
            statements: vec![],
        };

        let mut ledger = Ledger::new();
        let result = ledger.add_journal_entry(JournalEntry {
            id: Ulid::generate(),
            version: 0,
            action: AddAccount {
                account: test_account,
            },
        });

        if let Err(e) = result {
            debug!("Expected validation error: {:?}", e);
        } else {
            debug!("Expected ok result");
        }
    }
}
