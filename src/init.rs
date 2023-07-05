use crate::business::{
    BuyStrategy, Inventory, ItemType, Manufacturer, ManufacturerBundle, ProductionCycle,
    SellStrategy, Worker,
};
use crate::money::money_from_str_or_num;
use crate::money::Money;
use bevy::core::Name;
use bevy::log::info;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum TemplateType {
    #[default]
    Manufacturers,
    ProductionCycles,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default)]
pub struct Templates {
    pub manufacturers: Vec<ManufacturerTemplate>,
    pub production_cycles: Vec<ProductionCycleTemplate>,
    pub(crate) production_cycles_json: String,
    pub(crate) manufacturers_json: String,
    pub(crate) selected_template: TemplateType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManufacturerTemplate {
    name: String,
    #[serde(deserialize_with = "money_from_str_or_num")]
    money: Money,
    workers: Vec<Worker>,
    production_cycle: String,
    sell_strategy: SellStrategy,
    copies: u32,
}

impl ManufacturerTemplate {
    pub fn to_manufacturer(
        &self,
        production_cycles: HashMap<String, ProductionCycle>,
        commands: &mut Commands,
    ) -> Vec<ManufacturerBundle> {
        let mut manufacturers = Vec::new();
        for _ in 0..self.copies {
            let workers = self
                .workers
                .iter()
                .map(|w| commands.spawn(*w).id())
                .collect::<Vec<_>>();
            let manufacturer = ManufacturerBundle {
                name: Name::new(self.name.clone()),
                manufacturer: Manufacturer {
                    production_cycle: production_cycles.get(&self.production_cycle)
                        .cloned()
                        .unwrap_or_else(|| panic!("{} not found, make sure production cycle with this name is defined in production_cycles.json", self.production_cycle)),
                    assets: Inventory {
                        money: self.money,
                        items: HashMap::new(),
                        items_to_sell: Default::default(),
                    },
                    hired_workers: workers,
                },
                sell_strategy: self.sell_strategy,
            };
            manufacturers.push(manufacturer);
        }
        info!(
            "Created {} manufacturers of type {}",
            manufacturers.len(),
            self.name
        );
        manufacturers
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

fn load_production_cycles() -> Result<(String, Vec<ProductionCycleTemplate>), Box<dyn Error>> {
    let mut file = File::open("data/production_cycles.json")?;
    let mut json_string = String::new();
    file.read_to_string(&mut json_string)?;
    let templates: Vec<ProductionCycleTemplate> = serde_json::from_str(&json_string)?;
    Ok((json_string, templates))
}

fn load_manufacturers() -> Result<(String, Vec<ManufacturerTemplate>), Box<dyn Error>> {
    let mut file = File::open("data/manufacturers.json")?;
    let mut json_string = String::new();
    file.read_to_string(&mut json_string)?;
    let templates: Vec<ManufacturerTemplate> = serde_json::from_str(&json_string)?;
    Ok((json_string, templates))
}

pub fn init_manufacturers(mut commands: Commands, mut templates: ResMut<Templates>) {
    let (production_json, production_cycles) =
        load_production_cycles().expect("Unable to load production cycles");
    let (manufacturers_json, manufacturer_templates) =
        load_manufacturers().expect("Unable to load manufacturers");
    templates.manufacturers = manufacturer_templates.clone();
    templates.production_cycles = production_cycles.clone();
    templates.production_cycles_json = production_json;
    templates.manufacturers_json = manufacturers_json;
    let production_cycles = production_cycles
        .into_iter()
        .map(|p| p.to_production_cycle())
        .collect::<HashMap<_, _>>();
    info!("Loaded {} production cycles", production_cycles.len());
    info!(
        "Loaded {} manufacturer templates",
        manufacturer_templates.len()
    );
    for template in manufacturer_templates {
        let manufacturers = template.to_manufacturer(production_cycles.clone(), &mut commands);
        for manufacturer in manufacturers {
            if manufacturer.manufacturer.production_cycle.input.is_empty() {
                commands.spawn(manufacturer);
            } else {
                // TODO check if this works even if input is empty and if so create default buy strategy
                info!(
                    "Creating manufacturer {} with buy strategy",
                    manufacturer.name.to_string()
                );
                commands.spawn((
                    manufacturer,
                    BuyStrategy {
                        target_production_cycles: 2,
                        outstanding_orders: 0,
                    },
                ));
            }
        }
    }
}
