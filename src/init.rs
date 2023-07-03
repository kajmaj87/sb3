use crate::business::{
    BuyStrategy, Inventory, ItemType, Manufacturer, ManufacturerBundle, ProductionCycle,
    SellStrategy, Worker,
};
use crate::money::Money;
use bevy::core::Name;
use bevy::prelude::Commands;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::Read;

// #[derive(Deserialize, Debug)]
// pub struct ManufacturerTemplate {
//     name: String,
//     #[serde(deserialize_with = "money_from_str_or_num")]
//     money: Money,
//     workers: Vec<Worker>,
//     production_cycle: String,
//     sell_strategy: SellStrategy,
//     copies: u32,
// }
//
// impl ManufacturerTemplate {
//     pub fn to_manufacturer(&self) -> Vec<Manufacturer> {
//         let mut manufacturers = Vec::new();
//         for _ in 0..self.copies {
//             let manufacturer = Manufacturer {
//                 name: self.name.clone(),
//                 money: self.money.clone(),
//                 production_cycle: self.production_cycle.clone(),
//                 assets: (),
//                 sell_strategy: self.sell_strategy.clone(),
//                 hired_workers: vec![],
//             };
//             manufacturers.push(manufacturer);
//         }
//         manufacturers
//     }
// }

#[derive(Deserialize, Debug)]
pub struct ProductionCycleTemplate {
    name: String,
    input: HashMap<String, u32>,
    output: (String, u32),
    workdays_needed: u32,
}

impl ProductionCycleTemplate {
    pub fn to_production_cycle(&self) -> (String, ProductionCycle) {
        let input = self
            .input
            .iter()
            .map(|(name, &count)| (ItemType { name: name.clone() }, count))
            .collect();

        let output = (
            ItemType {
                name: self.output.0.clone(),
            },
            self.output.1,
        );

        let production_cycle = ProductionCycle {
            input,
            output,
            workdays_needed: self.workdays_needed,
        };

        (self.name.clone(), production_cycle)
    }
}

fn load_production_cycles() -> Result<HashMap<String, ProductionCycle>, Box<dyn Error>> {
    let mut file = File::open("data/production_cycles.json")?;
    let mut json_string = String::new();
    file.read_to_string(&mut json_string)?;
    let templates: Vec<ProductionCycleTemplate> = serde_json::from_str(&json_string)?;
    Ok(templates
        .into_iter()
        .map(|template| template.to_production_cycle())
        .collect())
}

pub fn init_manufacturers(mut commands: Commands) {
    let production_cycles = load_production_cycles().expect("Unable to load production cycles");
    let board_maker = commands
        .spawn((
            Worker {
                salary: Money(1000),
            },
            Name::new("Board maker"),
        ))
        .id();
    let lumberjack = commands
        .spawn((Worker { salary: Money(600) }, Name::new("Lumberjack")))
        .id();
    let furniture_maker = commands
        .spawn((
            Worker {
                salary: Money(1500),
            },
            Name::new("Furniture maker"),
        ))
        .id();
    let furniture_maker_2 = commands
        .spawn((
            Worker {
                salary: Money(1200),
            },
            Name::new("Furniture maker"),
        ))
        .id();
    // spawn lumberjack
    commands.spawn(ManufacturerBundle {
        name: Name::new("Lumberjack Hut"),
        manufacturer: Manufacturer {
            production_cycle: production_cycles["Wood slow"].clone(),
            assets: Inventory {
                items: HashMap::new(),
                items_to_sell: HashSet::new(),
                money: Money(20000),
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
            production_cycle: production_cycles["Wood fast"].clone(),
            assets: Inventory {
                items: HashMap::new(),
                items_to_sell: HashSet::new(),
                money: Money(20000),
            },
            hired_workers: vec![lumberjack],
        },
        sell_strategy: SellStrategy {
            min_margin: 0.5,
            margin_drop_per_day: 0.1,
            current_margin: 2.0,
        },
    });

    for _ in 0..10 {
        // spawn wooden board manufacturer
        commands.spawn((
            ManufacturerBundle {
                name: Name::new("Wooden board manufacturer"),
                manufacturer: Manufacturer {
                    production_cycle: production_cycles["Boards"].clone(),
                    assets: Inventory {
                        items: HashMap::new(),
                        items_to_sell: HashSet::new(),
                        money: Money(50000),
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
    commands.spawn((
        ManufacturerBundle {
            name: Name::new("Furniture manufacturer"),
            manufacturer: Manufacturer {
                production_cycle: production_cycles["Furniture"].clone(),
                assets: Inventory {
                    items: HashMap::new(),
                    items_to_sell: HashSet::new(),
                    money: Money(100000),
                },
                hired_workers: vec![furniture_maker, furniture_maker_2],
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
