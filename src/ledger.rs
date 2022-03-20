use crate::journal::Action::{AddAccount, AddCurrency, AddTransaction};
use crate::journal::{
    Account, AccountId, AccountNumber, AccountType, Currency, CurrencyNumber, Journal,
    JournalEntry, Transaction, TransactionId,
};
use crate::Error;
use log::debug;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Ledger {
    pub account_map: BTreeMap<AccountId, Account>,
    pub currency_map: BTreeMap<CurrencyNumber, Currency>,
    pub transaction_map: BTreeMap<TransactionId, Transaction>,
}

impl Ledger {
    pub fn new(journal: &Journal) -> Result<Ledger, Error> {
        let journal_entries = journal.view()?;

        let mut account_map: BTreeMap<AccountId, Account> = BTreeMap::new();
        let mut currency_map: BTreeMap<CurrencyNumber, Currency> = BTreeMap::new();
        let mut transaction_map: BTreeMap<TransactionId, Transaction> = BTreeMap::new();

        // sync journal entries to ledger collections
        for je in journal_entries {
            // TODO validate id and version

            match je {
                JournalEntry {
                    id: _,
                    version: _,
                    action: AddAccount { account },
                } => {
                    debug!(
                        "insert account: {}",
                        serde_json::to_string(&account).unwrap()
                    );
                    account_map.insert(account.id, account);
                }
                JournalEntry {
                    id: _,
                    version: _,
                    action: AddCurrency { currency },
                } => {
                    debug!(
                        "insert currency: {}",
                        serde_json::to_string(&currency).unwrap()
                    );
                    currency_map.insert(currency.number, currency);
                }
                JournalEntry {
                    id: _,
                    version: _,
                    action: AddTransaction { transaction },
                } => {
                    debug!(
                        "insert transaction: {}",
                        serde_json::to_string(&transaction).unwrap()
                    );
                    transaction_map.insert(transaction.id, transaction);
                }
            }
        }

        Ok(Ledger {
            account_map,
            currency_map,
            transaction_map,
        })
    }

    pub fn get_parent_id(&self, account: &Account) -> Result<Option<AccountId>, Error> {
        Ok(match account.account_type {
            AccountType::Organization {
                parent_id: Some(parent_id),
            } => Some(parent_id),
            AccountType::Organization { parent_id: None } => None,
            AccountType::OrganizationUnit { parent_id } => Some(parent_id),
            AccountType::Category { parent_id, .. } => Some(parent_id),
            AccountType::SubAccount { parent_id } => Some(parent_id),
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

    pub fn get_children<'a>(&'a self, account: &'a Account) -> Vec<&Account> {
        self.account_map.values().filter(|a| {
            if let Ok ( Some ( parent_id ) ) = self.get_parent_id(a) {
              parent_id == account.id
            } else {
                false
            }
        }).collect()
    }

    pub fn get_child_ids<'a>(&'a self, account: &'a Account) -> Vec<AccountId> {
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
}

#[cfg(test)]
mod test {
    use log::debug;
    use crate::journal::Action::AddAccount;
    use crate::journal::{
        Account, AccountType, AccountValue, Action, Currency, FinancialStatement, Journal,
        JournalEntry, Transaction,
    };
    use crate::ledger::Ledger;
    use rust_decimal::Decimal;
    use time::macros::datetime;

    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| env_logger::init_from_env(env_logger::Env::new().default_filter_or("info")));
    }

    #[test]
    fn test_new_get() {
        setup();

        let journal = Journal::new_mem().expect("journal");

        let test_data = test_data();
        for entry in &test_data.journal_entries {
            journal.add(entry.clone()).unwrap();
        }

        let ledger = Ledger::new(&journal).expect("ledger");

        assert_eq!(ledger.account_map.len(), 9);
        assert_eq!(ledger.currency_map.len(), 1);
        assert_eq!(ledger.transaction_map.len(), 1);

        let transaction1 = ledger.transaction_map.values().next().expect("transaction");
        assert_eq!(transaction1.credits.len(), 1);
        assert_eq!(transaction1.debits.len(), 1);

        let debit = transaction1.debits.get(0).unwrap();
        let debit_account = ledger.account_map.get(&debit.account_id).unwrap();
        assert_eq!(debit_account.number, 100);
        if let AccountType::SubAccount { parent_id } = debit_account.account_type {
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
        let ledger = Ledger::new(&journal).expect("ledger");

        for account in &test_data.accounts {
            debug!("account: {:?}, parent: {:?}", account, ledger.get_parent(account).unwrap());
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
        let ledger = Ledger::new(&journal).expect("ledger");

        for account in &test_data.accounts {
            debug!("account: {:?}, number: {:?}", account, ledger.get_full_number(account).unwrap());
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
        let ledger = Ledger::new(&journal).expect("ledger");

        for account in &test_data.accounts {
            debug!("account: {:?}, children: {:?}", account, ledger.get_children(account));
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
        let ledger = Ledger::new(&journal).expect("ledger");

        for account in &test_data.accounts {
            debug!("account: {:?}, children: {:?}", account, ledger.get_child_ids(account));
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
        // COA entries
        let org_acct = Account::new(
            10,
            "Test Organization".to_string(),
            AccountType::Organization { parent_id: None },
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
            AccountType::SubAccount {
                parent_id: equity_acct.id,
            },
        );
        let bank_checking_acct = Account::new(
            100,
            "Bank Checking".to_string(),
            AccountType::SubAccount {
                parent_id: assets_acct.id,
            },
        );
        let office_supp_acct = Account::new(
            100,
            "Office Supplies".to_string(),
            AccountType::SubAccount {
                parent_id: expenses_acct.id,
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

        let usd = Currency {
            number: 840,
            code: "USD".to_string(),
            scale: 2,
            name: Some("US Dollars".to_string()),
            description: Some("US Dollar Reserve Notes".to_string()),
        };

        let currencies = vec![usd.clone()];

        let debits = vec![AccountValue {
            account_id: bank_checking_acct.id.clone(),
            currency_number: usd.number,
            amount: Decimal::new(10_000_00, usd.scale), // USD 10,000.00
            description: Some("Owner funds deposited to bank".to_string()),
        }];

        let credits = vec![AccountValue {
            account_id: owner1_acct.id.clone(),
            currency_number: usd.number,
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
