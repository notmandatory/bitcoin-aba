use crate::journal::Action::{
    AddAccount, AddContact, AddCurrency, AddOrganization, AddTransaction,
};
use crate::journal::{
    Account, AccountCategory, AccountId, AccountNumber, AccountType, Contact, ContactId, Currency,
    CurrencyId, JournalEntry, LedgerEntry, Organization, OrganizationId, Transaction,
    TransactionId,
};

use log::error;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub mod report;

#[derive(Debug, Clone)]
pub enum Error {
    MissingAccount(AccountId),
    AccountExists(AccountId),
    MissingCurrency(CurrencyId),
    CurrencyExists(CurrencyId),
    MissingContact(ContactId),
    ContactExists(ContactId),
    MissingTransaction(TransactionId),
    TransactionExists(TransactionId),
    LedgerEntriesExists(TransactionId),
    MissingOrganization(OrganizationId),
    OrganizationExists(OrganizationId),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAccount(a) => write!(f, "missing account: {}", a),
            Self::AccountExists(a) => write!(f, "account exists: {}", a),
            Self::MissingCurrency(c) => write!(f, "missing currency: {}", c),
            Self::CurrencyExists(c) => write!(f, "currency exists: {}", c),
            Self::MissingContact(c) => write!(f, "missing contact: {}", c),
            Self::ContactExists(c) => write!(f, "contact exists: {}", c),
            Self::MissingTransaction(t) => write!(f, "missing transaction: {}", t),
            Self::TransactionExists(t) => write!(f, "transaction exists: {}", t),
            Self::LedgerEntriesExists(t) => write!(f, "transaction entries exists: {}", t),
            Self::MissingOrganization(o) => write!(f, "missing organization: {}", o),
            Self::OrganizationExists(o) => write!(f, "organization exists: {}", o),
        }
    }
}

pub struct OrganizationLedgers {
    organization_map: BTreeMap<OrganizationId, Organization>,
    ledger_map: BTreeMap<OrganizationId, Ledger>,
}

impl OrganizationLedgers {
    pub fn new() -> Self {
        let organization_map = BTreeMap::new();
        let ledger_map = BTreeMap::new();
        OrganizationLedgers {
            organization_map,
            ledger_map,
        }
    }

    pub fn organization_exists(&self, organization_id: &OrganizationId) -> bool {
        self.organization_map.contains_key(organization_id)
    }

    pub fn get_ledger(&self, organization_id: &OrganizationId) -> Result<&Ledger, Error> {
        match self.ledger_map.get(organization_id) {
            Some(ledger) => Ok(ledger),
            None => Err(Error::MissingOrganization(organization_id.clone())),
        }
    }

    pub fn get_mut_ledger(
        &mut self,
        organization_id: &OrganizationId,
    ) -> Result<&mut Ledger, Error> {
        match self.ledger_map.get_mut(organization_id) {
            Some(ledger) => Ok(ledger),
            None => Err(Error::MissingOrganization(organization_id.clone())),
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
                organization_id,
                action:
                    AddOrganization {
                        contact,
                        organization,
                    },
            } => {
                // debug!(
                //     "add contact: {}, organization: {}",
                //     serde_json::to_string(&contact).expect("contact"),
                //     serde_json::to_string(&organization).expect("organization"),
                // );
                if self.organization_exists(&organization.id) {
                    return Err(Error::OrganizationExists(organization.id));
                } else {
                    self.organization_map
                        .insert(organization_id.clone(), organization);
                    let ledger = Ledger::new();
                    self.ledger_map.insert(organization_id.clone(), ledger);
                    self.get_mut_ledger(&organization_id)?
                        .add_contact(contact)?;
                }
            }
            JournalEntry {
                id: _,
                version: _,
                organization_id,
                action: AddAccount { account },
            } => {
                //debug!("add account: {}", serde_json::to_string(&account)?);
                let ledger = self.get_mut_ledger(&organization_id)?;
                ledger.add_account(account)?;
            }
            JournalEntry {
                id: _,
                version: _,
                organization_id,
                action: AddCurrency { currency },
            } => {
                //debug!("insert currency: {}", serde_json::to_string(&currency)?);
                let ledger = self.get_mut_ledger(&organization_id)?;
                ledger.add_currency(currency)?;
            }
            JournalEntry {
                id: _,
                version: _,
                organization_id,
                action: AddContact { contact },
            } => {
                let ledger = self.get_mut_ledger(&organization_id)?;
                ledger.add_contact(contact)?;
            }
            JournalEntry {
                id: _,
                version: _,
                organization_id,
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
                let ledger = self.get_mut_ledger(&organization_id)?;
                let transaction_id = transaction.id.clone();
                ledger.add_transaction(transaction)?;
                let ledger_entries = ledger_entries.iter().map(|e| Arc::new(e.clone())).collect();
                ledger.add_ledger_entries(transaction_id, &ledger_entries)?;
                ledger.add_account_entries(&ledger_entries)
            }
        }
        Ok(())
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
    pub fn new() -> Self {
        let account_map = BTreeMap::new();
        let currency_map = BTreeMap::new();
        let contact_map = BTreeMap::new();
        let transaction_map = BTreeMap::new();
        let transaction_entries_map = BTreeMap::new();
        let account_entries_map = BTreeMap::new();
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

    pub fn add_account(&mut self, account: Account) -> Result<(), Error> {
        if !self.account_map.contains_key(&account.id) {
            self.account_map.insert(account.id, Arc::new(account));
            Ok(())
        } else {
            Err(Error::AccountExists(account.id))
        }
    }

    pub fn add_currency(&mut self, currency: Currency) -> Result<(), Error> {
        if !self.currency_map.contains_key(&currency.id) {
            self.currency_map.insert(currency.id, Arc::new(currency));
            Ok(())
        } else {
            Err(Error::CurrencyExists(currency.id))
        }
    }

    pub fn currency_exists(&self, currency_id: &CurrencyId) -> Result<(), Error> {
        if !self.currency_map.contains_key(&currency_id) {
            return Err(Error::MissingCurrency(currency_id.clone()));
        }
        Ok(())
    }

    pub fn add_contact(&mut self, contact: Contact) -> Result<(), Error> {
        if !self.contact_map.contains_key(&contact.id) {
            self.contact_map.insert(contact.id, Arc::new(contact));
            Ok(())
        } else {
            Err(Error::ContactExists(contact.id))
        }
    }

    pub fn contact_exists(&self, contact_id: &ContactId) -> Result<(), Error> {
        if !self.contact_map.contains_key(&contact_id) {
            return Err(Error::MissingContact(contact_id.clone()));
        }
        Ok(())
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<(), Error> {
        if !self.transaction_map.contains_key(&transaction.id) {
            self.transaction_map
                .insert(transaction.id.clone(), Arc::new(transaction));
            Ok(())
        } else {
            Err(Error::TransactionExists(transaction.id))
        }
    }

    pub fn add_ledger_entries(
        &mut self,
        transaction_id: TransactionId,
        ledger_entries: &Vec<Arc<LedgerEntry>>,
    ) -> Result<(), Error> {
        if !self.transaction_entries_map.contains_key(&transaction_id) {
            let entries: Vec<Arc<LedgerEntry>> = ledger_entries
                .iter()
                .cloned()
                .map(|entry| entry.clone())
                .collect();
            self.transaction_entries_map.insert(transaction_id, entries);
            Ok(())
        } else {
            Err(Error::LedgerEntriesExists(transaction_id))
        }
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

    pub fn children<'a>(&'a self, account_id: &'a AccountId) -> Vec<Arc<Account>> {
        self.account_map
            .values()
            .filter(|a| {
                if let Some(parent_id) = a.parent_id {
                    &parent_id == account_id
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    pub fn child_ids<'a>(&'a self, account: &'a Arc<Account>) -> Vec<AccountId> {
        self.children(&account.id).iter().map(|c| c.id).collect()
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

    pub fn get_root_account(&self, category: AccountCategory) -> Option<AccountId> {
        self.account_map
            .values()
            .filter(|account| account.parent_id.is_none())
            .find_map(|account| {
                if account.account_category.eq(&category) {
                    Some(account.id.clone())
                } else {
                    None
                }
            })
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

    pub fn add_account_entries(&mut self, entries: &Vec<Arc<LedgerEntry>>) {
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
    use crate::journal::{Account, AccountCategory, AccountType, BalanceSheetCategory, Contact, ContactType, JournalEntry, Organization, test_entries};
    use crate::journal::{EntryType, LedgerEntry};
    use crate::ledger::OrganizationLedgers;
    use log::debug;
    use std::sync::Arc;
    use std::sync::Once;
    use rusty_ulid::Ulid;
    use crate::journal::Action::{AddAccount, AddOrganization};

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
        let organization_id = test_entries.organization.id;
        let organization_ledgers = &mut OrganizationLedgers::new();
        organization_ledgers
            .add_journal_entries(test_entries.journal_entries)
            .expect("load journal");
        let ledger = organization_ledgers
            .get_ledger(&organization_id)
            .expect("ledger");
        assert_eq!(ledger.account_map.len(), test_entries.accounts.len());
        assert_eq!(ledger.currency_map.len(), test_entries.currencies.len());
        assert_eq!(
            ledger.transaction_map.len(),
            test_entries.transactions.len()
        );

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

        let organization_id = test_entries.organization.id;
        let organization_ledgers = &mut OrganizationLedgers::new();
        organization_ledgers
            .add_journal_entries(test_entries.journal_entries)
            .expect("load journal");
        let ledger = organization_ledgers
            .get_ledger(&organization_id)
            .expect("ledger");

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

        let organization_id = test_entries.organization.id;
        let organization_ledgers = &mut OrganizationLedgers::new();
        organization_ledgers
            .add_journal_entries(test_entries.journal_entries)
            .expect("load journal");
        let ledger = organization_ledgers
            .get_ledger(&organization_id)
            .expect("ledger");

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

        let organization_id = test_entries.organization.id;
        let organization_ledgers = &mut OrganizationLedgers::new();
        organization_ledgers
            .add_journal_entries(test_entries.journal_entries)
            .expect("load journal");
        let ledger = organization_ledgers
            .get_ledger(&organization_id)
            .expect("ledger");

        for account in &test_entries.accounts {
            dbg!(
                "account: {:?}, children: {:?}",
                account,
                ledger.children(&account.id)
            );
        }
    }

    #[test]
    fn test_get_child_ids() {
        setup();
        let test_entries = test_entries();

        let organization_id = test_entries.organization.id;
        let organization_ledgers = &mut OrganizationLedgers::new();
        organization_ledgers
            .add_journal_entries(test_entries.journal_entries)
            .expect("load journal");
        let ledger = organization_ledgers
            .get_ledger(&organization_id)
            .expect("ledger");

        for account in &test_entries.accounts {
            debug!(
                "account: {:?}, children: {:?}",
                account,
                ledger.child_ids(&Arc::new(account.clone()))
            );
        }
    }

    #[test]
    fn test_invalid_account() {
        setup();

        let organization_contact = Contact::new(ContactType::Individual, "test".to_string(), None);
        let organization = Organization::new(&organization_contact.id);
        let organization_id = organization.id.clone();
        let organization_ledgers = &mut OrganizationLedgers::new();
        organization_ledgers.add_journal_entry(JournalEntry::new_gen_id(organization.id, AddOrganization {
            contact: organization_contact,
            organization: organization
        })).expect("Add organization");
        let test_account = Account {
            id: Ulid::generate(),
            parent_id: Some(Ulid::generate()),
            number: 10,
            description: "Invalid".to_string(),
            account_type: AccountType::ContactAccount {
                contact_id: Ulid::generate(),
            },
            account_category: AccountCategory::BalanceSheet(BalanceSheetCategory::Equity),
        };

        let result = organization_ledgers.add_journal_entry(JournalEntry {
            id: Ulid::generate(),
            version: 0,
            organization_id,
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
