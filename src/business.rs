use crate::debug_ui::Performance;
use crate::money::Money;
use crate::people::Person;
use crate::Days;
use bevy::prelude::*;
use macros::measured;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Ord, PartialOrd, Deserialize)]
pub struct ItemType {
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub struct ProductionCycle {
    pub input: HashMap<ItemType, u32>,
    pub output: (ItemType, u32),
    // tools: HashMap<ItemType, u32>,
    pub workdays_needed: u32,
}

impl fmt::Display for ProductionCycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Production Cycle:\n")?;
        writeln!(f, "Input:")?;
        for (item_type, count) in &self.input {
            writeln!(f, "  - {}: {}", item_type.name, count)?;
        }
        writeln!(f, "Output:")?;
        writeln!(f, "  - {}: {}\n", self.output.0.name, self.output.1)?;
        write!(f, "Workdays: {}", self.workdays_needed)
    }
}

#[derive(Debug, Default)]
pub struct Inventory {
    pub(crate) items: HashMap<ItemType, Vec<Entity>>,
    pub(crate) items_to_sell: HashSet<Entity>,
}

#[derive(Component)]
pub struct Wallet {
    pub(crate) money: Money,
}

#[derive(Bundle)]
pub struct ManufacturerBundle {
    pub name: Name,
    pub manufacturer: Manufacturer,
    pub sell_strategy: SellStrategy,
    pub wallet: Wallet,
    pub transaction_log: TransactionLog,
}

#[derive(Component, Debug)]
pub struct Manufacturer {
    pub(crate) production_cycle: ProductionCycle,
    pub(crate) assets: Inventory,
    pub(crate) hired_workers: Vec<Entity>,
}

#[derive(Component, Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Worker {
    pub(crate) salary: Money,
    // employer: Entity,
}

#[derive(Component, Debug)]
pub struct Item {
    item_type: ItemType,
    production_cost: Money,
    buy_cost: Money,
    // owner: Entity,
}

#[derive(Debug, Clone)]
pub enum TransactionType {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    transaction_type: TransactionType,
    buyer: Entity,
    seller: Entity,
    item: Entity,
    item_type: ItemType,
    // price: Money,
    // date: usize,
}

#[derive(Component, Debug, Default)]
pub struct TransactionLog {
    unprocessed_transactions: Vec<Transaction>,
    history: Vec<Transaction>,
}

#[derive(Component, Debug)]
pub struct SellOrder {
    item: Entity,
    pub(crate) item_type: ItemType,
    pub(crate) seller: Entity,
    pub(crate) price: Money,
    pub(crate) base_price: Money,
}

impl PartialEq for SellOrder {
    fn eq(&self, other: &Self) -> bool {
        self.item_type == other.item_type && self.price == other.price
    }
}

impl PartialOrd for SellOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(if self.price != other.price {
            self.price.cmp(&other.price)
        } else {
            self.item_type.name.cmp(&other.item_type.name)
        })
    }
}

impl Eq for SellOrder {} // This indicates to the compiler that your PartialEq implementation fulfills the stricter requirements of Eq

impl Ord for SellOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.price != other.price {
            self.price.cmp(&other.price)
        } else {
            self.item_type.name.cmp(&other.item_type.name)
        }
    }
}

#[derive(Component, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct SellStrategy {
    // max_margin: f32,
    pub(crate) margin_drop_per_day: f32,
    pub(crate) current_margin: f32,
    pub(crate) min_margin: f32,
}

#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
    // Limit(u64),
}

#[derive(Component, Debug, Clone)]
pub struct BuyOrder {
    pub(crate) item_type: ItemType,
    pub(crate) buyer: Entity,
    pub(crate) order: OrderType,
    pub(crate) expiration: Option<u64>,
}

#[derive(Component, Clone)]
pub struct BuyStrategy {
    pub(crate) target_production_cycles: u32,
    pub(crate) outstanding_orders: HashMap<ItemType, u32>,
}

#[derive(Debug)]
pub enum MaxCycleError {
    NoMaterialInInventory(String),
    // the String will contain the material name
    NotEnoughMaterialsOrWorkers,
    CantPayWorkers,
}

#[measured]
pub fn produce(
    mut manufacturers: Query<(&mut Wallet, &mut Manufacturer)>,
    mut commands: Commands,
    workers_query: Query<&Worker>,
    items_query: Query<&Item>,
) {
    for (mut wallet, mut manufacturer) in manufacturers.iter_mut() {
        // fill production cycle
        // produce_for_manufacturer(&mut b, commands, &production_cost);
        execute_production_cycle(
            &mut commands,
            &mut manufacturer,
            &mut wallet,
            &workers_query,
            &items_query,
        )
    }
}

fn execute_production_cycle(
    commands: &mut Commands,
    manufacturer: &mut Mut<Manufacturer>,
    wallet: &mut Mut<Wallet>,
    workers_query: &Query<&Worker>,
    items_query: &Query<&Item>,
) {
    match calculate_max_cycles(wallet, manufacturer, workers_query) {
        Ok((max_cycles, mut cost_per_cycle)) => {
            let input = manufacturer.production_cycle.input.clone();
            for (input_material, quantity_needed) in input.iter() {
                for _ in 0..quantity_needed * max_cycles {
                    let item = manufacturer
                        .assets
                        .items
                        .get_mut(input_material)
                        .unwrap()
                        .pop()
                        .unwrap();
                    if let Ok(item_cost) = items_query.get(item) {
                        commands.entity(item).despawn_recursive();
                        cost_per_cycle += item_cost.buy_cost;
                    } else {
                        error!("Item not found");
                    }
                }
            }
            debug!("Total cost per cycle: {}", cost_per_cycle);

            // if wallet.money < cost_per_cycle {
            //     debug!("Not enough money to run a cycle, nothing will be produced");
            //     return;
            // }
            // wallet.money -= cost_per_cycle;

            // Calculate output materials and their unit costs
            let (output_material, quantity_produced) =
                &manufacturer.production_cycle.output.clone();
            let unit_cost = cost_per_cycle / (*quantity_produced * max_cycles) as u64;
            for _ in 0..*quantity_produced * max_cycles {
                let item = Item {
                    item_type: output_material.clone(),
                    production_cost: unit_cost,
                    buy_cost: Money(0),
                    // owner: manufacturer_id,
                };
                debug!("Produced {:?}", item);
                let output_item = commands
                    .spawn((item, Name::new(output_material.name.to_string())))
                    .id();
                // manufacturer.assets.items.entry(output_material.clone()).or_insert(Vec::new()).push(output_item);
                manufacturer.assets.items_to_sell.insert(output_item);
            }
        }
        Err(e) => match e {
            MaxCycleError::NoMaterialInInventory(material) => {
                debug!(
                    "No material {} in inventory, max cycles will be 0",
                    material
                );
            }
            MaxCycleError::NotEnoughMaterialsOrWorkers => {
                debug!("Not enough materials or workers to run a cycle, nothing will be produced");
            }
            MaxCycleError::CantPayWorkers => {
                debug!("Dear Lord, we can't even pay our workers, we're doomed!");
            }
        },
    }
}

fn calculate_max_cycles(
    wallet: &Mut<Wallet>,
    manufacturer: &Mut<Manufacturer>,
    workers_query: &Query<&Worker>,
) -> Result<(u32, Money), MaxCycleError> {
    let mut max_cycles = u32::MAX;

    for (input_material, &quantity_needed) in manufacturer.production_cycle.input.iter() {
        if let Some(items_in_inventory) = manufacturer.assets.items.get(input_material) {
            let possible_cycles = items_in_inventory.len() as u32 / quantity_needed;
            max_cycles = max_cycles.min(possible_cycles);
        } else {
            debug!(
                "No material {:?} in inventory, max cycles will be 0",
                input_material
            );
            return Err(MaxCycleError::NoMaterialInInventory(
                input_material.name.to_string(),
            ));
        }
    }

    let max_cycles_by_workers =
        (manufacturer.hired_workers.len() as u32) / manufacturer.production_cycle.workdays_needed;
    debug!("Max cycles by workers: {}", max_cycles_by_workers);
    max_cycles = max_cycles.min(max_cycles_by_workers);
    debug!("Max cycles: {}", max_cycles);

    if max_cycles == 0 {
        debug!("Not enough materials or workers to run a cycle, nothing will be produced");
        return Err(MaxCycleError::NotEnoughMaterialsOrWorkers);
    }

    // Calculate the cost per cycle
    let mut cost_per_cycle = Money(0);
    for worker in manufacturer.hired_workers.iter() {
        cost_per_cycle += workers_query.get(*worker).unwrap().salary;
    }
    debug!("Salaries cost per cycle: {}", cost_per_cycle);

    if wallet.money < cost_per_cycle {
        debug!("Dear Lord, we can't even pay our workers, we're doomed!");
        return Err(MaxCycleError::CantPayWorkers);
    }

    Ok((max_cycles, cost_per_cycle))
}

#[measured]
pub fn create_sell_orders(
    mut commands: Commands,
    mut manufacturers: Query<(Entity, &mut Manufacturer, &SellStrategy)>,
    items_query: Query<(&Name, &Item)>,
) {
    for (seller, mut manufacturer, strategy) in manufacturers.iter_mut() {
        let mut items_to_sell = vec![];
        for item in manufacturer.assets.items_to_sell.iter() {
            items_to_sell.push(*item);
        }
        for item in items_to_sell {
            if let Ok((name, item_cost)) = items_query.get(item) {
                let sell_order = SellOrder {
                    item,
                    item_type: item_cost.item_type.clone(),
                    seller,
                    price: item_cost.production_cost
                        * strategy.current_margin.max(strategy.min_margin),
                    base_price: item_cost.production_cost,
                };
                debug!("Created sell order {:?} for {}", sell_order, name.as_str());
                let strategy_copy = *strategy;
                commands.spawn((
                    sell_order,
                    Name::new(format!("{} sell order", name.as_str())),
                    strategy_copy,
                ));
                manufacturer.assets.items_to_sell.remove(&item);
            }
        }
    }
}

#[measured]
pub fn update_sell_order_prices(
    mut sell_orders: Query<(Entity, &Name, &mut SellOrder, &mut SellStrategy)>,
) {
    for (_, name, mut sell_order, mut sell_strategy) in sell_orders.iter_mut() {
        sell_strategy.current_margin -= sell_strategy.margin_drop_per_day;
        sell_order.price =
            sell_order.base_price * sell_strategy.current_margin.max(sell_strategy.min_margin);
        debug!(
            "Updated {} sell order price to {}",
            name.as_str(),
            sell_order.price
        );
        if sell_strategy.current_margin < 1.0 {
            debug!("Oh my god, we're selling {} at a loss!", name.as_str());
        }
    }
}

#[measured]
pub fn create_buy_orders(
    mut commands: Commands,
    mut manufacturers: Query<(Entity, &Name, &Manufacturer, &mut BuyStrategy)>,
) {
    debug!(
        "Creating buy orders for {} buyers",
        manufacturers.iter_mut().count()
    );
    for (buyer, name, manufacturer, mut strategy) in manufacturers.iter_mut() {
        let needed_materials = &manufacturer.production_cycle.input;
        let inventory = &manufacturer.assets.items;
        debug!(
            "{}: Needed materials: {:?}",
            name.as_str(),
            needed_materials
        );

        for (material, &quantity_needed) in needed_materials.iter() {
            let inventory_quantity = inventory
                .get(material)
                .map_or(0, |items| items.len() as u32);

            let cycles_possible_with_current_inventory = inventory_quantity / quantity_needed;
            debug!(
                "{}: Cycles possible with current inventory: {}",
                name, cycles_possible_with_current_inventory
            );
            if cycles_possible_with_current_inventory < strategy.target_production_cycles {
                let current_orders = *strategy.outstanding_orders.get(material).unwrap_or(&0);
                debug!(
                    "{}: I need to buy {} for {} more production cycles ({} in total). I already have {} and {:?} in orders",
                    name,
                    material.name,
                    strategy.target_production_cycles - cycles_possible_with_current_inventory,
                    (strategy.target_production_cycles - cycles_possible_with_current_inventory) * quantity_needed,
                    inventory_quantity,
                    current_orders
                );
                let quantity_to_buy = ((strategy.target_production_cycles
                    - cycles_possible_with_current_inventory)
                    * quantity_needed) as i32
                    - current_orders as i32;
                if quantity_to_buy <= 0 {
                    debug!(
                        "{}: No need to buy any more {}, I already have {} and {} in orders",
                        name,
                        material.name,
                        inventory_quantity,
                        strategy.outstanding_orders.get(material).unwrap_or(&0)
                    );
                    continue;
                } else {
                    strategy
                        .outstanding_orders
                        .insert(material.clone(), current_orders + quantity_to_buy as u32);
                }

                let buy_order = BuyOrder {
                    item_type: material.clone(), // assuming ItemType implements Copy
                    buyer,
                    expiration: None,
                    order: OrderType::Market, // Always buying at market price
                };

                debug!(
                    "{}: Created buy order {:?} for {}",
                    name, buy_order, quantity_to_buy
                );

                // Assuming we have a way to track the quantity in BuyOrder
                for _ in 0..quantity_to_buy {
                    commands.spawn((
                        buy_order.clone(),
                        Name::new(format!("{} buy order @Market", material.name)),
                    ));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[measured]
pub fn execute_orders(
    mut commands: Commands,
    mut buy_orders: Query<(Entity, &BuyOrder)>,
    mut sell_orders: Query<(Entity, &SellOrder)>,
    mut trade_participants: Query<(Entity, &Name, &mut Wallet, &mut TransactionLog)>,
    mut buy_strategy: Query<(Entity, &mut BuyStrategy)>,
    mut items: Query<(Entity, &mut Item)>,
    date: Res<Days>,
) {
    let mut rng = rand::thread_rng();
    let mut already_sold = HashSet::new();

    // Iterate over each buy order
    for (buy_order_id, buy_order) in buy_orders.iter_mut() {
        let matching_sell_orders: Vec<_> = sell_orders
            .iter_mut()
            .filter(|(order_id, sell_order)| {
                sell_order.item_type == buy_order.item_type && !already_sold.contains(order_id)
            }) // Match by material
            .collect();

        if !matching_sell_orders.is_empty() {
            // Take a random sample
            let sample_size = (matching_sell_orders.len() as f64 * 0.1).ceil() as usize; // 10% for example
            let sampled_orders: Vec<_> = matching_sell_orders
                .choose_multiple(&mut rng, sample_size)
                .cloned()
                .collect();
            debug!(
                "I have {} sell orders to choose from for {}, prices: ({})",
                sampled_orders.len(),
                buy_order.item_type.name,
                sampled_orders
                    .iter()
                    .map(|(_, sell_order)| sell_order.price.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            // Sort by price ascending
            let mut sorted_sample = sampled_orders;
            sorted_sample.sort_by(|(_, a), (_, b)| a.price.cmp(&b.price));
            // Get the p-th percentile order (for example, the lowest price, which would be the first one after sorting)
            let p = 0.25; // for example, let's assume you want to get the 25th percentile order
            let index = ((sorted_sample.len() - 1) as f64 * p).round() as usize;
            if index >= sorted_sample.len() {
                panic!(
                    "Index {} is out of bounds for sample of size {}",
                    index,
                    sorted_sample.len()
                );
            }
            debug!(
                "I'm paying {} for {} (best price was {}) (index: {})!",
                sorted_sample[index].1.price,
                buy_order.item_type.name,
                sorted_sample.first().unwrap().1.price,
                index
            );
            if let Some(&(sell_order_id, sell_order)) = sorted_sample.get(index) {
                match buy_order.order {
                    OrderType::Market => {
                        if execute_order(
                            &mut buy_strategy,
                            &mut trade_participants,
                            &mut commands,
                            (sell_order_id, sell_order),
                            (buy_order_id, buy_order),
                            &mut items,
                            &date,
                        ) {
                            already_sold.insert(sell_order_id);
                        }
                    } // OrderType::Limit(max_price) => {
                      //     if sell_order.price <= max_price {
                      //         execute_order(
                      //             &mut buy_strategy,
                      //             &mut manufacturers,
                      //             &mut commands,
                      //             (sell_order_id, sell_order),
                      //             (buy_order_id, buy_order),
                      //             &mut items,
                      //         );
                      //     }
                      // }
                }
            }
        } else {
            trace!(
                "No sell orders for {} (buy order: {:?})",
                buy_order.item_type.name,
                buy_order
            );
        }
    }
}

fn execute_order(
    buy_strategy: &mut Query<(Entity, &mut BuyStrategy)>,
    trade_participants: &mut Query<(Entity, &Name, &mut Wallet, &mut TransactionLog)>,
    commands: &mut Commands,
    sell_order: (Entity, &SellOrder),
    buy_order: (Entity, &BuyOrder),
    items: &mut Query<(Entity, &mut Item)>,
    _date: &Res<Days>,
) -> bool {
    let (sell_order_id, sell_order) = sell_order;
    let (buy_order_id, buy_order) = buy_order;
    let sell_item_type = &sell_order.item_type;
    let buy_item_type = &buy_order.item_type;

    // Assume that the item type in the sell order is same as the buy order
    assert_eq!(buy_item_type, sell_item_type);

    // Phase 1: Do all the checks and computations
    if let Ok((_, name, wallet, _)) = trade_participants.get_mut(buy_order.buyer) {
        if wallet.money < sell_order.price {
            debug!(
                "{}: Cannot execute the order: buyer does not have enough money",
                name
            );
            return false;
        }
    } else {
        warn!("Buyer does not exist");
    }
    // if let Ok((_, mut seller_manufacturer)) = manufacturers.get_mut(sell_order.owner) {
    //     if let Some(_) = seller_manufacturer.assets.items_to_sell.take(&sell_order.item) {
    //         seller_has_item = true;
    //     }
    // }

    // Phase 2: Execute the operations
    if let Ok((_, name, mut wallet, mut transaction_log)) =
        trade_participants.get_mut(buy_order.buyer)
    {
        // Transfer money from buyer to seller
        wallet.money -= sell_order.price;
        if let Ok((_, mut strategy)) = buy_strategy.get_mut(buy_order.buyer) {
            *strategy
                .outstanding_orders
                .get_mut(&buy_order.item_type)
                .unwrap() -= 1;
        }

        // Transfer item from seller to buyer
        if let Ok((_, mut item)) = items.get_mut(sell_order.item) {
            item.buy_cost = sell_order.price;
        }
        transaction_log.unprocessed_transactions.push(Transaction {
            transaction_type: TransactionType::Buy,
            buyer: buy_order.buyer,
            seller: sell_order.seller,
            item: sell_order.item,
            item_type: sell_order.item_type.clone(),
            // price: sell_order.price,
            // date: date.days,
        });
        commands.entity(buy_order_id).despawn();
        debug!(
            "{}: !!!! Executed order: {:?} -> {:?} (buyer got his goods)",
            name, buy_order, sell_order
        );
    } else {
        warn!("Buyer does not exist");
    }
    if let Ok((_, name, mut wallet, mut transaction_log)) =
        trade_participants.get_mut(sell_order.seller)
    {
        // Add money to seller
        wallet.money += sell_order.price;
        transaction_log.unprocessed_transactions.push(Transaction {
            transaction_type: TransactionType::Sell,
            buyer: buy_order.buyer,
            seller: sell_order.seller,
            item: sell_order.item,
            item_type: sell_order.item_type.clone(),
            // price: sell_order.price,
            // date: date.days,
        });
        commands.entity(sell_order_id).despawn();
        debug!(
            "{}: !!!! Executed order: {:?} -> {:?} (seller got his money)",
            name, sell_order, buy_order
        );
        return true;
    } else {
        warn!("Seller does not exist");
    }
    false
}

pub fn process_transactions(
    mut transaction_logs: Query<(Entity, &mut TransactionLog)>,
    mut manufacturers: Query<(Entity, &mut Manufacturer)>,
    mut people: Query<(Entity, &mut Person)>,
) {
    for (_, mut transaction_log) in transaction_logs.iter_mut() {
        let mut unprocessed_transactions = transaction_log.unprocessed_transactions.clone();
        for transaction in transaction_log.unprocessed_transactions.iter() {
            match transaction.transaction_type {
                TransactionType::Buy => {
                    if let Ok((_, mut person)) = people.get_mut(transaction.buyer) {
                        person
                            .assets
                            .items
                            .entry(transaction.item_type.clone())
                            .or_default()
                            .push(transaction.item);
                    }
                    if let Ok((_, mut manufacturer)) = manufacturers.get_mut(transaction.buyer) {
                        manufacturer
                            .assets
                            .items
                            .entry(transaction.item_type.clone())
                            .or_default()
                            .push(transaction.item);
                    }
                }
                TransactionType::Sell => {
                    if let Ok((_, mut manufacturer)) = manufacturers.get_mut(transaction.seller) {
                        manufacturer.assets.items_to_sell.remove(&transaction.item);
                    }
                }
            }
        }
        transaction_log
            .history
            .append(&mut unprocessed_transactions);
        transaction_log.unprocessed_transactions.clear();
    }
}

pub fn salary_payout(
    mut workers: Query<(Entity, &mut Wallet, &Worker), Without<Manufacturer>>,
    mut manufacturers: Query<(Entity, &Name, &mut Wallet, &Manufacturer), Without<Worker>>,
) {
    for (_, name, mut manufacturer_wallet, manufacturer) in manufacturers.iter_mut() {
        for worker in manufacturer.hired_workers.iter() {
            if let Ok((_, mut worker_wallet, worker)) = workers.get_mut(*worker) {
                if worker.salary > manufacturer_wallet.money {
                    debug!(
                        "{}: Cannot pay salary to worker. Has {} left but salary is {}",
                        name, manufacturer_wallet.money, worker.salary
                    );
                } else {
                    worker_wallet.money += worker.salary;
                    manufacturer_wallet.money -= worker.salary;
                }
            }
        }
    }
}

pub fn order_expiration(mut buy_orders: Query<(Entity, &mut BuyOrder)>, mut commands: Commands) {
    for (buy_order_id, mut buy_order) in buy_orders.iter_mut() {
        if let Some(expiration) = buy_order.expiration {
            if expiration == 0 {
                info!("Order expired: {:?}", buy_order);
                commands.entity(buy_order_id).despawn();
            } else {
                buy_order.expiration = Some(expiration - 1);
            }
        }
    }
}
