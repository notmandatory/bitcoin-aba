use crate::journal::Action::AddAccount;
use crate::journal::{Account, AccountId, Journal, JournalEntry};
use crate::Error;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Ledger {
    pub account_map: BTreeMap<AccountId, Account>,
}

impl Ledger {
    pub fn new(journal: &Journal) -> Result<Ledger, Error> {
        let journal_entries = journal.view()?;

        let mut account_map: BTreeMap<AccountId, Account> = BTreeMap::new();

        // sync journal entries to ledger collections
        for je in journal_entries {
            match je {
                JournalEntry {
                    id: _,
                    version: _,
                    action: AddAccount { account },
                } => {
                    account_map.insert(account.id, account);
                }
                _ => (),
            }
        }

        Ok(Ledger { account_map })
    }
}

#[cfg(test)]
mod test {
    use crate::journal::{Account, AccountType, Action, Journal, JournalEntry};
    use crate::ledger::Ledger;

    #[test]
    fn test_new_get() {
        let journal = Journal::new_mem().expect("journal");

        let account = Account::new(
            100,
            "Test account".to_string(),
            AccountType::Organization { parent_id: None },
        );
        let account_id = account.id.clone();
        let entry = JournalEntry::new(Action::AddAccount {
            account: account.clone(),
        });
        journal.add(entry.clone()).unwrap();

        let ledger = Ledger::new(&journal).expect("ledger");

        assert_eq!(ledger.account_map.len(), 1);
        assert_eq!(ledger.account_map.get(&account_id), Some(&account));
    }
}
