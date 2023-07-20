use crate::business::ItemType;
use crate::logs::LogEvent;
use crate::money::{Money, MoneyChange};
use bevy::prelude::*;
use either::Either;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;

#[derive(Debug, Clone)]
pub enum TradeSide {
    Pay,
    Receive,
}

impl fmt::Display for TradeSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TradeSide::Pay => write!(f, "Payed for"),
            TradeSide::Receive => write!(f, "Received"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Transaction {
    Trade {
        side: TradeSide,
        buyer: Entity,
        seller: Entity,
        item: Entity,
        item_type: ItemType,
        price: Money,
        date: usize,
    },
    Salary {
        side: TradeSide,
        employer: Entity,
        worker: Entity,
        salary: Money,
        date: usize,
    },
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Transaction::Trade {
                side,
                item_type,
                price,
                ..
            } => write!(f, "{} {} for {}", side, price, item_type.name),
            Transaction::Salary { side, salary, .. } => write!(f, "{} salary: {}", side, salary),
        }
    }
}

impl Transaction {
    /// This method returns the financial change brought about by the transaction.
    ///
    /// For a `Trade`, the change is represented by the `price` of the traded item.
    /// If the `side` of the trade is `Pay`, the change is a cost (outgoing money),
    /// thus it's returned as `Either::Left(price)`.
    ///
    /// If the `side` of the trade is `Receive`, the change is a gain (incoming money),
    /// thus it's returned as `Either::Right(price)`.
    ///
    /// For a `Salary`, the change is represented by the `salary`.
    /// If the `side` of the salary transaction is `Pay`, the change is a cost (outgoing money),
    /// thus it's returned as `Either::Left(salary)`.
    ///
    /// If the `side` of the salary transaction is `Receive`, the change is a gain (incoming money),
    /// thus it's returned as `Either::Right(salary)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::Transaction;
    /// # use your_crate::TradeSide;
    /// # let price = 100;
    /// let transaction = Transaction::Trade {
    ///     side: TradeSide::Pay,
    ///     // ... rest of the fields
    /// };
    /// assert_eq!(transaction.get_change(), Either::Left(price));
    ///
    /// let transaction = Transaction::Trade {
    ///     side: TradeSide::Receive,
    ///     // ... rest of the fields
    /// };
    /// assert_eq!(transaction.get_change(), Either::Right(price));
    /// ```
    ///
    /// This method returns the financial change brought about by the transaction
    /// as a `MoneyChange`.
    ///
    /// For a `Trade`, the change is represented by the `price` of the traded item.
    /// If the `side` of the trade is `Pay`, the change is a cost (outgoing money),
    /// thus it's returned as `MoneyChange::Left(price)`.
    ///
    /// If the `side` of the trade is `Receive`, the change is a gain (incoming money),
    /// thus it's returned as `MoneyChange::Right(price)`.
    ///
    /// For a `Salary`, the change is represented by the `salary`.
    /// If the `side` of the salary transaction is `Pay`, the change is a cost (outgoing money),
    /// thus it's returned as `MoneyChange::Left(salary)`.
    ///
    /// If the `side` of the salary transaction is `Receive`, the change is a gain (incoming money),
    /// thus it's returned as `MoneyChange::Right(salary)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::Transaction;
    /// # use your_crate::TradeSide;
    /// # let price = 100;
    /// let transaction = Transaction::Trade {
    ///     side: TradeSide::Pay,
    ///     // ... rest of the fields
    /// };
    /// assert_eq!(transaction.get_change(), MoneyChange::Left(price));
    ///
    /// let transaction = Transaction::Trade {
    ///     side: TradeSide::Receive,
    ///     // ... rest of the fields
    /// };
    /// assert_eq!(transaction.get_change(), MoneyChange::Right(price));
    /// ```
    pub fn get_change(&self) -> Either<Money, Money> {
        match self {
            Transaction::Trade { side, price, .. } => match side {
                TradeSide::Pay => Either::Left(*price),
                TradeSide::Receive => Either::Right(*price),
            },
            Transaction::Salary { side, salary, .. } => match side {
                TradeSide::Pay => Either::Left(*salary),
                TradeSide::Receive => Either::Right(*salary),
            },
        }
    }

    pub fn get_date(&self) -> usize {
        match self {
            Transaction::Trade { date, .. } => *date,
            Transaction::Salary { date, .. } => *date,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TransactionError {
    InsufficientFunds(Money),
    WalletNotFound,
}

#[derive(Component, Default)]
pub struct Wallet {
    money: Money,
    pub(crate) transactions: VecDeque<Transaction>,
}

impl Wallet {
    pub fn new(money: Money) -> Self {
        Self {
            money,
            transactions: VecDeque::new(),
        }
    }

    pub fn money(&self) -> Money {
        self.money
    }

    fn add_money(&mut self, money: Money) {
        self.money += money;
    }

    fn subtract_money(&mut self, money: Money) -> Result<(), TransactionError> {
        if self.money >= money {
            self.money -= money;
            Ok(())
        } else {
            Err(TransactionError::InsufficientFunds(money - self.money))
        }
    }

    pub fn transaction(
        &mut self,
        other_wallet: &mut Wallet,
        transaction: &Transaction,
        logs: &mut EventWriter<LogEvent>,
    ) -> Result<(), TransactionError> {
        match transaction.clone() {
            Transaction::Trade {
                side,
                buyer,
                seller,
                item,
                item_type,
                price,
                date,
            } => {
                self.process_payout(other_wallet, side.clone(), price)?;
                let symmetric_transaction = Transaction::Trade {
                    side: match side {
                        TradeSide::Pay => TradeSide::Receive,
                        TradeSide::Receive => TradeSide::Pay,
                    },
                    buyer: seller,
                    seller: buyer,
                    item,
                    item_type: item_type.clone(),
                    price,
                    date,
                };
                other_wallet.transactions.push_front(symmetric_transaction);
                // TODO refactor so we can easily create log events from transactions
                logs.send(LogEvent::Trade {
                    buyer,
                    seller,
                    item_type,
                    price,
                });
            }
            Transaction::Salary {
                side,
                employer,
                worker,
                salary,
                date,
            } => {
                self.process_payout(other_wallet, side.clone(), salary)?;
                let symmetric_transaction = Transaction::Salary {
                    side: match side {
                        TradeSide::Pay => TradeSide::Receive,
                        TradeSide::Receive => TradeSide::Pay,
                    },
                    employer: worker,
                    worker: employer,
                    salary,
                    date,
                };
                other_wallet.transactions.push_front(symmetric_transaction);
                logs.send(LogEvent::Salary {
                    employer,
                    worker,
                    salary,
                });
            }
        }
        self.transactions.push_front(transaction.clone());
        Ok(())
    }

    fn process_payout(
        &mut self,
        other_wallet: &mut Wallet,
        side: TradeSide,
        price: Money,
    ) -> Result<(), TransactionError> {
        match side {
            TradeSide::Pay => {
                self.subtract_money(price)?;
                other_wallet.add_money(price);
            }
            TradeSide::Receive => {
                self.add_money(price);
                other_wallet.subtract_money(price)?;
            }
        }
        Ok(())
    }

    /// Calculates the total financial change in the last `n` days.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::Wallet;
    /// # let wallet: Wallet = get_wallet(); // assume `get_wallet` is a function which returns a wallet
    /// # let n = 30;
    /// let total_change = wallet.calculate_total_change(n);
    /// println!("The total financial change in the last {} days is: {}", n, total_change);
    /// ```
    pub fn calculate_total_change(&self, current_date: usize, n: usize) -> MoneyChange {
        // The tuples represent (cost, gain)
        let (total_cost, total_gain): (Money, Money) = self
            .transactions
            .iter()
            .take_while(|transaction| current_date - transaction.get_date() <= n)
            .map(|transaction| match transaction.get_change() {
                MoneyChange::Left(cost) => (cost, Money(0)), // increase total cost
                MoneyChange::Right(gain) => (Money(0), gain), // increase total gain
            })
            .fold(
                (Money(0), Money(0)),
                |(acc_cost, acc_gain), (cost, gain)| {
                    (acc_cost + cost, acc_gain + gain) // increment total cost and gain
                },
            );

        // Determine the net change
        if total_gain > total_cost {
            MoneyChange::Right(total_gain - total_cost) // net gain
        } else {
            MoneyChange::Left(total_cost - total_gain) // net cost
        }
    }

    /// Generate a summary of transactions for the last n days and last m transactions.
    ///
    /// This summary includes total costs and profits by item type, as well as a list of the last m transactions.
    ///
    /// # Arguments
    ///
    /// * `n` - A number of days to consider for the summary.
    /// * `m` - A number of transactions to include in the list of last transactions.
    ///
    /// # Returns
    ///
    /// * A `String` containing the summary.

    pub fn get_summary(&self, current_date: usize, n: usize, m: usize) -> String {
        let mut costs = BTreeMap::new();
        let mut profits = BTreeMap::new();
        let mut cost_items_amount = HashMap::new();
        let mut profit_items_amount = HashMap::new();
        let mut salary_costs = Money(0);
        let mut salary_profits = Money(0);
        let transactions = self
            .transactions
            .iter()
            .take_while(|t| current_date - t.get_date() <= n)
            .collect::<Vec<_>>();

        // transactions.reverse();

        for transaction in &transactions {
            match transaction {
                Transaction::Trade {
                    side,
                    item_type,
                    price,
                    ..
                } => match side {
                    TradeSide::Pay => {
                        *costs.entry(item_type).or_insert(Money(0)) += *price;
                        *cost_items_amount.entry(item_type).or_insert(0) += 1;
                    }
                    TradeSide::Receive => {
                        *profits.entry(item_type).or_insert(Money(0)) += *price;
                        *profit_items_amount.entry(item_type).or_insert(0) += 1;
                    }
                },
                Transaction::Salary { side, salary, .. } => match side {
                    TradeSide::Pay => salary_costs += *salary,
                    TradeSide::Receive => salary_profits += *salary,
                },
            }
        }

        let total_costs: Money = costs.values().sum::<Money>() + salary_costs;
        let total_profits: Money = profits.values().sum::<Money>() + salary_profits;

        let mut summary = String::new();

        summary.push_str(&format!("Summary for the last {} days:\n\n", n));

        if !costs.is_empty() || salary_costs.0 > 0 {
            summary.push_str("Costs:\n");
        }

        if !costs.is_empty() {
            let mut cost_items: Vec<_> = costs.iter().collect();
            cost_items.sort_by_key(|&(_, cost)| Reverse(*cost));
            for (item_type, cost) in cost_items {
                summary.push_str(&format!(
                    "  {}: {} ({})\n",
                    item_type,
                    cost,
                    cost_items_amount.get(item_type).unwrap_or(&0)
                ));
            }
            summary.push_str(&format!(
                "  Total Purchases: {}\n\n",
                total_costs - salary_costs
            ));
        }

        if salary_costs.0 > 0 {
            summary.push_str(&format!("  Salaries: {}\n\n", salary_costs));
        }

        if !profits.is_empty() || salary_profits.0 > 0 {
            summary.push_str("Profits:\n");
        }

        if !profits.is_empty() {
            let mut profit_items: Vec<_> = profits.iter().collect();
            profit_items.sort_by_key(|&(_, profit)| Reverse(*profit));
            for (item_type, profit) in profit_items {
                summary.push_str(&format!(
                    "  {}: {} ({})\n",
                    item_type,
                    profit,
                    profit_items_amount.get(item_type).unwrap_or(&0)
                ));
            }
            summary.push_str(&format!(
                "  Total Sales: {}\n\n",
                total_profits - salary_profits
            ));
        }

        if salary_profits.0 > 0 {
            summary.push_str(&format!("  Salaries: {}\n\n", salary_profits));
        }

        if total_costs > total_profits {
            summary.push_str(&format!("Total Net: -{}\n\n", total_costs - total_profits));
        } else {
            summary.push_str(&format!("Total Net: {}\n\n", total_profits - total_costs));
        };

        summary.push_str(&format!("Last {} transactions:\n", m));
        for transaction in transactions.iter().take(m) {
            summary.push_str(&format!("  {}\n", transaction));
        }

        summary
    }
}
