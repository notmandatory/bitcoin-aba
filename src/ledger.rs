use crate::journal::Action::{AddAccount, AddCurrency, AddEntity, AddTransaction};
use crate::journal::{
    Account, AccountId, AccountNumber, AccountType, Currency, CurrencyId, Entity, EntityId,
    Journal, JournalEntry, Transaction, TransactionId,
};
use crate::Error;
use log::{debug, error};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Ledger {
    account_map: BTreeMap<AccountId, Arc<Account>>,
    currency_map: BTreeMap<CurrencyId, Arc<Currency>>,
    entity_map: BTreeMap<EntityId, Arc<Entity>>,
    transaction_map: BTreeMap<TransactionId, Arc<Transaction>>,
}

impl Ledger {
    pub fn new() -> Ledger {
        let account_map: BTreeMap<AccountId, Arc<Account>> = BTreeMap::new();
        let currency_map: BTreeMap<CurrencyId, Arc<Currency>> = BTreeMap::new();
        let entity_map: BTreeMap<EntityId, Arc<Entity>> = BTreeMap::new();
        let transaction_map: BTreeMap<TransactionId, Arc<Transaction>> = BTreeMap::new();

        Ledger {
            account_map,
            currency_map,
            entity_map,
            transaction_map,
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

    pub fn entity_exists(&self, entity_id: &EntityId) -> Result<(), Error> {
        if !self.entity_map.contains_key(&entity_id) {
            return Err(Error::MissingEntity(entity_id.clone()));
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
            AccountType::Organization {
                parent_id,
                entity_id,
            } => {
                if let Some(parent_id) = parent_id {
                    self.account_exists(parent_id)?;
                }
                self.entity_exists(entity_id)?;
            }
            AccountType::OrganizationUnit {
                parent_id,
                entity_id,
            } => {
                self.account_exists(parent_id)?;
                self.entity_exists(entity_id)?;
            }
            AccountType::Category { parent_id, .. } => {
                self.account_exists(parent_id)?;
            }
            AccountType::LedgerAccount {
                parent_id,
            } => {
                debug!("adding ledger account with parent id {}", &parent_id);
                self.account_exists(parent_id)?;
            }
            AccountType::EquityAccount {
                parent_id,
                entity_id,
            } => {
                self.account_exists(parent_id)?;
                self.entity_exists(entity_id)?;
            }
            AccountType::BankAccount {
                parent_id,
                currency_id,
                entity_id,
                ..
            } => {
                self.account_exists(parent_id)?;
                self.currency_exists(currency_id)?;
                self.entity_exists(entity_id)?;
            }
            AccountType::BitcoinAccount { parent_id, .. } => {
                self.account_exists(parent_id)?;
            }
        }
        Ok(())
    }

    pub fn load_journal(&mut self, journal: &Journal) -> Result<(), Error> {
        let journal_entries = journal.view()?;
        self.add_journal_entries(journal_entries)?;
        Ok(())
    }

    // add journal entries to ledger collections
    pub fn add_journal_entries(&mut self, journal_entries: Vec<JournalEntry>) -> Result<(), Error> {
        for je in journal_entries {
            if let Err(error) = self.add_journal_entry(je) {
                error!("Error: {}", error);
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
                debug!("insert account: {}", serde_json::to_string(&account)?);
                self.account_type_valid(&account.account_type)?;
                self.account_map.insert(account.id, Arc::new(account));
            }
            JournalEntry {
                id: _,
                version: _,
                action: AddCurrency { currency },
            } => {
                debug!("insert currency: {}", serde_json::to_string(&currency)?);
                self.currency_map.insert(currency.id, Arc::new(currency));
            }
            JournalEntry {
                id: _,
                version: _,
                action: AddEntity { entity },
            } => match self.entity_map.insert(entity.id, Arc::new(entity.clone())) {
                None => {
                    debug!("insert new entity: {}", serde_json::to_string(&entity)?);
                }
                Some(old) => {
                    debug!(
                        "replace entity old {} with new: {}",
                        serde_json::to_string(&old)?,
                        serde_json::to_string(&entity)?
                    );
                }
            },
            JournalEntry {
                id: _,
                version: _,
                action: AddTransaction { transaction },
            } => {
                debug!(
                    "insert transaction: {}",
                    serde_json::to_string(&transaction)?
                );
                self.transaction_map.insert(transaction.id, Arc::new(transaction));
            }
        }
        Ok(())
    }

    pub fn get_parent_id(&self, account: &Account) -> Result<Option<AccountId>, Error> {
        Ok(match account.account_type {
            AccountType::Organization {
                parent_id: Some(parent_id),
                ..
            } => Some(parent_id),
            AccountType::Organization {
                parent_id: None, ..
            } => None,
            AccountType::OrganizationUnit { parent_id, .. } => Some(parent_id),
            AccountType::Category { parent_id, .. } => Some(parent_id),
            AccountType::LedgerAccount { parent_id, .. } => Some(parent_id),
            AccountType::EquityAccount { parent_id, .. } => Some(parent_id),
            AccountType::BankAccount { parent_id, .. } => Some(parent_id),
            AccountType::BitcoinAccount { parent_id, .. } => Some(parent_id),
        })
    }

    pub fn get_parent<'a>(&'a self, account: &'a Account) -> Result<Option<&Account>, Error> {
        let parent_id: Option<AccountId> = self.get_parent_id(account)?;
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

    pub fn get_children<'a>(&'a self, account: &'a Arc<Account>) -> Vec<Arc<Account>> {
        self.account_map
            .values()
            .filter(|a| {
                if let Ok(Some(parent_id)) = self.get_parent_id(a) {
                    parent_id == account.id
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    pub fn get_child_ids<'a>(&'a self, account: &'a Arc<Account>) -> Vec<AccountId> {
        self.get_children(account).iter().map(|c| c.id).collect()
    }

    pub fn get_full_number(&self, account: &Account) -> Result<Vec<AccountNumber>, Error> {
        let parent_opt = self.get_parent(account)?;
        match parent_opt {
            Some(parent) => {
                let mut full_number = self.get_full_number(parent)?;
                full_number.push(account.number);
                Ok(full_number)
            }
            None => Ok(vec![account.number]),
        }
    }

    pub fn get_accounts(&self) -> Vec<Arc<Account>> {
        self.account_map.values().cloned().collect()
    }

    pub fn get_currencies(&self) -> Vec<Arc<Currency>> {
        self.currency_map.values().cloned().collect()
    }

    pub fn get_entities(&self) -> Vec<Arc<Entity>> {
        self.entity_map.values().cloned().collect()
    }

    pub fn get_transactions(&self) -> Vec<Arc<Transaction>> {
        self.transaction_map.values().cloned().collect()
    }
}

#[cfg(test)]
mod test {
    use crate::journal::Action::{AddAccount, AddEntity};
    use crate::journal::{
        Account, AccountType, AccountValue, Action, Currency, Entity, EntityType,
        FinancialStatement, Journal, JournalEntry, Transaction,
    };
    use crate::ledger::Ledger;
    use log::debug;
    use rust_decimal::Decimal;
    use time::macros::datetime;

    use crate::Error;
    use rusty_ulid::Ulid;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"))
        });
    }

    #[test]
    fn test_new_get() {
        setup();

        let journal = Journal::new_mem().expect("journal");

        let test_data = test_data();
        for entry in &test_data.journal_entries {
            journal.add(entry.clone()).unwrap();
        }

        let mut ledger = Ledger::new();
        ledger.load_journal(&journal).expect("loaded journal");

        assert_eq!(ledger.account_map.len(), 9);
        assert_eq!(ledger.currency_map.len(), 1);
        assert_eq!(ledger.transaction_map.len(), 1);

        let transaction1 = ledger.transaction_map.values().next().expect("transaction");
        assert_eq!(transaction1.credits.len(), 1);
        assert_eq!(transaction1.debits.len(), 1);

        let debit = transaction1.debits.get(0).unwrap();
        let debit_account = ledger.account_map.get(&debit.account_id).unwrap();
        assert_eq!(debit_account.number, 100);
        if let AccountType::BankAccount { parent_id, .. } = debit_account.account_type {
            let parent_account = ledger.account_map.get(&parent_id).unwrap();
            assert_eq!(parent_account.number, 100);
        } else {
            panic!()
        }
    }

    #[test]
    fn test_get_parent() {
        setup();
        let journal = Journal::new_mem().expect("journal");

        let test_data = test_data();
        for entry in &test_data.journal_entries {
            journal.add(entry.clone()).unwrap();
        }
        let mut ledger = Ledger::new();
        ledger.load_journal(&journal).expect("loaded journal");

        for account in &test_data.accounts {
            debug!(
                "account: {:?}, parent: {:?}",
                account,
                ledger.get_parent(account).unwrap()
            );
        }
    }

    #[test]
    fn test_get_full_number() {
        setup();
        let journal = Journal::new_mem().expect("journal");

        let test_data = test_data();
        for entry in &test_data.journal_entries {
            journal.add(entry.clone()).unwrap();
        }
        let mut ledger = Ledger::new();
        ledger.load_journal(&journal).expect("loaded journal");

        for account in &test_data.accounts {
            debug!(
                "account: {:?}, number: {:?}",
                account,
                ledger.get_full_number(account).unwrap()
            );
        }
    }

    #[test]
    fn test_get_children() {
        setup();

        let journal = Journal::new_mem().expect("journal");

        let test_data = test_data();
        for entry in &test_data.journal_entries {
            journal.add(entry.clone()).unwrap();
        }
        let mut ledger = Ledger::new();
        ledger.load_journal(&journal).expect("loaded journal");

        for account in &test_data.accounts {
            debug!(
                "account: {:?}, children: {:?}",
                account,
                ledger.get_children(account)
            );
        }
    }

    #[test]
    fn test_get_child_ids() {
        setup();

        let journal = Journal::new_mem().expect("journal");

        let test_data = test_data();
        for entry in &test_data.journal_entries {
            journal.add(entry.clone()).unwrap();
        }
        let mut ledger = Ledger::new();
        ledger.load_journal(&journal).expect("loaded journal");

        for account in &test_data.accounts {
            debug!(
                "account: {:?}, children: {:?}",
                account,
                ledger.get_child_ids(account)
            );
        }
    }

    #[test]
    fn test_add_entity() {
        setup();

        let test_entity = Entity {
            id: Ulid::generate(),
            entity_type: EntityType::Individual,
            name: "Tester".to_string(),
            address: None,
        };

        let test_account = Account {
            id: Ulid::generate(),
            number: 10,
            description: "Valid".to_string(),
            account_type: AccountType::Organization {
                parent_id: None,
                entity_id: test_entity.id.clone(),
            },
        };

        let mut ledger = Ledger::new();

        let result = ledger.add_journal_entry(JournalEntry {
            id: Ulid::generate(),
            version: 0,
            action: AddEntity {
                entity: test_entity.clone(),
            },
        });

        debug!("Result: {:?}", result);
        assert!(result.is_ok());

        let result = ledger.entity_exists(&test_entity.id);
        debug!("Result: {:?}", result);
        assert!(result.is_ok());

        let entities = ledger.get_entities();
        debug!("Entities: {:?}", entities);
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn test_invalid_account() {
        setup();

        let test_account = Account {
            id: Ulid::generate(),
            number: 10,
            description: "Invalid".to_string(),
            account_type: AccountType::Organization {
                parent_id: Some(Ulid::generate()),
                entity_id: Ulid::generate(),
            },
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

    // utility functions

    pub struct TestEntries {
        pub accounts: Vec<Account>,
        pub currencies: Vec<Currency>,
        pub transactions: Vec<Transaction>,
        pub journal_entries: Vec<JournalEntry>,
    }

    pub fn test_data() -> TestEntries {
        // Entity
        let company = Entity::new(EntityType::Organization, "Test Company".to_string(), None);
        let owner = Entity::new(EntityType::Individual, "Test Owner".to_string(), None);
        let bank1 = Entity::new(EntityType::Organization, "Test Bank".to_string(), None);

        // Currencies
        let usd = Currency {
            id: 840,
            code: "USD".to_string(),
            scale: 2,
            name: Some("US Dollars".to_string()),
            description: Some("US Dollar Reserve Notes".to_string()),
        };

        // COA entries
        let org_acct = Account::new(
            10,
            "Test Organization".to_string(),
            AccountType::Organization {
                parent_id: None,
                entity_id: company.id,
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
                entity_id: owner.id,
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

        let mut journal_entries: Vec<JournalEntry> = accounts
            .iter()
            .map(|a| JournalEntry::new(AddAccount { account: a.clone() }))
            .collect();

        // Test transaction entry

        let currencies = vec![usd.clone()];

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

        let transactions = vec![funding_tx.clone()];

        let usd_entry = JournalEntry::new(Action::AddCurrency { currency: usd });
        journal_entries.push(usd_entry);

        let transaction_entry = JournalEntry::new(Action::AddTransaction {
            transaction: funding_tx,
        });
        journal_entries.push(transaction_entry);

        TestEntries {
            accounts,
            currencies,
            transactions,
            journal_entries,
        }
    }
}
