use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

use bevy::prelude::*;
use either::Either;
use rand::distributions::{Distribution, WeightedIndex};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use macros::measured;

use crate::government::BusinessPermit;
use crate::init::{ProductionCycleTemplate, Templates};
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
    pub(crate) items: HashMap<ItemType, Vec<Item>>,
    pub(crate) items_to_sell: Vec<Item>,
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
    pub(crate) days_since_last_staff_change: u32,
    pub(crate) production_log: VecDeque<ProductionLog>,
    pub owner: Entity,
}

impl Manufacturer {
    pub fn has_enough_input(&self) -> bool {
        for (item_type, count) in &self.production_cycle.input {
            if self.assets.items.get(item_type).unwrap_or(&vec![]).len() < (*count as usize) {
                return false;
            }
        }
        true
    }
}

#[derive(Component, Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Worker {
    pub(crate) salary: Money,
    pub(crate) employed_at: Option<Entity>,
}

#[derive(Debug, Clone)]
pub struct Item {
    item_type: ItemType,
    production_cost: Money,
    buy_cost: Money,
}

#[derive(Component, Debug, Clone)]
pub struct SellOrder {
    pub(crate) items: Vec<Item>,
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

#[derive(Component, Copy, Clone, Debug, Serialize, Deserialize, Default)]
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

#[derive(Component, Debug, Clone)]
pub struct JobOffer {
    pub salary: Money,
    pub employer: Entity,
    pub taken_by: Option<Entity>,
}

#[derive(Component, Clone, Default)]
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
    workers_query: Query<&Worker>,
    date: Res<Days>,
) {
    for (wallet, mut manufacturer) in manufacturers.iter_mut() {
        // fill production cycle
        // produce_for_manufacturer(&mut b, commands, &production_cost);
        execute_production_cycle(&mut manufacturer, wallet, &workers_query, &date)
    }
}

fn execute_production_cycle(
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
                let mut buy_costs = Money(0);
                for (input_material, quantity_needed) in input.iter() {
                    // drain the quantity needed from the inventory and sum up costs
                    let item_costs: Money = manufacturer
                        .assets
                        .items
                        .get_mut(input_material)
                        .unwrap()
                        .drain(..*quantity_needed as usize)
                        .map(|item| item.buy_cost)
                        .sum::<Money>();
                    buy_costs += item_costs;
                }
                let (output_material, quantity_produced) =
                    &manufacturer.production_cycle.output.clone();
                let unit_cost = buy_costs / (*quantity_produced)
                    + cost_per_day * manufacturer.production_cycle.workdays_needed
                        / (*quantity_produced);
                for _ in 0..*quantity_produced {
                    let output_item = Item {
                        item_type: output_material.clone(),
                        production_cost: unit_cost,
                        buy_cost: Money(0),
                    };
                    debug!("Produced {:?}", output_item);
                    manufacturer.assets.items_to_sell.push(output_item);
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
        cost_per_day += workers_query.get(*worker).map_or(Money(0), |w| w.salary);
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
    mut logs: EventWriter<LogEvent>,
) {
    for (seller, mut manufacturer, mut strategy) in manufacturers.iter_mut() {
        let amount_to_sell = (manufacturer.assets.items_to_sell.len()
            * manufacturer.hired_workers.len())
            / manufacturer.production_cycle.workdays_needed as usize;
        debug!(
            "Creating sell orders for {} items from {}",
            amount_to_sell,
            manufacturer.assets.items_to_sell.len()
        );
        let items_to_sell = manufacturer
            .assets
            .items_to_sell
            .drain(..amount_to_sell)
            .collect::<Vec<Item>>();
        debug!(
            "Creating sell orders for {} items from (size after draining: {})",
            items_to_sell.len(),
            manufacturer.assets.items_to_sell.len()
        );
        if let Some(first_item) = items_to_sell.get(0) {
            let item_name = first_item.item_type.name.clone();
            strategy.base_price = first_item.production_cost;
            if strategy.current_price == Money(0) {
                strategy.current_price = first_item.production_cost;
                logs.send(LogEvent::Generic {
                    text: format!(
                        "I'm just starting, setting the price for {} to production cost: {}",
                        first_item.item_type.name.as_str(),
                        strategy.current_price
                    ),
                    entity: seller,
                });
            }
            let sell_order = SellOrder {
                items: items_to_sell.to_vec(),
                item_type: first_item.item_type.clone(),
                seller,
                price: strategy.current_price,
                base_price: strategy.base_price,
            };
            debug!(
                "Created sell order {:?} for {} with total {} items",
                sell_order,
                item_name,
                sell_order.items.len()
            );
            let strategy_copy = *strategy;
            commands.spawn((
                sell_order,
                Name::new(format!("{} sell order", item_name)),
                strategy_copy,
            ));
        }
    }
}

pub fn merge_sell_orders(mut sell_orders: Query<(Entity, &mut SellOrder)>) {
    // Map from seller to (first_order_entity, accumulated_items)
    let mut order_map: HashMap<Entity, (Entity, Vec<Item>)> = HashMap::new();

    for (order_entity, mut sell_order) in sell_orders.iter_mut() {
        match order_map.get_mut(&sell_order.seller) {
            Some((_, first_order_items)) => {
                // Accumulate items and despawn if not the first order.
                first_order_items.append(&mut sell_order.items);
            }
            None => {
                // This is the first order from this seller, remember it.
                order_map.insert(sell_order.seller, (order_entity, sell_order.items.clone()));
            }
        }
    }

    // Update the first order of each seller with the accumulated items.
    for (_, (first_order_entity, first_order_items)) in order_map {
        if let Ok((_, mut sell_order)) = sell_orders.get_mut(first_order_entity) {
            debug!(
                "Setting items of sell order {:?} to {:?}",
                sell_order, first_order_items
            );
            sell_order.items = first_order_items;
        }
    }
}

pub fn delete_empty_sell_orders(mut commands: Commands, sell_orders: Query<(Entity, &SellOrder)>) {
    for (sell_order_entity, sell_order) in sell_orders.iter() {
        if sell_order.items.is_empty() {
            commands.entity(sell_order_entity).despawn_recursive();
        }
    }
}

#[measured]
pub fn update_sell_order_prices(
    mut sell_orders: Query<(Entity, &Name, &mut SellOrder)>,
    sell_strategies: Query<&SellStrategy>,
) {
    for (_, name, mut sell_order) in sell_orders.iter_mut() {
        // startegy many not exist anymore when the business went bankrupt, he sells at the base price
        if let Ok(sell_strategy) = sell_strategies.get(sell_order.seller) {
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
        let upper_bound = 0.8;
        let selling_ratio = sold_items as f32 / produced_items as f32;
        let change = if selling_ratio < lower_bound {
            let change =
                1.0 - (lower_bound - selling_ratio) * sell_strategy.max_price_change_per_day;
            logs.send(LogEvent::Generic { text: format!("I'm selling too slow! Time to decrease price to {} (ratio {:.2}, change {:.2}%)", sell_strategy.current_price, selling_ratio, 100.0 * change), entity: seller });
            change
        } else if selling_ratio > upper_bound {
            let mut change =
                (selling_ratio - upper_bound).min(1.0) * sell_strategy.max_price_change_per_day;
            if sell_strategy.current_price < sell_strategy.base_price {
                change *= 10.0;
            }
            change += 1.0;
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

#[allow(clippy::too_many_arguments)]
pub fn create_business(
    mut people: Query<(Entity, &mut Person)>,
    mut wallets: Query<&mut Wallet>,
    workers: Query<&Worker>,
    templates: Res<Templates>,
    business_permits: Query<(Entity, &BusinessPermit)>,
    manufacturers: Query<(Entity, &Manufacturer)>,
    buy_orders: Query<&BuyOrder>,
    mut commands: Commands,
    mut logs: EventWriter<LogEvent>,
    date: Res<Days>,
    config: Res<Config>,
) {
    let demand = buy_orders
        .iter()
        .fold(HashMap::new(), |mut acc, buy_order| {
            *acc.entry(buy_order.item_type.clone()).or_insert(0) += 1;
            acc
        });
    let unemployed = people
        .iter_mut()
        .filter(|(person, _)| workers.get(*person).is_err())
        .count();
    if unemployed == 0 {
        return;
    }
    let last_days = config.business.prices.sell_history_to_consider.value;
    let sells_in_last_days =
        manufacturers
            .iter()
            .fold(HashMap::new(), |mut acc, (entity, manufacturer)| {
                let manufacturer_wallet = wallets.get(entity).unwrap();
                let sells = manufacturer_wallet.get_amount_of_sell_transactions(
                    date.days,
                    &manufacturer.production_cycle.output.0,
                    last_days,
                );
                *acc.entry(&manufacturer.production_cycle.output.0)
                    .or_insert(0) += sells;
                acc
            });
    for (permit, _) in business_permits.iter() {
        for (entity, _) in people.iter_mut() {
            let mut wallet = wallets.get_mut(entity).unwrap();
            if wallet.money() > Money::from_str("100k").unwrap() {
                if let Some(cycle) = choose_best_business(
                    &demand,
                    &sells_in_last_days,
                    &manufacturers,
                    &templates.production_cycles,
                ) {
                    logs.send(LogEvent::Generic {
                        text: format!("I'm creating a business for {}", cycle.output.0.as_str()),
                        entity,
                    });
                    let mut new_wallet = Wallet::default();
                    let business_id = commands
                        .spawn((
                            Manufacturer {
                                production_cycle: cycle.to_production_cycle().1,
                                hired_workers: vec![],
                                assets: Inventory::default(),
                                production_log: VecDeque::new(),
                                days_since_last_staff_change: 0,
                                owner: entity,
                            },
                            Name::new(format!("{} factory", cycle.output.0.as_str())),
                            SellStrategy {
                                max_price_change_per_day: config
                                    .business
                                    .prices
                                    .max_change_per_day
                                    .value,
                                ..Default::default()
                            },
                            BuyStrategy {
                                target_production_cycles: config
                                    .business
                                    .keep_resources_for_cycles_amount
                                    .value,
                                ..Default::default()
                            },
                        ))
                        .id();
                    wallet
                        .transaction(
                            &mut new_wallet,
                            &Transaction::Transfer {
                                side: TradeSide::Pay,
                                sender: entity,
                                receiver: business_id,
                                amount: config.business.money_to_create_business.value,
                                date: date.days,
                            },
                            &mut logs,
                        )
                        .unwrap(); // this must work as we check for money above
                    commands.entity(business_id).insert(new_wallet);
                    commands.entity(permit).despawn();
                    break;
                }
            }
        }
    }
}

fn choose_best_business<'a>(
    demand: &HashMap<ItemType, usize>,
    sells: &HashMap<&ItemType, usize>,
    manufacturers: &Query<(Entity, &Manufacturer)>,
    cycles: &'a Vec<ProductionCycleTemplate>,
) -> Option<&'a ProductionCycleTemplate> {
    let demand_count_by_item_type = demand.iter().fold(
        HashMap::new(),
        |mut acc: HashMap<ItemType, usize>, (item_type, count)| {
            *acc.entry(item_type.clone()).or_insert(0) += count;
            acc
        },
    );
    debug!("{:?}", demand_count_by_item_type);
    let manufacturers_count_by_item_type = manufacturers.iter().fold(
        HashMap::new(),
        |mut acc: HashMap<ItemType, usize>, (_, manufacturer)| {
            *acc.entry(manufacturer.production_cycle.output.0.clone())
                .or_insert(0) += 1;
            acc
        },
    );
    cycles.iter().map(
        |cycle| {
            let demand_exists = demand_count_by_item_type
                .get(&ItemType { name: cycle.output.0.clone() })
                .unwrap_or(&0).min(&(1_usize));
            let sells = sells.get(&ItemType { name: cycle.output.0.clone() }).unwrap_or(&0);
            let extreme_demand = demand_count_by_item_type
                .get(&ItemType { name: cycle.output.0.clone() })
                .unwrap_or(&0) > sells && sells > &0_usize;
            let count_by_manufacturers = manufacturers_count_by_item_type
                .get(&ItemType { name: cycle.output.0.clone() })
                .unwrap_or(&0);
            let extreme_demand_bonus = if extreme_demand { 5 } else { 0 };
            let process_complexity = find_required_inputs(&cycle.output.0, cycles);
            let complexity_risk = 0;//process_complexity.len();
            let missing_input_risk = process_complexity.iter().fold(0, |acc, input| {
                if manufacturers_count_by_item_type.contains_key(&ItemType { name: input.clone() }) {
                    acc
                } else {
                    acc + 1
                }
            });

            let risk = extreme_demand_bonus + *demand_exists as i32 - *count_by_manufacturers as i32 - complexity_risk - missing_input_risk;
            debug!("Risk calculation for {} = {}: extreme_demand: {}, demand exists: {} competition size: {} process_complexity: {} missing input: {}", cycle.output.0.as_str(), risk, extreme_demand, demand_exists, count_by_manufacturers, complexity_risk, missing_input_risk);
            (cycle, risk)
        }).max_by_key(|(_, count)| *count).map(|(cycle, _)| cycle)
}

#[allow(clippy::too_many_arguments)]
#[measured]
pub fn bankruption(
    manufacturers: Query<(Entity, &Name, &Manufacturer)>,
    mut wallets: Query<&mut Wallet>,
    mut sell_orders: Query<&mut SellOrder>,
    buy_orders: Query<(Entity, &BuyOrder)>,
    mut logs: EventWriter<LogEvent>,
    mut commands: Commands,
    date: Res<Days>,
    config: Res<Config>,
) {
    for (entity, name, manufacturer) in manufacturers.iter() {
        // TODO change to something better after implementing better job market system
        let [mut manufacturer_wallet, mut owner_wallet] =
            wallets.get_many_mut([entity, manufacturer.owner]).unwrap();
        if manufacturer_wallet.money() < config.business.new_worker_salary.value {
            info!("{} is bankrupt", name.as_str());
            sell_orders
                .iter_mut()
                .filter(|sell_order| sell_order.seller == entity)
                .for_each(|mut sell_order| {
                    sell_order.seller = manufacturer.owner;
                    sell_order.price = sell_order.base_price;
                });
            let amount = manufacturer_wallet.money();
            manufacturer_wallet
                .transaction(
                    &mut owner_wallet,
                    &Transaction::Transfer {
                        side: TradeSide::Pay,
                        sender: entity,
                        receiver: manufacturer.owner,
                        amount,
                        date: date.days,
                    },
                    &mut logs,
                )
                .unwrap();
            logs.send(LogEvent::Generic {
                text: format!(
                    "My business {} is bankrupt! I'll sell all stuff by production price.",
                    name.as_str()
                ),
                entity,
            });
            buy_orders
                .iter()
                .filter(|(_, buy_order)| buy_order.buyer == entity)
                .for_each(|(order_entity, _)| {
                    commands.entity(order_entity).despawn_recursive();
                });
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn payout_dividends(
    manufacturers: Query<(Entity, &Manufacturer)>,
    // people: Query<(Entity, &Name, &Person)>,
    mut wallets: Query<&mut Wallet>,
    mut logs: EventWriter<LogEvent>,
    date: Res<Days>,
    config: Res<Config>,
) {
    let dividend = config.business.monthly_dividend.value;
    for (owned_business, manufacturer) in manufacturers.iter() {
        let [mut manufacturer_wallet, mut owner_wallet] = wallets
            .get_many_mut([owned_business, manufacturer.owner])
            .unwrap();
        if let Either::Right(money) = manufacturer_wallet.calculate_total_change(date.days, 30) {
            if manufacturer_wallet.money() > money * dividend {
                // let (_, owner_name, owner) = people.get(manufacturer.owner).unwrap();
                manufacturer_wallet
                    .transaction(
                        &mut owner_wallet,
                        &Transaction::Transfer {
                            side: TradeSide::Pay,
                            sender: owned_business,
                            receiver: manufacturer.owner,
                            amount: money * dividend,
                            date: date.days,
                        },
                        &mut logs,
                    )
                    .unwrap();
            }
        }
    }
}

fn find_required_inputs(
    cycle_output: &String,
    global_cycles: &Vec<ProductionCycleTemplate>,
) -> HashSet<String> {
    let mut required_inputs = HashSet::new();

    for cycle in global_cycles {
        if cycle.output.0 == *cycle_output {
            for input_item in cycle.input.keys() {
                required_inputs.insert(input_item.clone());

                // If there is a cycle for this input, recursively find its inputs
                let sub_inputs = find_required_inputs(input_item, global_cycles);
                for sub_input in sub_inputs {
                    required_inputs.insert(sub_input);
                }
            }
        }
    }

    required_inputs
}

pub fn create_job_offers(
    mut manufacturers: Query<(Entity, &mut Manufacturer, &SellStrategy)>,
    jobs: Query<&JobOffer>,
    mut logs: EventWriter<LogEvent>,
    mut commands: Commands,
    config: Res<Config>,
) {
    for (manufacturer, manufacturer_data, sell_strategy) in manufacturers.iter_mut() {
        let total_offers = jobs
            .iter()
            .filter(|job| job.employer == manufacturer)
            .count();
        if ((manufacturer_data.hired_workers.len()
            < manufacturer_data.production_cycle.workdays_needed as usize
            && sell_strategy.current_price > sell_strategy.base_price * 2)
            || (manufacturer_data.hired_workers.is_empty() && manufacturer_data.has_enough_input()))
            && total_offers == 0
            && manufacturer_data.days_since_last_staff_change == 0
        {
            let salary = config.business.new_worker_salary.value;
            commands.spawn(JobOffer {
                salary,
                employer: manufacturer,
                taken_by: None,
            });
            logs.send(LogEvent::Generic {
                text: format!(
                    "I'm creating a job offer for {}. My current workers: {}",
                    salary,
                    manufacturer_data.hired_workers.len()
                ),
                entity: manufacturer,
            });
            warn!(
                "I'm creating a job offer for {}. My current workers: {}",
                salary,
                manufacturer_data.hired_workers.len()
            );
        }
    }
}

pub fn take_job_offers(
    jobs: Query<(Entity, &JobOffer)>,
    unemployed: Query<(Entity, &Person), Without<Worker>>,
    names: Query<&Name>,
    mut manufacturers: Query<(Entity, &mut Manufacturer)>,
    mut logs: EventWriter<LogEvent>,
    mut commands: Commands,
    config: Res<Config>,
) {
    let mut unemployed: Vec<(Entity, &Person)> = unemployed.iter().collect();
    for (job, offer) in jobs.iter() {
        if let Ok((manufacturer_entity, mut manufacturer)) = manufacturers.get_mut(offer.employer) {
            if let Some((person, _)) = unemployed.pop() {
                // somehow people are hired multiple times
                let worker_name = names.get(person).unwrap();
                let manufacturer_name = names.get(manufacturer_entity).unwrap();
                manufacturer.hired_workers.push(person);
                manufacturer.days_since_last_staff_change =
                    config.business.min_days_between_staff_change.value;
                commands.entity(person).insert(Worker {
                    salary: offer.salary,
                    employed_at: Some(offer.employer),
                });
                logs.send(LogEvent::Generic {
                    text: format!("I my job offer was taken by a worker {}!", worker_name),
                    entity: manufacturer_entity,
                });
                logs.send(LogEvent::Generic {
                    text: format!("I've taken job offer at {}!", manufacturer_name),
                    entity: person,
                });
                warn!(
                    "Job offer to work at {} taken by {}!",
                    manufacturer_name, worker_name
                );
                commands.entity(job).despawn();
            }
        } else {
            // employer no longer exists
            commands.entity(job).despawn();
        }
    }
}

pub fn reduce_days_since_last_staff_change(mut manufacturers: Query<&mut Manufacturer>) {
    for mut manufacturer in manufacturers.iter_mut() {
        if manufacturer.days_since_last_staff_change > 0 {
            manufacturer.days_since_last_staff_change -= 1;
        }
    }
}

pub fn fire_staff(
    mut manufacturers: Query<(Entity, &Wallet, &mut Manufacturer, &SellStrategy)>,
    workers: Query<(Entity, &Worker)>,
    sell_orders: Query<&SellOrder>,
    names: Query<&Name>,
    mut logs: EventWriter<LogEvent>,
    mut commands: Commands,
    config: Res<Config>,
) {
    let sell_orders_count_grouped_by_manufacturer = sell_orders
        .iter()
        .map(|sell_order| sell_order.seller)
        .fold(HashMap::new(), |mut acc, employer| {
            *acc.entry(employer).or_insert(0) += 1;
            acc
        });
    for (manufacturer, wallet, mut manufacturer_data, sell_strategy) in manufacturers.iter_mut() {
        if manufacturer_data.days_since_last_staff_change == 0
            && manufacturer_data.hired_workers.len() > 1
            && (sell_strategy.current_price < sell_strategy.base_price * 0.8
                || (sell_orders_count_grouped_by_manufacturer
                    .get(&manufacturer)
                    .unwrap_or(&0)
                    > &(manufacturer_data.production_cycle.output.1
                        * config.business.goal_produced_cycles_count.value)))
        {
            let worker = manufacturer_data.hired_workers.pop();
            if let Some(worker) = worker {
                let worker_name = names.get(worker).unwrap();
                let manufacturer_name = names.get(manufacturer).unwrap();
                manufacturer_data.days_since_last_staff_change =
                    config.business.min_days_between_staff_change.value;
                logs.send(LogEvent::Generic {
                    text: format!("I fired a worker {}!", worker_name),
                    entity: manufacturer,
                });
                logs.send(LogEvent::Generic {
                    text: format!("I was fired from {}!", manufacturer_name),
                    entity: worker,
                });
                // let (worker, mut worker_data) = workers.get_mut(worker).unwrap();
                // worker_data.fire();
                warn!(
                    "Firing worker {}, my current workers: {}",
                    worker_name,
                    manufacturer_data.hired_workers.len()
                );
                commands.entity(worker).remove::<Worker>();
            }
        }
        if wallet.money()
            < manufacturer_data
                .hired_workers
                .iter()
                .map(|&worker| {
                    workers
                        .get(worker)
                        .map_or(Money(0), |(_, worker)| worker.salary)
                })
                .sum::<Money>()
        {
            let worker = manufacturer_data.hired_workers.pop();
            if let Some(worker) = worker {
                let name = names.get(worker).unwrap();
                logs.send(LogEvent::Generic {
                    text: format!(
                        "I fired a worker {} because I can't afford to pay him!",
                        name
                    ),
                    entity: manufacturer,
                });
                logs.send(LogEvent::Generic {
                    text: format!(
                        "I was fired from {} because he could not afford to pay me!",
                        names.get(manufacturer).unwrap()
                    ),
                    entity: worker,
                });
                warn!(
                    "I fired a worker {} because I can't afford to pay him! My current workers: {}",
                    name,
                    manufacturer_data.hired_workers.len()
                );
                commands.entity(worker).remove::<Worker>();
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
    buy_orders: Query<(Entity, &BuyOrder)>,
    mut sell_orders: Query<(Entity, &mut SellOrder)>,
    mut trade_participants: Query<&mut Wallet>,
    mut buy_strategy: Query<(Entity, &mut BuyStrategy)>,
    mut logs: EventWriter<LogEvent>,
    mut manufacturers: Query<(Entity, &mut Manufacturer)>,
    mut people: Query<(Entity, &mut Person)>,
    date: Res<Days>,
    config: Res<Config>,
) {
    let mut rng = rand::thread_rng();

    // iterate buy orders in randomized order
    let mut buy_orders: Vec<_> = buy_orders.iter().collect();
    buy_orders.shuffle(&mut rng);
    // Iterate over each buy order
    for (buy_order_id, buy_order) in buy_orders.iter() {
        let matching_sell_orders: Vec<_> = sell_orders
            .iter()
            .filter(|(_, sell_order)| {
                sell_order.item_type == buy_order.item_type && !sell_order.items.is_empty()
            }) // Match by material
            .collect();

        if !matching_sell_orders.is_empty() {
            // Take a random sample
            let sample_size = (matching_sell_orders.len() as f64
                * config.business.market.amount_of_sell_orders_seen.value)
                .ceil() as usize; // 10% for example
            let sampled_orders: Vec<_> = choose_weighted_orders(&matching_sell_orders, sample_size);

            // Sort by price ascending
            let mut sorted_sample = sampled_orders;
            sorted_sample.sort_by(|(_, a), (_, b)| a.price.cmp(&b.price));
            let sampled_sell_order_ids =
                sorted_sample.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            debug!(
                "I have {} sell orders to choose from for {}, prices: ({})",
                sorted_sample.len(),
                buy_order.item_type.name,
                sorted_sample
                    .iter()
                    .map(|(_, sell_order)| sell_order.price.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            // randomly get one of the top 25% of prices
            let p = rng.gen_range(
                0.0..=config
                    .business
                    .market
                    .amount_of_sell_orders_to_choose_best_price_from
                    .value,
            );
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
            if let Some(sell_order_id) = sampled_sell_order_ids.get(index) {
                match buy_order.order {
                    OrderType::Market => {
                        let _ = execute_order(
                            &mut buy_strategy,
                            &mut trade_participants,
                            &mut commands,
                            sell_order_id,
                            &mut sell_orders,
                            (*buy_order_id, buy_order),
                            &mut logs,
                            &mut manufacturers,
                            &mut people,
                            &date,
                        );
                    }
                }
            }
        } else {
            debug!(
                "No sell orders for {} (buy order: {:?})",
                buy_order.item_type.name, buy_order
            );
        }
    }
}

fn choose_weighted_orders<'a>(
    items: &'a [(Entity, &'a SellOrder)],
    sample_size: usize,
) -> Vec<(Entity, &'a SellOrder)> {
    let mut rng = rand::thread_rng();
    // Create a WeightedIndex distribution with the order quantities as weights
    let weights: Vec<_> = items
        .iter()
        .map(|(_, sell_order)| sell_order.items.len())
        .collect();
    let dist = WeightedIndex::new(weights).unwrap();

    // Sample from the distribution to get indices, and return the corresponding items
    (0..sample_size)
        .map(|_| items[dist.sample(&mut rng)])
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn execute_order(
    buy_strategy: &mut Query<(Entity, &mut BuyStrategy)>,
    trade_participants: &mut Query<&mut Wallet>,
    commands: &mut Commands,
    sell_order_id: &Entity,
    sell_orders: &mut Query<(Entity, &mut SellOrder)>,
    buy_order: (Entity, &BuyOrder),
    logs: &mut EventWriter<LogEvent>,
    manufacturers: &mut Query<(Entity, &mut Manufacturer)>,
    people: &mut Query<(Entity, &mut Person)>,
    date: &Res<Days>,
) -> Result<(), TransactionError> {
    // let (sell_order_id, &mut sell_order) = sell_order;
    let (_, mut sell_order) = sell_orders.get_mut(*sell_order_id).unwrap();
    let (buy_order_id, buy_order) = buy_order;
    // Assume that the item type in the sell order is same as the buy order
    assert_eq!(buy_order.item_type, sell_order.item_type);
    if sell_order.items.is_empty() {
        warn!("Sell order {} has quantity 0", sell_order.item_type);
        return Err(TransactionError::SellOrderEmpty);
    }

    let [mut buyer_wallet, mut seller_wallet] = trade_participants
        .get_many_mut([buy_order.buyer, sell_order.seller])
        .map_err(|_| TransactionError::WalletNotFound)?;

    let mut item_to_sell = sell_order.items.last().unwrap().clone();
    item_to_sell.buy_cost = sell_order.price;

    buyer_wallet.transaction(
        &mut seller_wallet,
        &Transaction::Trade {
            side: TradeSide::Pay,
            buyer: buy_order.buyer,
            seller: sell_order.seller,
            item: item_to_sell.clone(),
            item_type: sell_order.item_type.clone(),
            price: sell_order.price,
            date: date.days,
        },
        logs,
    )?;
    // we remove the item only if the transaction was successful
    sell_order.items.pop();
    if let Ok((_, mut strategy)) = buy_strategy.get_mut(buy_order.buyer) {
        *strategy
            .outstanding_orders
            .get_mut(&buy_order.item_type)
            .unwrap() -= 1;
    }
    commands.entity(buy_order_id).despawn();
    if sell_order.items.is_empty() {
        commands.entity(*sell_order_id).despawn();
    }
    if let Ok((_, mut person)) = people.get_mut(buy_order.buyer) {
        person
            .assets
            .items
            .entry(sell_order.item_type.clone())
            .or_default()
            .push(item_to_sell.clone());
    }
    if let Ok((_, mut manufacturer)) = manufacturers.get_mut(buy_order.buyer) {
        manufacturer
            .assets
            .items
            .entry(sell_order.item_type.clone())
            .or_default()
            .push(item_to_sell);
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
                debug!("Order expired: {:?}", buy_order);
                commands.entity(buy_order_id).despawn();
            } else {
                buy_order.expiration = Some(expiration - 1);
            }
        }
    }
}

pub fn assing_workers_to_businesses(
    mut workers: Query<(Entity, &mut Worker, &Person)>,
    manufacturers: Query<(Entity, &Manufacturer)>,
) {
    for (manufacturer_entity, manufacturer) in manufacturers.iter() {
        for (worker_entity, mut worker, _) in workers.iter_mut() {
            if manufacturer.hired_workers.contains(&worker_entity) {
                worker.employed_at = Some(manufacturer_entity);
            }
        }
    }
}
