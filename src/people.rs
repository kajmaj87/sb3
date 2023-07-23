use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::{SliceRandom, ThreadRng};
use rand::Rng;
use serde::{Deserialize, Deserializer};

use macros::measured;

use crate::business::{BuyOrder, Inventory, ItemType, OrderType};
use crate::logs::LogEvent;
use crate::stats::PriceHistory;
use crate::ui::debug::Performance;
use crate::wallet::Wallet;

#[derive(Debug, Deserialize, Resource, Default, Clone)]
pub struct Names {
    first_names: Vec<String>,
    nicknames: Vec<String>,
    last_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Need {
    pub base: f64,
    pub preference: f64,
    #[serde(deserialize_with = "deserialize_item_type_map")]
    pub satisfied_by: HashMap<ItemType, f64>,
    #[serde(default, deserialize_with = "deserialize_optional_item_type_map")]
    pub increased_by: Option<HashMap<ItemType, f64>>,
}

#[derive(Resource, Default)]
pub struct Needs {
    pub needs: HashMap<ItemType, Need>,
}

#[derive(Deserialize, Debug)]
pub struct Item {
    consumption_rate: f64,
}

#[derive(Debug, Deserialize, Resource, Default)]
pub struct Items {
    pub items: HashMap<String, Item>,
}

impl Items {
    pub fn load(&mut self) {
        let items = std::fs::read_to_string("data/items.json").unwrap();
        let items: HashMap<String, Item> = serde_json::from_str(&items).unwrap();
        self.items = items;
    }
}

impl Needs {
    pub fn load(&mut self) {
        let needs = std::fs::read_to_string("data/needs.json").unwrap();
        let needs: HashMap<String, Need> = serde_json::from_str(&needs).unwrap();
        self.needs = needs
            .into_iter()
            .map(|(k, v)| (ItemType { name: k }, v))
            .collect();
    }
}

impl Names {
    pub fn load(&mut self) {
        let names = std::fs::read_to_string("data/names.json").unwrap();
        let names: Names = serde_json::from_str(&names).unwrap();
        self.first_names = names.first_names;
        self.nicknames = names.nicknames;
        self.last_names = names.last_names;
        let unique_names = self.first_names.len() as u64
            * self.nicknames.len() as u64
            * self.last_names.len() as u64;
        info!(
            "Loaded names. Possible unique combinations: {}",
            unique_names
        );
        info!("Name collision probabilities for n people: 10: {:.3}%, 100: {:.3}%, 1000: {:.3}%, 10000: {:.3}%", collision_probability(10, unique_names), collision_probability(100, unique_names), collision_probability(1000, unique_names), collision_probability(10000, unique_names));
    }
}

fn deserialize_item_type_map<'de, D>(deserializer: D) -> Result<HashMap<ItemType, f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let map: HashMap<String, f64> = HashMap::deserialize(deserializer)?;
    Ok(map
        .into_iter()
        .map(|(k, v)| (ItemType { name: k }, v))
        .collect())
}

fn deserialize_optional_item_type_map<'de, D>(
    deserializer: D,
) -> Result<Option<HashMap<ItemType, f64>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_map: Option<HashMap<String, f64>> = Option::deserialize(deserializer)?;
    match opt_map {
        Some(map) => Ok(Some(
            map.into_iter()
                .map(|(k, v)| (ItemType { name: k }, v))
                .collect(),
        )),
        None => Ok(None),
    }
}

fn collision_probability(samples: u64, unique_names: u64) -> f64 {
    (1.0 - (-(samples as f64) * ((samples - 1) as f64) / (2.0 * unique_names as f64)).exp()) * 100.0
}

pub(crate) fn generate_name(names: &Res<Names>) -> String {
    let mut rng = rand::thread_rng();
    let first_name = names.first_names.choose(&mut rng).unwrap();
    let nickname = names.nicknames.choose(&mut rng).unwrap();
    let last_name = names.last_names.choose(&mut rng).unwrap();

    format!("{} \"{}\" {}", first_name, nickname, last_name)
}

#[derive(Component, Default)]
pub struct Person {
    pub(crate) assets: Inventory,
    pub utility: VecDeque<f64>,
}

#[measured]
pub fn consume(
    mut people: Query<(Entity, &Name, &mut Person)>,
    items: Res<Items>,
    mut commands: Commands,
) {
    let mut rng = rand::thread_rng();
    for (_, name, mut person) in people.iter_mut() {
        let mut amount_to_remove: HashMap<ItemType, usize> = HashMap::new();
        for (item_type, all_items) in person.assets.items.iter_mut() {
            let consumption_rate = items
                .items
                .get(&item_type.name)
                .unwrap_or_else(|| {
                    panic!(
                        "Item {} does not have consumption rate! Fix this in items.json",
                        &item_type.name
                    )
                })
                .consumption_rate;
            for _ in all_items.iter_mut() {
                if rng.gen_range(0.0..=1.0) < consumption_rate {
                    debug!("{} consumed {}", name, item_type.name);
                    amount_to_remove
                        .entry(item_type.clone())
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
            }
        }

        for (item_type, amount) in amount_to_remove.iter() {
            person
                .assets
                .items
                .get_mut(item_type)
                .unwrap()
                .drain(0..*amount)
                .for_each(|e| commands.entity(e).despawn());
        }
    }
}

#[measured]
pub fn create_buy_orders_for_people(
    mut people: Query<(Entity, &Name, &Wallet, &mut Person)>,
    needs: Res<Needs>,
    price_history: Res<PriceHistory>,
    mut logs: EventWriter<LogEvent>,
    mut commands: Commands,
) {
    let mut rng = rand::thread_rng();
    for (buyer, name, _, mut person) in people.iter_mut() {
        let mut total_assets = calculate_total_items(&person.assets);
        let mut item_buy_success_count = 0;

        person
            .utility
            .push_front(utility(&needs, name, &total_assets, &price_history));
        while item_buy_success_count < 5 {
            match try_to_buy_item(
                &needs,
                &price_history,
                &mut logs,
                &mut commands,
                &mut rng,
                buyer,
                name,
                &total_assets,
            ) {
                Some(item) => {
                    *total_assets.entry(item).or_insert(0) += 1;
                    item_buy_success_count += 1;
                }
                None => break,
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn try_to_buy_item(
    needs: &Res<Needs>,
    price_history: &Res<PriceHistory>,
    logs: &mut EventWriter<LogEvent>,
    commands: &mut Commands,
    mut rng: &mut ThreadRng,
    buyer: Entity,
    name: &Name,
    total_assets: &HashMap<ItemType, u64>,
) -> Option<ItemType> {
    let mut person_marginal_utilities: HashMap<ItemType, f64> = HashMap::new();
    for need in needs.needs.iter().flat_map(|(_, n)| n.satisfied_by.keys()) {
        let util = marginal_utility(needs, name, total_assets, price_history, need);
        person_marginal_utilities.insert(need.clone(), util);
    }
    if let Some(money_utility) = calculate_money_utility(&person_marginal_utilities, price_history)
    {
        if let Some(value) = create_buy_order_with_money_utility(
            price_history,
            logs,
            commands,
            &mut rng,
            buyer,
            name,
            &mut person_marginal_utilities,
            money_utility,
        ) {
            Some(value)
        } else {
            create_buy_order_without_money_utlity(
                logs,
                commands,
                buyer,
                name,
                &mut person_marginal_utilities,
            )
        }
    } else {
        create_buy_order_without_money_utlity(
            logs,
            commands,
            buyer,
            name,
            &mut person_marginal_utilities,
        )
    }
}

fn create_buy_order_without_money_utlity(
    logs: &mut EventWriter<LogEvent>,
    commands: &mut Commands,
    buyer: Entity,
    name: &Name,
    person_marginal_utilities: &mut HashMap<ItemType, f64>,
) -> Option<ItemType> {
    let biggest_marginal_utility_item_type = person_marginal_utilities
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(item_type, _)| item_type)
        .unwrap();
    Some(create_buy_order(
        logs,
        commands,
        buyer,
        name,
        biggest_marginal_utility_item_type,
        1,
    ))
}

#[allow(clippy::too_many_arguments)]
fn create_buy_order_with_money_utility(
    price_history: &Res<PriceHistory>,
    logs: &mut EventWriter<LogEvent>,
    commands: &mut Commands,
    mut rng: &mut &mut ThreadRng,
    buyer: Entity,
    name: &Name,
    person_marginal_utilities: &mut HashMap<ItemType, f64>,
    money_utility: f64,
) -> Option<ItemType> {
    debug!("Money utility for {} is {}", name, money_utility);
    let utilities_with_prices = calculate_marginal_utilities_adjusted_by_prices(
        person_marginal_utilities,
        price_history,
        money_utility,
    );
    // info!("Utilities without prices for {} are:\n {:#?}", name, person_marginal_utilities);
    debug!(
        "Utilities with prices for {} are:\n {:#?}",
        name, utilities_with_prices
    );
    // Sort by utility
    let mut utilities: Vec<(&ItemType, &f64)> = utilities_with_prices.iter().collect();
    utilities.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

    // Convert utilities to weights
    let weights: Vec<f64> = utilities.iter().map(|(_, util)| **util).collect();

    // Create a WeightedIndex distribution
    let dist = WeightedIndex::new(weights);
    if dist.is_err() {
        logs.send(LogEvent::Generic {
            text: "There is no item I can buy!".to_string(),
            entity: buyer,
        });
        return None;
    }

    // Sample from it
    let index = dist.unwrap().sample(&mut rng);

    // Get the corresponding item
    let (item_type, _util) = utilities[index];

    trace!("Chosen item for person {} is {}", name, item_type.name);
    Some(create_buy_order(logs, commands, buyer, name, item_type, 1))
}

fn create_buy_order(
    logs: &mut EventWriter<LogEvent>,
    commands: &mut Commands,
    buyer: Entity,
    name: &Name,
    item_type: &ItemType,
    expiration: u64,
) -> ItemType {
    let buy_order = BuyOrder {
        item_type: item_type.clone(),
        buyer,
        order: OrderType::Market, // Always buying at market price
        expiration: Some(expiration),
    };
    logs.send(LogEvent::Generic {
        text: format!(
            "{}: I'll try to buy {} at market price",
            name, item_type.name
        ),
        entity: buyer,
    });
    commands.spawn((
        buy_order.clone(),
        Name::new(format!("Consumer {} buy order @Market", item_type.name)),
    ));
    item_type.clone()
}

fn calculate_marginal_utilities_adjusted_by_prices(
    item_utilities: &HashMap<ItemType, f64>,
    price_history: &Res<PriceHistory>,
    money_utility: f64,
) -> HashMap<ItemType, f64> {
    let mut result = HashMap::new();
    for (item_type, item_utility) in item_utilities.iter() {
        if let Some(price_stats) = price_history.prices.get(item_type) {
            if let Some(last_price) = price_stats.last() {
                let updated_utility = item_utility - last_price.median.as_f64() * money_utility;
                if updated_utility > 0.0 {
                    result.insert(item_type.clone(), updated_utility);
                }
            }
        }
    }
    result
}

fn calculate_money_utility(
    item_utilities: &HashMap<ItemType, f64>,
    price_history: &Res<PriceHistory>,
) -> Option<f64> {
    let mut total_utility = 0.0;
    let mut price_count = 0;

    for (item_type, item_utility) in item_utilities.iter() {
        if let Some(price_stats) = price_history.prices.get(item_type) {
            if let Some(last_price) = price_stats.last() {
                total_utility += *item_utility / last_price.median.as_f64();
                price_count += 1;
            }
        }
    }

    if price_count > 0 {
        Some(total_utility / price_count as f64)
    } else {
        None
    }
}

fn calculate_total_items(assets: &Inventory) -> HashMap<ItemType, u64> {
    let mut result = HashMap::new();
    for (item_type, items) in assets.items.iter() {
        result.insert(item_type.clone(), items.len() as u64);
    }
    result
}

fn marginal_utility(
    needs: &Needs,
    name: &Name,
    total_items: &HashMap<ItemType, u64>,
    price_history: &PriceHistory,
    item_type: &ItemType,
) -> f64 {
    // Create a mutable copy of the total_items HashMap
    let mut total_items_copy = total_items.clone();

    // Increase the quantity of the given ItemType by one.
    // If the ItemType is not already in the HashMap, this inserts it with a quantity of one.
    let original_utility = utility(needs, name, total_items, price_history);
    *total_items_copy.entry(item_type.clone()).or_insert(0) += 1;
    let new_utility = utility(needs, name, &total_items_copy, price_history);
    new_utility - original_utility
}

fn utility(
    needs: &Needs,
    _name: &Name,
    total_items: &HashMap<ItemType, u64>,
    _price_history: &PriceHistory,
) -> f64 {
    let mut result = 1.0;
    // calculate utility for each need
    for (_, need) in needs.needs.iter() {
        for (item_type, amount) in need.satisfied_by.iter() {
            let items_count = *total_items.get(item_type).unwrap_or(&0);
            let item_utility =
                ((items_count as f64 * amount + 1.0) / need.base).powf(need.preference);
            // info!("Utility for person {} for {} is {}", name, item, item_utility);
            result *= item_utility;
        }
    }
    // info!("Total utility for person {} is {}", name, result);
    result
}
