use crate::business::{BuyOrder, Inventory, ItemType, OrderType, Wallet};
use crate::debug_ui::Performance;
use crate::stats::PriceHistory;
use bevy::prelude::*;
use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::SliceRandom;
use serde::Deserialize;
use std::collections::HashMap;

use macros::measured;

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
    pub satisfied_by: HashMap<String, f64>,
    pub increased_by: Option<HashMap<String, f64>>,
}

#[derive(Resource, Default)]
pub struct Needs {
    pub needs: HashMap<String, Need>,
}

impl Needs {
    pub fn load(&mut self) {
        let needs = std::fs::read_to_string("data/needs.json").unwrap();
        let needs: HashMap<String, Need> = serde_json::from_str(&needs).unwrap();
        self.needs = needs;
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
    pub utility: f64,
}

#[measured]
pub fn create_buy_orders_for_people(
    mut people: Query<(Entity, &Name, &Wallet, &mut Person)>,
    needs: Res<Needs>,
    price_history: Res<PriceHistory>,
    mut commands: Commands,
) {
    let mut marginal_utilities: HashMap<(String, String), f64> = HashMap::new();
    let mut rng = rand::thread_rng();
    for (buyer, name, wallet, mut person) in people.iter_mut() {
        let total_assets = calculate_total_items(&person.assets);
        let mut person_marginal_utilities: HashMap<String, f64> = HashMap::new();
        for need in needs
            .needs
            .iter()
            .map(|(name, n)| n.satisfied_by.keys())
            .flatten()
        {
            let item_type = ItemType {
                name: need.to_string(),
            };
            let util = marginal_utility(&needs, &name, &total_assets, &price_history, &item_type);
            // marginal_utilities.insert((name.to_string().clone(), item_type.name.clone()), util);
            person_marginal_utilities.insert(item_type.name.clone(), util);
        }
        person.utility = utility(&needs, &name, &total_assets, &price_history);
        // Sort by utility
        let mut utilities: Vec<(&String, &f64)> = person_marginal_utilities.iter().collect();
        utilities.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

        // Convert utilities to weights
        let weights: Vec<f64> = utilities.iter().map(|(_, util)| **util).collect();

        // Create a WeightedIndex distribution
        let dist = WeightedIndex::new(&weights).unwrap();

        // Sample from it
        let index = dist.sample(&mut rng);

        // Get the corresponding item
        let (item_name, _util) = utilities[index];

        info!("Chosen item for person {} is {}", name, item_name);
        let buy_order = BuyOrder {
            item_type: ItemType {
                name: item_name.to_string(),
            },
            buyer,
            order: OrderType::Market, // Always buying at market price
            expiration: Some(10),
        };
        commands.spawn((
            buy_order.clone(),
            Name::new(format!("Consumer {} buy order @Market", item_name)),
        ));
    }

    //     let mut sums: HashMap<String, f64> = HashMap::new();
    //     for ((person_name, _), &util) in &marginal_utilities {
    //         let entry = sums.entry(person_name.clone()).or_insert(0.0);
    //         *entry += util;
    //     }
    //
    // // Normalize utilities and convert HashMap to Vec for sorting
    //     let mut marginal_utilities_vec: Vec<((String, String), f64)> = marginal_utilities.drain()
    //         .map(|((person_name, item_name), util)| {
    //             let sum = sums.get(&person_name).unwrap();
    //             ((person_name, item_name), util / sum)
    //         })
    //         .collect();
    //
    // // Sort by person name first, then by utility value in descending order
    //     marginal_utilities_vec.sort_by(|((person_a, _), util_a), ((person_b, _), util_b)| {
    //         let person_cmp = person_a.cmp(person_b);
    //         if person_cmp == std::cmp::Ordering::Equal {
    //             util_b.partial_cmp(util_a).unwrap_or(std::cmp::Ordering::Equal)
    //         } else {
    //             person_cmp
    //         }
    //     });
    //
    // // Iterate and print
    // //     for ((person_name, item_name), util) in marginal_utilities_vec {
    //     // info!("Marginal utility for person {} for {} is {:.3}", person_name, item_name, util);
    //     // }
    //
    //     let mut rng = rand::thread_rng();
    //
    // // Group utilities by person
    //     let mut utilities_by_person: HashMap<String, Vec<((String, String), f64)>> = HashMap::new();
    //     for ((person_name, item_name), util) in marginal_utilities {
    //         utilities_by_person.entry(person_name.clone()).or_insert_with(Vec::new).push(((person_name.clone(), item_name.clone()), util));
    //     }
    //
    // // Select an item for each person
    //     for (person_name, mut utilities) in utilities_by_person {
    //         // Sort by utility
    //         utilities.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    //
    //         // Convert utilities to weights
    //         let weights: Vec<f64> = utilities.iter().map(|(_, util)| *util).collect();
    //
    //         // Create a WeightedIndex distribution
    //         let dist = WeightedIndex::new(&weights).unwrap();
    //
    //         // Sample from it
    //         let index = dist.sample(&mut rng);
    //
    //         // Get the corresponding item
    //         let ((person_name, item_name), _util) = &utilities[index];
    //
    //         info!("Chosen item for person {} is {}", person_name, item_name);
    //     }
}

fn calculate_total_items(assets: &Inventory) -> HashMap<ItemType, u64> {
    let mut result = HashMap::new();
    for (item_type, items) in assets.items.iter() {
        result.insert(item_type.clone(), items.len() as u64);
    }
    result
}

fn marginal_utility(
    needs: &Res<Needs>,
    name: &Name,
    total_items: &HashMap<ItemType, u64>,
    price_history: &Res<PriceHistory>,
    item_type: &ItemType,
) -> f64 {
    // Create a mutable copy of the total_items HashMap
    let mut total_items_copy = total_items.clone();

    // Increase the quantity of the given ItemType by one.
    // If the ItemType is not already in the HashMap, this inserts it with a quantity of one.
    let original_utility = utility(needs, name, total_items, price_history);
    *total_items_copy.entry(item_type.clone()).or_insert(0) += 1;
    let new_utility = utility(needs, name, &total_items_copy, price_history);
    let result = new_utility - original_utility;
    // info!("Marginal utility for person {} for {} is {}", name, item_type.name, result);
    result
}

fn utility(
    needs: &Res<Needs>,
    name: &Name,
    total_items: &HashMap<ItemType, u64>,
    price_history: &Res<PriceHistory>,
) -> f64 {
    let mut result = 1.0;
    // calculate utility for each need
    for (need_name, need) in needs.needs.iter() {
        for (item, amount) in need.satisfied_by.iter() {
            let items_count = *total_items
                .get(&ItemType {
                    name: item.to_string(),
                })
                .unwrap_or(&0);
            let item_utility =
                ((items_count as f64 * amount + 1.0) / need.base).powf(need.preference);
            // info!("Utility for person {} for {} is {}", name, item, item_utility);
            result *= item_utility;
        }
    }
    // info!("Total utility for person {} is {}", name, result);
    result
}
