use bevy::prelude::*;
use rand::seq::SliceRandom;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Ord, PartialOrd)]
pub struct ItemType {
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
struct ProductionCycle {
    input: HashMap<ItemType, u32>,
    output: (ItemType, u32),
    // tools: HashMap<ItemType, u32>,
    workdays_needed: u32,
}

pub struct Inventory {
    items: HashMap<ItemType, Vec<Entity>>,
    items_to_sell: HashSet<Entity>,
    money: u64,
}

#[derive(Bundle)]
pub struct ManufacturerBundle {
    pub name: Name,
    pub manufacturer: Manufacturer,
    pub sell_strategy: SellStrategy,
}

#[derive(Component)]
pub struct Manufacturer {
    production_cycle: ProductionCycle,
    assets: Inventory,
    hired_workers: Vec<Entity>,
}

#[derive(Component)]
pub struct Worker {
    salary: u64,
    // employer: Entity,
}

#[derive(Component, Debug)]
pub struct Item {
    item_type: ItemType,
    production_cost: u64,
    buy_cost: u64,
    // owner: Entity,
}

#[derive(Component, Debug)]
pub struct SellOrder {
    item: Entity,
    pub(crate) item_type: ItemType,
    owner: Entity,
    pub(crate) price: u64,
    pub(crate) base_price: u64,
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
#[derive(Component, Copy, Clone)]
pub struct SellStrategy {
    // max_margin: f32,
    margin_drop_per_day: f32,
    current_margin: f32,
    min_margin: f32,
}

#[derive(Debug, Clone)]
enum OrderType {
    Market,
    // Limit(u64),
}

#[derive(Component, Debug, Clone)]
pub struct BuyOrder {
    item_type: ItemType,
    owner: Entity,
    order: OrderType,
}

#[derive(Component, Copy, Clone)]
pub struct BuyStrategy {
    target_production_cycles: u32,
    outstanding_orders: u32,
}

#[derive(Debug)]
pub enum MaxCycleError {
    NoMaterialInInventory(String),
    // the String will contain the material name
    NotEnoughMaterialsOrWorkers,
    CantPayWorkers,
}

pub fn init(mut commands: Commands) {
    let board_maker = commands
        .spawn((Worker { salary: 100 }, Name::new("Board maker")))
        .id();
    let lumberjack = commands
        .spawn((Worker { salary: 60 }, Name::new("Lumberjack")))
        .id();
    // spawn lumberjack
    commands.spawn(ManufacturerBundle {
        name: Name::new("Lumberjack Hut"),
        manufacturer: Manufacturer {
            production_cycle: ProductionCycle {
                input: HashMap::new(),
                output: (
                    ItemType {
                        name: "wood".to_string(),
                    },
                    4,
                ),
                // tools: HashMap::new(),
                workdays_needed: 1,
            },
            assets: Inventory {
                items: HashMap::new(),
                items_to_sell: HashSet::new(),
                money: 2000,
            },
            hired_workers: vec![lumberjack],
        },
        sell_strategy: SellStrategy {
            min_margin: 0.5,
            margin_drop_per_day: 0.1,
            current_margin: 2.0,
        },
    });
    commands.spawn(ManufacturerBundle {
        name: Name::new("Lumberjack Hut"),
        manufacturer: Manufacturer {
            production_cycle: ProductionCycle {
                input: HashMap::new(),
                output: (
                    ItemType {
                        name: "wood".to_string(),
                    },
                    3,
                ),
                // tools: HashMap::new(),
                workdays_needed: 1,
            },
            assets: Inventory {
                items: HashMap::new(),
                items_to_sell: HashSet::new(),
                money: 2000,
            },
            hired_workers: vec![lumberjack],
        },
        sell_strategy: SellStrategy {
            min_margin: 0.5,
            margin_drop_per_day: 0.1,
            current_margin: 2.0,
        },
    });

    // spawn wooden board manufacturer
    let mut items = HashMap::new();
    let mut items_in_inventory = vec![];
    for _ in 0..2 {
        let item = commands
            .spawn((
                Item {
                    item_type: ItemType {
                        name: "wood".to_string(),
                    },
                    production_cost: 0,
                    buy_cost: 20,
                    // owner: Entity::new(0),
                },
                Name::new("Wood"),
            ))
            .id();
        items_in_inventory.push(item);
    }
    items.insert(
        ItemType {
            name: "wood".to_string(),
        },
        items_in_inventory,
    );
    let mut input = HashMap::new();
    input.insert(
        ItemType {
            name: "wood".to_string(),
        },
        1,
    );
    commands.spawn((
        ManufacturerBundle {
            name: Name::new("Wooden board manufacturer"),
            manufacturer: Manufacturer {
                production_cycle: ProductionCycle {
                    input,
                    output: (
                        ItemType {
                            name: "boards".to_string(),
                        },
                        10,
                    ),
                    // tools: HashMap::new(),
                    workdays_needed: 1,
                },
                assets: Inventory {
                    items,
                    items_to_sell: HashSet::new(),
                    money: 5000,
                },
                hired_workers: vec![board_maker],
            },
            sell_strategy: SellStrategy {
                min_margin: 0.5,
                margin_drop_per_day: 0.1,
                current_margin: 2.0,
            },
        },
        BuyStrategy {
            target_production_cycles: 3,
            outstanding_orders: 0,
        },
    ));
}

pub fn produce(
    mut manufacturers: Query<&mut Manufacturer>,
    mut commands: Commands,
    workers_query: Query<&Worker>,
    items_query: Query<&Item>,
) {
    for mut manufacturer in manufacturers.iter_mut() {
        // fill production cycle
        // produce_for_manufacturer(&mut b, commands, &production_cost);
        execute_production_cycle(
            &mut commands,
            &mut manufacturer,
            &workers_query,
            &items_query,
        )
    }
}

fn execute_production_cycle(
    commands: &mut Commands,
    manufacturer: &mut Manufacturer,
    workers_query: &Query<&Worker>,
    items_query: &Query<&Item>,
) {
    match calculate_max_cycles(manufacturer, workers_query) {
        Ok((max_cycles, mut cost_per_cycle)) => {
            for (input_material, &quantity_needed) in manufacturer.production_cycle.input.iter() {
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

            if (manufacturer.assets.money as i64 - cost_per_cycle as i64) < 0 {
                debug!("Not enough money to run a cycle, nothing will be produced");
                return;
            }
            manufacturer.assets.money -= cost_per_cycle;

            // Calculate output materials and their unit costs
            let (output_material, quantity_produced) = &manufacturer.production_cycle.output;
            let unit_cost = cost_per_cycle / (*quantity_produced * max_cycles) as u64;
            for _ in 0..*quantity_produced * max_cycles {
                let item = Item {
                    item_type: output_material.clone(),
                    production_cost: unit_cost,
                    buy_cost: 0,
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
    manufacturer: &Manufacturer,
    workers_query: &Query<&Worker>,
) -> Result<(u32, u64), MaxCycleError> {
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
    let mut cost_per_cycle = 0;
    for worker in manufacturer.hired_workers.iter() {
        cost_per_cycle += workers_query.get(*worker).unwrap().salary;
    }
    debug!("Salaries cost per cycle: {}", cost_per_cycle);

    if (manufacturer.assets.money as i64 - cost_per_cycle as i64) < 0 {
        debug!("Dear Lord, we can't even pay our workers, we're doomed!");
        return Err(MaxCycleError::CantPayWorkers);
    }

    Ok((max_cycles, cost_per_cycle))
}

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
                    owner: seller,
                    price: (item_cost.production_cost as f32
                        * strategy.current_margin.max(strategy.min_margin))
                        as u64,
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

pub fn update_sell_order_prices(
    mut sell_orders: Query<(Entity, &Name, &mut SellOrder, &mut SellStrategy)>,
) {
    for (_, name, mut sell_order, mut sell_strategy) in sell_orders.iter_mut() {
        sell_strategy.current_margin -= sell_strategy.margin_drop_per_day;
        sell_order.price = (sell_order.base_price as f32
            * sell_strategy.current_margin.max(sell_strategy.min_margin))
            as u64;
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

pub fn create_buy_orders(
    mut commands: Commands,
    mut manufacturers: Query<(Entity, &Manufacturer, &mut BuyStrategy)>,
) {
    for (buyer, manufacturer, mut strategy) in manufacturers.iter_mut() {
        let needed_materials = &manufacturer.production_cycle.input;
        let inventory = &manufacturer.assets.items;

        for (material, &quantity_needed) in needed_materials.iter() {
            let inventory_quantity = inventory
                .get(material)
                .map_or(0, |items| items.len() as u32);

            let cycles_possible_with_current_inventory = inventory_quantity / quantity_needed;
            if cycles_possible_with_current_inventory < strategy.target_production_cycles {
                let quantity_to_buy = (strategy.target_production_cycles
                    - cycles_possible_with_current_inventory)
                    * quantity_needed
                    - strategy.outstanding_orders;
                strategy.outstanding_orders += quantity_to_buy;
                let buy_order = BuyOrder {
                    item_type: material.clone(), // assuming ItemType implements Copy
                    owner: buyer,
                    order: OrderType::Market, // Always buying at market price
                };

                if quantity_to_buy == 0 {
                    debug!(
                        "No need to buy any more {}, I already have {} and {} in orders",
                        material.name, inventory_quantity, strategy.outstanding_orders
                    );
                    continue;
                }
                debug!("Created buy order {:?} for {}", buy_order, quantity_to_buy);

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

pub fn execute_orders_for_manufacturers(
    mut commands: Commands,
    mut buy_orders: Query<(Entity, &BuyOrder)>,
    mut sell_orders: Query<(Entity, &SellOrder)>,
    mut manufacturers: Query<(Entity, &mut Manufacturer)>,
    mut buy_strategy: Query<(Entity, &mut BuyStrategy)>,
    mut items: Query<(Entity, &mut Item)>,
) {
    let mut rng = rand::thread_rng();

    // Iterate over each buy order
    for (buy_order_id, buy_order) in buy_orders.iter_mut() {
        let matching_sell_orders: Vec<_> = sell_orders
            .iter_mut()
            .filter(|(_, sell_order)| sell_order.item_type == buy_order.item_type) // Match by material
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
                        execute_order(
                            &mut buy_strategy,
                            &mut manufacturers,
                            &mut commands,
                            (sell_order_id, sell_order),
                            (buy_order_id, buy_order),
                            &mut items,
                        );
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
        }
    }
}
fn execute_order(
    buy_strategy: &mut Query<(Entity, &mut BuyStrategy)>,
    manufacturers: &mut Query<(Entity, &mut Manufacturer)>,
    commands: &mut Commands,
    sell_order: (Entity, &SellOrder),
    buy_order: (Entity, &BuyOrder),
    items: &mut Query<(Entity, &mut Item)>,
) {
    let (sell_order_id, sell_order) = sell_order;
    let (buy_order_id, buy_order) = buy_order;
    let sell_item_type = &sell_order.item_type;
    let buy_item_type = &buy_order.item_type;

    // Assume that the item type in the sell order is same as the buy order
    assert_eq!(buy_item_type, sell_item_type);

    // Phase 1: Do all the checks and computations
    let mut buyer_has_enough_money = false;
    // let mut seller_has_item = false;
    if let Ok((_, buyer_manufacturer)) = manufacturers.get_mut(buy_order.owner) {
        if buyer_manufacturer.assets.money >= sell_order.price {
            buyer_has_enough_money = true;
        }
    }
    // if let Ok((_, mut seller_manufacturer)) = manufacturers.get_mut(sell_order.owner) {
    //     if let Some(_) = seller_manufacturer.assets.items_to_sell.take(&sell_order.item) {
    //         seller_has_item = true;
    //     }
    // }

    // If any check failed, abort the operation
    if !buyer_has_enough_money {
        debug!("Cannot execute the order: buyer does not have enough money");
        return;
    }

    // if !seller_has_item {
    //     debug!("Cannot execute the order: seller does not have the item");
    //     return;
    // }

    // Phase 2: Execute the operations
    if let Ok((_, mut buyer_manufacturer)) = manufacturers.get_mut(buy_order.owner) {
        // Transfer money from buyer to seller
        buyer_manufacturer.assets.money -= sell_order.price;
        if let Ok((_, mut strategy)) = buy_strategy.get_mut(buy_order.owner) {
            strategy.outstanding_orders -= 1;
        }

        // Transfer item from seller to buyer
        if let Ok((_, mut item)) = items.get_mut(sell_order.item) {
            item.buy_cost = sell_order.price;
        }
        buyer_manufacturer
            .assets
            .items
            .entry(buy_item_type.clone())
            .or_default()
            .push(sell_order.item);
        commands.entity(buy_order_id).despawn();
    }
    if let Ok((_, mut seller_manufacturer)) = manufacturers.get_mut(sell_order.owner) {
        // Add money to seller
        seller_manufacturer.assets.money += sell_order.price;
        commands.entity(sell_order_id).despawn();
    }
    debug!("!!!! Executed order: {:?} -> {:?}", sell_order, buy_order);
}

// pub fn create_buy_orders_for_manufacturers(
//     mut commands: Commands,
//     mut manufacturers: Query<(Entity, &mut Manufacturer, &BuyStrategy)>,
//     items_query: Query<&Item>,
// ) {
//     for (buyer, mut manufacturer, strategy) in manufacturers.iter_mut() {
//         let mut items_to_buy = vec![];
//         for item in manufacturer.assets.items_to_buy.iter() {
//             items_to_buy.push(*item);
//         }
//         for item in items_to_buy {
//             if let Ok(item_cost) = items_query.get(item) {
//                 let buy_order = BuyOrder {
//                     item,
//                     owner: buyer,
//                     price: (item_cost.production_cost as f32 * strategy.current_margin) as u64,
//                     base_price: item_cost.production_cost,
//                 };
//                 debug!("Created buy order {:?}", buy_order);
//                 let strategy_copy = strategy.clone();
//                 commands.spawn((buy_order, Name::new("Wood buy order"), strategy_copy));
//                 manufacturer.assets.items_to_buy.remove(&item);
//             }
//         }
//     }
// }

// fn produce_for_manufacturer(b: &mut Manufacturer, commands: &mut Commands, production_cost: &Query<&ProductionCost>) {
//     let mut free_workers = b.hired_workers.len() as u32;
//     let mut variable_cost = 0;
//     let mut fixed_cost = 0;
//     let mut cycles = 0;
//     while b.assets.contains_materials(&b.production_cycle) && free_workers > 0 {
//         // remove input materials
//         for (item, amount) in b.production_cycle.input.clone() {
//             if let Some(a) = b.assets.items.get_mut(&item) {
//                 if a.len() >= amount as usize {
//                     for _ in 0..amount {
//                         if let Some(used_item) = a.pop() {
//                             production_cost.get(used_item).map(|cost| {
//                                 variable_cost += cost.variable_cost;
//                                 fixed_cost += cost.fixed_cost;
//                             });
//                             commands.entity(used_item).despawn_recursive();
//                         }
//                     }
//                 }
//             }
//         }
//         free_workers -= b.production_cycle.work_days_needed;
//         cycles += 1;
//     }
//     // add output materials
//     for (item, amount) in b.production_cycle.output.clone() {
//         // *b.assets.items.entry(item).or_insert(0) += amount * cycles;
//         let unit_cost =
//         for _ in 0..amount * cycles {
//             let new_item = commands.spawn().insert(item.clone()).id();
//             b.assets.items.entry(item.clone()).or_insert(Vec::new()).push(new_item);
//         }
//     }
// }
// // mark what tools or machines were used
// // update assets
// // calculate production cost
// #[test]
// fn test_produce_for_business() {
//     // all used up
//     run_produce_for_business_test(1, 12, 1, 2, 2, 0, 24);
//     // not enough workers
//     run_produce_for_business_test(1, 12, 1, 2, 1, 1, 12);
//     // not enough input
//     run_produce_for_business_test(1, 12, 1, 0, 2, 0, 0);
// }
//
// fn run_produce_for_business_test(
//     input_item_amount_per_cycle: u32,
//     output_item_amount_per_cycle: u32,
//     workers_needed_per_cycle: u32,
//     input_available: u32,
//     workers: u32,
//     expected_remaining_input: u32,
//     expected_output: u32,
// ) {
//     // Define test items
//     let input_item = ItemType {
//         name: "Wood".to_string(),
//     };
//     let output_item = ItemType {
//         name: "Furniture".to_string(),
//     };
//
//     // Create a ProductionCycle and Assets for the Business
//     let production_cycle = ProductionCycle {
//         input: [(input_item.clone(), input_item_amount_per_cycle)]
//             .iter()
//             .cloned()
//             .collect(),
//         output: [(output_item.clone(), output_item_amount_per_cycle)]
//             .iter()
//             .cloned()
//             .collect(),
//         tools: HashMap::new(),
//         work_days_needed: workers_needed_per_cycle,
//     };
//
//     let assets = Inventory {
//         resources: [(input_item.clone(), input_available)]
//             .iter()
//             .cloned()
//             .collect(),
//         tools: HashMap::new(),
//         products: HashMap::new(),
//         money: 0,
//     };
//
//     let hired_workers: Vec<Entity> = (1..=workers).map(|i| Entity::from_raw(i)).collect();
//     // Create a Business
//     let mut business = Manufacturer {
//         production_cycle,
//         assets,
//         hired_workers: hired_workers.clone(),
//     };
//
//     // Run the produce_for_business function
//     produce_for_manufacturer(&mut business);
//
//     // Check the results
//     let resources = &business.assets.resources;
//     let products = &business.assets.products;
//
//     assert_eq!(
//         resources.get(&input_item),
//         Some(&expected_remaining_input),
//         "Resource item should have been consumed"
//     );
//     assert_eq!(
//         products.get(&output_item),
//         Some(&(expected_output)),
//         "Output product should have been produced"
//     );
// }
