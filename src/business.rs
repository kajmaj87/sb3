use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::fmt::Display;

use bevy::prelude::*;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use macros::measured;

use crate::logs::LogEvent;
use crate::money::Money;
use crate::people::Person;
use crate::ui::debug::Performance;
use crate::wallet::{TradeSide, Transaction, TransactionError, Wallet};
use crate::Days;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Ord, PartialOrd, Deserialize)]
pub struct ItemType {
    pub(crate) name: String,
}

impl Display for ItemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProductionCycle {
    pub input: HashMap<ItemType, u32>,
    pub output: (ItemType, u32),
    pub workdays_needed: u32,
    pub workdays_left: u32,
}

impl Display for ProductionCycle {
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

#[derive(Bundle)]
pub struct ManufacturerBundle {
    pub name: Name,
    pub manufacturer: Manufacturer,
    pub sell_strategy: SellStrategy,
    pub wallet: Wallet,
}

#[derive(Debug)]
pub struct ProductionLog {
    date: usize,
}

#[derive(Component, Debug)]
pub struct Manufacturer {
    pub(crate) production_cycle: ProductionCycle,
    pub(crate) assets: Inventory,
    pub(crate) hired_workers: Vec<Entity>,
    pub(crate) delay_to_fire_next_worker: u32,
    pub(crate) production_log: VecDeque<ProductionLog>,
}

#[derive(Component, Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Worker {
    pub(crate) salary: Money,
    pub(crate) employed_at: Option<Entity>,
}

#[derive(Component, Debug)]
pub struct Item {
    item_type: ItemType,
    production_cost: Money,
    buy_cost: Money,
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
    pub(crate) max_price_change_per_day: f32,
    #[serde(skip)]
    pub(crate) current_price: Money,
    #[serde(skip)]
    pub(crate) base_price: Money,
}

#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
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
    mut manufacturers: Query<(&Wallet, &mut Manufacturer)>,
    mut commands: Commands,
    workers_query: Query<&Worker>,
    date: Res<Days>,
) {
    for (wallet, mut manufacturer) in manufacturers.iter_mut() {
        // fill production cycle
        // produce_for_manufacturer(&mut b, commands, &production_cost);
        execute_production_cycle(
            &mut commands,
            &mut manufacturer,
            wallet,
            &workers_query,
            &date,
        )
    }
}

fn execute_production_cycle(
    commands: &mut Commands,
    manufacturer: &mut Mut<Manufacturer>,
    wallet: &Wallet,
    workers_query: &Query<&Worker>,
    date: &Res<Days>,
) {
    match work_on_cycle_possible(wallet, manufacturer, workers_query) {
        Ok(cost_per_day) => {
            if manufacturer.production_cycle.workdays_left > manufacturer.hired_workers.len() as u32
            {
                // Continue the existing cycle
                manufacturer.production_cycle.workdays_left -=
                    manufacturer.hired_workers.len() as u32;
            } else {
                // Start a new cycle
                let input = manufacturer.production_cycle.input.clone();
                for (input_material, quantity_needed) in input.iter() {
                    for _ in 0..*quantity_needed {
                        let item = manufacturer
                            .assets
                            .items
                            .get_mut(input_material)
                            .unwrap()
                            .pop()
                            .unwrap();
                        commands.entity(item).despawn_recursive();
                    }
                }
                let (output_material, quantity_produced) =
                    &manufacturer.production_cycle.output.clone();
                let unit_cost = cost_per_day * manufacturer.production_cycle.workdays_needed
                    / (*quantity_produced);
                for _ in 0..*quantity_produced {
                    let item = Item {
                        item_type: output_material.clone(),
                        production_cost: unit_cost,
                        buy_cost: Money(0),
                    };
                    debug!("Produced {:?}", item);
                    let output_item = commands
                        .spawn((item, Name::new(output_material.name.to_string())))
                        .id();
                    manufacturer.assets.items_to_sell.insert(output_item);
                    manufacturer
                        .production_log
                        .push_front(ProductionLog { date: date.days });
                }
                manufacturer.production_cycle.workdays_left =
                    manufacturer.production_cycle.workdays_needed;
            }
        }
        Err(e) => match e {
            MaxCycleError::NoMaterialInInventory(material) => {
                debug!("No material {} in inventory, can't work on cycle", material);
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

fn work_on_cycle_possible(
    wallet: &Wallet,
    manufacturer: &Mut<Manufacturer>,
    workers_query: &Query<&Worker>,
) -> Result<Money, MaxCycleError> {
    for (input_material, &quantity_needed) in manufacturer.production_cycle.input.iter() {
        if let Some(items_in_inventory) = manufacturer.assets.items.get(input_material) {
            if items_in_inventory.len() < quantity_needed as usize {
                debug!(
                    "Not enough material {:?} in inventory, work on cycle not possible",
                    input_material
                );
                return Err(MaxCycleError::NoMaterialInInventory(
                    input_material.name.to_string(),
                ));
            }
        } else {
            debug!(
                "No material {:?} in inventory, work on cycle not possible",
                input_material
            );
            return Err(MaxCycleError::NoMaterialInInventory(
                input_material.name.to_string(),
            ));
        }
    }

    if manufacturer.hired_workers.is_empty() {
        debug!("Not enough workers to work on a cycle, nothing will be produced");
        return Err(MaxCycleError::NotEnoughMaterialsOrWorkers);
    }

    // Calculate the cost for one day of work
    let mut cost_per_day = Money(0);
    for worker in manufacturer.hired_workers.iter() {
        cost_per_day += workers_query.get(*worker).unwrap().salary;
    }
    debug!("Salaries cost per day: {}", cost_per_day);

    if wallet.money() < cost_per_day {
        debug!("Dear Lord, we can't even pay our workers, we're doomed!");
        return Err(MaxCycleError::CantPayWorkers);
    }

    Ok(cost_per_day)
}

#[measured]
pub fn create_sell_orders(
    mut commands: Commands,
    mut manufacturers: Query<(Entity, &mut Manufacturer, &mut SellStrategy)>,
    items_query: Query<(&Name, &Item)>,
) {
    for (seller, mut manufacturer, mut strategy) in manufacturers.iter_mut() {
        let mut items_to_sell = vec![];
        let amount_to_sell = (manufacturer.assets.items_to_sell.len()
            * manufacturer.hired_workers.len())
            / manufacturer.production_cycle.workdays_needed as usize;
        for item in manufacturer
            .assets
            .items_to_sell
            .iter()
            .take(amount_to_sell)
        {
            items_to_sell.push(*item);
        }
        for item in items_to_sell {
            if let Ok((name, item_cost)) = items_query.get(item) {
                strategy.base_price = item_cost.production_cost;
                if strategy.current_price == Money(0) {
                    strategy.current_price = item_cost.production_cost;
                }
                let sell_order = SellOrder {
                    item,
                    item_type: item_cost.item_type.clone(),
                    seller,
                    price: strategy.current_price,
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
    mut sell_orders: Query<(Entity, &Name, &mut SellOrder)>,
    sell_strategies: Query<&SellStrategy>,
) {
    for (_, name, mut sell_order) in sell_orders.iter_mut() {
        let sell_strategy = sell_strategies.get(sell_order.seller).unwrap();
        sell_order.price = sell_strategy.current_price;
        debug!(
            "Updated {} sell order price to {}",
            name.as_str(),
            sell_order.price
        );
        if sell_strategy.current_price < sell_order.base_price {
            debug!("Oh my god, we're selling {} at a loss!", name.as_str());
        }
    }
}

pub fn update_sell_strategy_margin(
    mut manufacturers: Query<(Entity, &mut SellStrategy, &Wallet, &Manufacturer)>,
    mut logs: EventWriter<LogEvent>,
    date: Res<Days>,
) {
    let days_to_look_at = 30;
    for (seller, mut sell_strategy, wallet, manufacturer) in manufacturers.iter_mut() {
        let sold_items = wallet.get_amount_of_sell_transactions(
            date.days,
            &manufacturer.production_cycle.output.0,
            days_to_look_at,
        );
        let produced_items = manufacturer
            .production_log
            .iter()
            .take_while(|log| date.days - log.date <= days_to_look_at)
            .count();
        logs.send(LogEvent::Generic {
            text: format!(
                "I'm thinking about my selling strategy, I've sold {} items and produced {} items.",
                sold_items, produced_items
            ),
            entity: seller,
        });
        if produced_items == 0 {
            continue;
        }
        let lower_bound = 0.5;
        let upper_bound = 0.9;
        let selling_ratio = sold_items as f32 / produced_items as f32;
        let change = if selling_ratio < lower_bound {
            let change =
                1.0 - (lower_bound - selling_ratio) * sell_strategy.max_price_change_per_day;
            logs.send(LogEvent::Generic { text: format!("I'm selling too slow! Time to decrease price to {} (ratio {:.2}, change {:.2}%)", sell_strategy.current_price, selling_ratio, 100.0 * change), entity: seller });
            change
        } else if selling_ratio > upper_bound {
            let change = 1.0
                + (selling_ratio - upper_bound).min(1.0) * sell_strategy.max_price_change_per_day;
            logs.send(LogEvent::Generic { text: format!("I'm selling too fast! Time to increase price to {} (ratio {:.2}, change {:.2}%)", sell_strategy.current_price, selling_ratio, 100.0 * change), entity: seller });
            change
            // sell_strategy.current_price -= change;
            // if sell_strategy.current_price < 0.3 {
            //     sell_strategy.current_price = 0.3;
            // } else {
            // }
        } else {
            logs.send(LogEvent::Generic {
                text: format!(
                    "I'm selling at a right price! {} (ratio {:.2}, change {:.2}%)",
                    sell_strategy.current_price, selling_ratio, 100.0
                ),
                entity: seller,
            });
            1.0
        };
        let old_price = sell_strategy.current_price;
        sell_strategy.current_price *= change;
        // ensure there is at least a little change in price
        if sell_strategy.current_price == old_price && change > 1.0 {
            sell_strategy.current_price += Money(1);
        }
        if sell_strategy.current_price == old_price
            && change < 1.0
            && sell_strategy.current_price > Money(1)
        {
            sell_strategy.current_price -= Money(1);
        }
    }
}

// pub fn hire_stuff(
//     mut manufacturers: Query<(Entity, &Wallet, &mut Manufacturer, &SellStrategy)>,
//     buy_orders: Query<&BuyOrder>,
// ) {
//     buy_orders.iter().map(|buy_order| {
//         let seller = buy_order.seller;
//         let item = buy_order.item_type;
//         (seller, item)
//     })
//     for (manufacturer, wallet, mut manufacturer_data, sell_strategy) in manufacturers.iter_mut() {
//         if manufacturer_data.hired_workers.len() < manufacturer_data.production_cycle.workdays_needed as usize &&  {
//
//         }
//     }
// }

pub fn fire_stuff(
    mut manufacturers: Query<(Entity, &Wallet, &mut Manufacturer, &SellStrategy)>,
    mut workers: Query<(Entity, &Name, &mut Worker)>,
    mut logs: EventWriter<LogEvent>,
) {
    for (manufacturer, wallet, mut manufacturer_data, sell_strategy) in manufacturers.iter_mut() {
        if manufacturer_data.delay_to_fire_next_worker == 0
            && sell_strategy.current_price < sell_strategy.base_price * 0.8
            && manufacturer_data.hired_workers.len() > 1
        {
            let worker = manufacturer_data.hired_workers.pop();
            if let Some(worker) = worker {
                let (_, name, _) = workers.get_mut(worker).unwrap();
                manufacturer_data.delay_to_fire_next_worker = 30;
                logs.send(LogEvent::Generic {
                    text: format!("I fired a worker {}!", name),
                    entity: manufacturer,
                });
            }
        } else if manufacturer_data.delay_to_fire_next_worker > 0 {
            manufacturer_data.delay_to_fire_next_worker -= 1;
        }
        if wallet.money()
            < manufacturer_data
                .hired_workers
                .iter()
                .map(|&worker| workers.get(worker).unwrap().2.salary)
                .sum::<Money>()
        {
            let worker = manufacturer_data.hired_workers.pop();
            if let Some(worker) = worker {
                let (_, name, _) = workers.get_mut(worker).unwrap();
                logs.send(LogEvent::Generic {
                    text: format!(
                        "I fired a worker {} because I can't afford to pay him!",
                        name
                    ),
                    entity: manufacturer,
                });
            }
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
    mut trade_participants: Query<&mut Wallet>,
    mut buy_strategy: Query<(Entity, &mut BuyStrategy)>,
    mut items: Query<(Entity, &mut Item)>,
    mut logs: EventWriter<LogEvent>,
    mut manufacturers: Query<(Entity, &mut Manufacturer)>,
    mut people: Query<(Entity, &mut Person)>,
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
                            &mut logs,
                            &mut manufacturers,
                            &mut people,
                            &date,
                        )
                        .is_ok()
                        {
                            already_sold.insert(sell_order_id);
                        }
                    }
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

#[allow(clippy::too_many_arguments)]
fn execute_order(
    buy_strategy: &mut Query<(Entity, &mut BuyStrategy)>,
    trade_participants: &mut Query<&mut Wallet>,
    commands: &mut Commands,
    sell_order: (Entity, &SellOrder),
    buy_order: (Entity, &BuyOrder),
    items: &mut Query<(Entity, &mut Item)>,
    logs: &mut EventWriter<LogEvent>,
    manufacturers: &mut Query<(Entity, &mut Manufacturer)>,
    people: &mut Query<(Entity, &mut Person)>,
    date: &Res<Days>,
) -> Result<(), TransactionError> {
    let (sell_order_id, sell_order) = sell_order;
    let (buy_order_id, buy_order) = buy_order;
    // Assume that the item type in the sell order is same as the buy order
    assert_eq!(buy_order.item_type, sell_order.item_type);

    let [mut buyer_wallet, mut seller_wallet] = trade_participants
        .get_many_mut([buy_order.buyer, sell_order.seller])
        .map_err(|_| TransactionError::WalletNotFound)?;

    buyer_wallet.transaction(
        &mut seller_wallet,
        &Transaction::Trade {
            side: TradeSide::Pay,
            buyer: buy_order.buyer,
            seller: sell_order.seller,
            item: sell_order.item,
            item_type: sell_order.item_type.clone(),
            price: sell_order.price,
            date: date.days,
        },
        logs,
    )?;
    if let Ok((_, mut strategy)) = buy_strategy.get_mut(buy_order.buyer) {
        *strategy
            .outstanding_orders
            .get_mut(&buy_order.item_type)
            .unwrap() -= 1;
    }
    if let Ok((_, mut item)) = items.get_mut(sell_order.item) {
        item.buy_cost = sell_order.price;
    }
    commands.entity(buy_order_id).despawn();
    commands.entity(sell_order_id).despawn();
    if let Ok((_, mut person)) = people.get_mut(buy_order.buyer) {
        person
            .assets
            .items
            .entry(sell_order.item_type.clone())
            .or_default()
            .push(sell_order.item);
    }
    if let Ok((_, mut manufacturer)) = manufacturers.get_mut(buy_order.buyer) {
        manufacturer
            .assets
            .items
            .entry(sell_order.item_type.clone())
            .or_default()
            .push(sell_order.item);
    }
    if let Ok((_, mut manufacturer)) = manufacturers.get_mut(sell_order.seller) {
        manufacturer.assets.items_to_sell.remove(&sell_order.item);
    }
    Ok(())
}

pub fn salary_payout(
    mut workers: Query<(Entity, &mut Wallet, &Worker), Without<Manufacturer>>,
    mut manufacturers: Query<(Entity, &mut Wallet, &Manufacturer), Without<Worker>>,
    mut logs: EventWriter<LogEvent>,
    date: Res<Days>,
) {
    for (employer, mut manufacturer_wallet, manufacturer) in manufacturers.iter_mut() {
        for worker in manufacturer.hired_workers.iter() {
            if let Ok((worker, mut worker_wallet, worker_data)) = workers.get_mut(*worker) {
                let _ = manufacturer_wallet.transaction(
                    &mut worker_wallet,
                    &Transaction::Salary {
                        side: TradeSide::Pay,
                        employer,
                        worker,
                        salary: worker_data.salary,
                        date: date.days,
                    },
                    &mut logs,
                );
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
