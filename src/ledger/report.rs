use crate::journal::{
    Account, AccountId, CurrencyAmount, CurrencyId, EntryType, FinancialStatement,
    LedgerEntry,
};
use crate::ledger::Ledger;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::Add;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Report {
    pub date_time: OffsetDateTime,
    pub financial_statement: FinancialStatement,
    pub account: Arc<Account>,
    pub account_totals: Vec<AccountTotals>,
}

impl Report {
    pub fn new(
        ledger: &Ledger,
        date_time: OffsetDateTime,
        statement: FinancialStatement,
        account_id: AccountId,
    ) -> Self {
        // TODO verify account
        let account = ledger.get_account(&account_id).unwrap();
        let account_totals = ledger
            .child_ids(&account)
            .iter()
            .flat_map(|child_id| ledger.get_account(child_id))
            .filter(|account| account.statements.contains(&statement))
            .map(|child_account| AccountTotals::new(ledger, child_account, &statement))
            .collect();

        Report {
            date_time,
            financial_statement: statement,
            account,
            account_totals,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct AccountTotals {
    pub account: Arc<Account>,
    pub debit_totals: Vec<CurrencyAmount>,
    pub credit_totals: Vec<CurrencyAmount>,
    pub child_account_totals: Vec<AccountTotals>,
}

impl AccountTotals {
    pub fn new(ledger: &Ledger, account: Arc<Account>, statement: &FinancialStatement) -> Self {
        let child_ids = ledger.child_ids(&account);
        let child_account_totals: Vec<AccountTotals> = child_ids
            .iter()
            .map(|account_id| ledger.get_account(account_id))
            .flatten()
            .filter(|account| account.statements.contains(statement))
            .map(|account| AccountTotals::new(&ledger, account, statement))
            .collect();
        let child_totals: [BTreeMap<CurrencyId, Decimal>; 2] = child_account_totals.iter().fold(
            [BTreeMap::new(), BTreeMap::new()],
            |mut acc, totals| {
                for debit_total in totals.debit_totals.clone() {
                    let new_debit_total = acc[0]
                        .entry(debit_total.currency_id)
                        .or_default()
                        .add(debit_total.amount);
                    acc[0].insert(debit_total.currency_id, new_debit_total);
                }
                for credit_total in totals.credit_totals.clone() {
                    let new_credit_total = acc[1]
                        .entry(credit_total.currency_id)
                        .or_default()
                        .add(credit_total.amount);
                    acc[1].insert(credit_total.currency_id, new_credit_total);
                }
                acc
            },
        );
        let [child_debit_totals, child_credit_totals]: [Vec<CurrencyAmount>; 2] =
            child_totals.map(|total_map| {
                total_map
                    .iter()
                    .map(|(currency_id, amount)| {
                        let currency_id = currency_id.clone();
                        let amount = amount.clone();
                        CurrencyAmount {
                            currency_id,
                            amount,
                        }
                    })
                    .collect()
            });

        let account_entries: Vec<Arc<LedgerEntry>> = ledger
            .get_account_entries(&account.id)
            .iter()
            .flatten()
            .cloned()
            .collect();

        let account_totals: [BTreeMap<CurrencyId, Decimal>; 2] = account_entries
            .iter()
            .cloned()
            .fold([BTreeMap::new(), BTreeMap::new()], |mut acc, entry| {
                let currency_amount = entry.currency_amount.clone();
                match entry.entry_type {
                    EntryType::Debit => {
                        let new_total = acc[0]
                            .entry(currency_amount.currency_id)
                            .or_default()
                            .add(currency_amount.amount);
                        acc[0].insert(currency_amount.currency_id, new_total);
                    }
                    EntryType::Credit => {
                        let new_total = acc[1]
                            .entry(currency_amount.currency_id)
                            .or_default()
                            .add(currency_amount.amount);
                        acc[1].insert(currency_amount.currency_id, new_total);
                    }
                }
                acc
            });
        let [debit_totals, credit_totals]: [Vec<CurrencyAmount>; 2] =
            account_totals.map(|total_map| {
                total_map
                    .iter()
                    .map(|(currency_id, amount)| {
                        let currency_id = currency_id.clone();
                        let amount = amount.clone();
                        CurrencyAmount {
                            currency_id,
                            amount,
                        }
                    })
                    .collect()
            });

        let mut debit_totals_appended: Vec<CurrencyAmount> = debit_totals.clone();
        debit_totals_appended.append(child_debit_totals.clone().as_mut());

        let mut credit_totals_appended: Vec<CurrencyAmount> = credit_totals.clone();
        credit_totals_appended.append(child_credit_totals.clone().as_mut());

        // fold together child account totals to account totals
        let [debit_totals, credit_totals]: [Vec<CurrencyAmount>; 2] =
            [debit_totals_appended, credit_totals_appended].map(|totals: Vec<CurrencyAmount>| {
                totals
                    .iter()
                    .fold(
                        BTreeMap::new(),
                        |mut acc: BTreeMap<CurrencyId, Decimal>, total| {
                            let new_total =
                                acc.entry(total.currency_id).or_default().add(total.amount);
                            acc.insert(total.currency_id, new_total);
                            acc
                        },
                    )
                    .iter()
                    .map(|(currency_id, amount)| {
                        let currency_id = currency_id.clone();
                        let amount = amount.clone();
                        CurrencyAmount {
                            currency_id,
                            amount,
                        }
                    })
                    .collect()
            });

        AccountTotals {
            account,
            debit_totals,
            credit_totals,
            child_account_totals,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::journal::test::test_entries;
    use crate::journal::{FinancialStatement, Journal};
    use crate::ledger::report::Report;
    use crate::ledger::test::setup;
    use crate::ledger::Ledger;
    use rust_decimal::Decimal;
    use time::OffsetDateTime;

    #[test]
    fn test_balance_sheet() {
        setup();
        let journal = Journal::new_mem().expect("journal");
        let test_entries = test_entries();
        for entry in &test_entries.journal_entries {
            journal.add(entry.clone()).unwrap();
        }

        let mut ledger = Ledger::new();
        ledger.load_journal(&journal).expect("loaded journal");

        let account_id = test_entries.accounts.get(0).expect("first account").id;
        let report = Report::new(
            &ledger,
            OffsetDateTime::now_utc(),
            FinancialStatement::BalanceSheet,
            account_id,
        );

        let account0_debits0_amount = report
            .account_totals
            .iter()
            .filter(|totals| totals.account.description.eq(&"Assets".to_string()))
            .next()
            .expect("assets account")
            .debit_totals
            .iter()
            .filter(|currency_amount| currency_amount.currency_id.eq(&840))
            .next()
            .expect("debits 0")
            .amount;
        assert_eq!(Decimal::new(18_000_00, 2), account0_debits0_amount);

        let account2_credits0_amount = report
            .account_totals
            .iter()
            .filter(|totals| totals.account.description.eq(&"Equity".to_string()))
            .next()
            .expect("equity account")
            .credit_totals
            .iter()
            .filter(|currency_amount| currency_amount.currency_id.eq(&840))
            .next()
            .expect("credits 0")
            .amount;
        assert_eq!(Decimal::new(10_000_00, 2), account2_credits0_amount);
    }

    #[test]
    fn test_income_statement() {
        setup();
        let journal = Journal::new_mem().expect("journal");
        let test_entries = test_entries();
        for entry in &test_entries.journal_entries {
            journal.add(entry.clone()).unwrap();
        }

        let mut ledger = Ledger::new();
        ledger.load_journal(&journal).expect("loaded journal");

        let account_id = test_entries.accounts.get(0).expect("first account").id;
        let report = Report::new(
            &ledger,
            OffsetDateTime::now_utc(),
            FinancialStatement::IncomeStatement,
            account_id,
        );

        let account0_credits0_amount = report
            .account_totals
            .iter()
            .filter(|totals| totals.account.description.eq(&"Revenue".to_string()))
            .next()
            .expect("revenue account")
            .credit_totals
            .iter()
            .filter(|currency_amount| currency_amount.currency_id.eq(&840))
            .next()
            .expect("credits 0")
            .amount;
        assert_eq!(Decimal::new(8_000_00, 2), account0_credits0_amount);
    }
}
