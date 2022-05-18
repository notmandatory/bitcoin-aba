use crate::journal::{Account, AccountId, CurrencyAmount, CurrencyId, EntryType, LedgerEntry};
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
    pub account_ids: Vec<AccountId>,
    pub account_totals: Vec<AccountTotals>,
}

impl Report {
    pub fn new(ledger: &Ledger, date_time: OffsetDateTime, account_ids: Vec<AccountId>) -> Self {
        let accounts = account_ids
            .iter()
            .map(|id| ledger.get_account(&id).expect("account"));
        let account_totals: Vec<AccountTotals> =
            accounts.map(|a| AccountTotals::new(ledger, a)).collect();

        Report {
            date_time,
            account_ids,
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
    pub fn new(ledger: &Ledger, account: Arc<Account>) -> Self {
        let child_ids = ledger.child_ids(&account);
        let child_account_totals: Vec<AccountTotals> = child_ids
            .iter()
            .map(|account_id| ledger.get_account(account_id))
            .flatten()
            .map(|account| AccountTotals::new(&ledger, account))
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
    use crate::journal::test_entries;
    use crate::journal::AccountCategory::{BalanceSheet, IncomeStatement};
    use crate::journal::BalanceSheetCategory::{Asset, Equity, Liability};
    use crate::journal::IncomeStatementCategory::{OperatingExpense, OperatingRevenue};
    use crate::ledger::report::Report;
    use crate::ledger::test::setup;
    use crate::ledger::OrganizationLedgers;
    use rust_decimal::Decimal;
    use time::OffsetDateTime;

    #[test]
    fn test_balance_sheet() {
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

        let asset_account_id = ledger
            .get_root_account(BalanceSheet(Asset))
            .expect("Asset account");
        let liability_account_id = ledger
            .get_root_account(BalanceSheet(Liability))
            .expect("Liability account");
        let equity_account_id = ledger
            .get_root_account(BalanceSheet(Equity))
            .expect("Equity account");
        let report = Report::new(
            &ledger,
            OffsetDateTime::now_utc(),
            vec![asset_account_id, liability_account_id, equity_account_id],
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
        let test_entries = test_entries();
        let organization_id = test_entries.organization.id;
        let organization_ledgers = &mut OrganizationLedgers::new();
        organization_ledgers
            .add_journal_entries(test_entries.journal_entries)
            .expect("load journal");
        let ledger = organization_ledgers
            .get_ledger(&organization_id)
            .expect("ledger");

        let revenue_account_id = ledger
            .get_root_account(IncomeStatement(OperatingRevenue))
            .expect("Revenue account");
        let expense_account_id = ledger
            .get_root_account(IncomeStatement(OperatingExpense))
            .expect("Expense account");

        let report = Report::new(
            &ledger,
            OffsetDateTime::now_utc(),
            vec![revenue_account_id, expense_account_id],
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
