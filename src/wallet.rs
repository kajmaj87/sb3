use crate::business::ItemType;
use crate::logs::LogEvent;
use crate::money::Money;
use bevy::prelude::*;

#[derive(Debug, Clone)]
pub enum TradeSide {
    Pay,
    Receive,
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
    },
    Salary {
        side: TradeSide,
        employer: Entity,
        worker: Entity,
        salary: Money,
    },
}
#[derive(Debug, Clone)]
pub enum TransactionError {
    InsufficientFunds(Money),
    WalletNotFound,
}

#[derive(Component, Default)]
pub struct Wallet {
    money: Money,
    pub(crate) transactions: Vec<Transaction>,
}

impl Wallet {
    pub fn new(money: Money) -> Self {
        Self {
            money,
            transactions: Vec::new(),
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
                };
                other_wallet.transactions.push(symmetric_transaction);
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
            } => {
                self.process_payout(other_wallet, side, salary)?;
                logs.send(LogEvent::Salary {
                    employer,
                    worker,
                    salary,
                });
            }
        }
        self.transactions.push(transaction.clone());
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
}
