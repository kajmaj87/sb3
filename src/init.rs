use crate::business::{
    BuyStrategy, Inventory, ItemType, Manufacturer, ManufacturerBundle, ProductionCycle,
    SellStrategy, TransactionLog, Wallet, Worker,
};
use crate::money::money_from_str_or_num;
use crate::money::Money;
use crate::people;
use crate::people::Person;
use crate::people::{Names, Needs};
use bevy::core::Name;
use bevy::log::info;
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum TemplateType {
    #[default]
    Manufacturers,
    ProductionCycles,
}

#[derive(Resource, Debug, Clone)]
pub struct Templates {
    pub manufacturers: Vec<ManufacturerTemplate>,
    pub production_cycles: Vec<ProductionCycleTemplate>,
    pub(crate) production_cycles_json: String,
    pub(crate) manufacturers_json: String,
    pub(crate) selected_template: TemplateType,
    production_cycles_path: String,
    manufacturers_path: String,
}

impl Default for Templates {
    fn default() -> Self {
        Self {
            manufacturers: Vec::new(),
            production_cycles: Vec::new(),
            production_cycles_json: String::new(),
            manufacturers_json: String::new(),
            selected_template: TemplateType::default(),
            production_cycles_path: "data/production_cycles.json".to_string(),
            manufacturers_path: "data/manufacturers.json".to_string(),
        }
    }
}

impl Templates {
    fn load(&mut self) {
        let (production_json, production_cycles) =
            Self::load_templates(&self.production_cycles_path)
                .expect("Unable to load production cycles");
        let (manufacturers_json, manufacturer_templates) =
            Self::load_templates(&self.manufacturers_path).expect("Unable to load manufacturers");
        self.manufacturers = manufacturer_templates;
        self.production_cycles = production_cycles;
        self.production_cycles_json = production_json;
        self.manufacturers_json = manufacturers_json;
    }
    pub(crate) fn save(&self) -> Result<(), Box<dyn Error>> {
        let manufacturers_json = serde_json::to_string_pretty(&self.manufacturers)?;
        let production_cycles_json = serde_json::to_string_pretty(&self.production_cycles)?;

        std::fs::write(&self.manufacturers_path, manufacturers_json)?;
        std::fs::write(&self.production_cycles_path, production_cycles_json)?;

        Ok(())
    }

    pub(crate) fn validate(&self) -> (Vec<String>, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let production_cycle_workdays: HashMap<_, _> = self
            .production_cycles
            .iter()
            .map(|p| (p.name.clone(), p.workdays_needed))
            .collect();
        let production_cycle_names: HashSet<_> = self
            .production_cycles
            .iter()
            .map(|p| p.name.clone())
            .collect();
        let mut production_cycle_references: HashSet<String> = HashSet::new();

        for manufacturer in &self.manufacturers {
            if production_cycle_names.contains(&manufacturer.production_cycle) {
                production_cycle_references.insert(manufacturer.production_cycle.clone());
                // Get the workdays_needed for this manufacturer's production cycle
                let cycle_workdays = production_cycle_workdays
                    .get(&manufacturer.production_cycle)
                    .expect("Production cycle not found"); // This shouldn't fail since we've already checked that the cycle exists

                // Compare the workdays_needed to the manufacturer's number of workers
                if *cycle_workdays > manufacturer.workers.len() as u32 {
                    warnings.push(format!(
                        "Manufacturer {} has fewer workers ({}) than workdays needed ({}) for production cycle {}",
                        manufacturer.name, manufacturer.workers.len(), cycle_workdays, manufacturer.production_cycle
                    ));
                }
            } else {
                errors.push(format!(
                    "Manufacturer {} has invalid production cycle {}",
                    manufacturer.name, manufacturer.production_cycle
                ));
            }
        }

        for name in production_cycle_names.difference(&production_cycle_references) {
            warnings.push(format!(
                "Production cycle {} is not referenced by any manufacturer",
                name
            ));
        }

        warnings.append(&mut self.validate_input_materials());

        (errors, warnings)
    }

    fn validate_input_materials(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Create a set of all materials that are produced
        let produced_materials: HashSet<_> = self
            .production_cycles
            .iter()
            .map(|p| p.output.0.clone())
            .collect();

        // Check each production cycle's inputs against the set of produced materials
        for production_cycle in &self.production_cycles {
            for input_material in production_cycle.input.keys() {
                if !produced_materials.contains(input_material) {
                    warnings.push(format!(
                        "Input material {} in production cycle {} cannot be created",
                        input_material, production_cycle.name
                    ));
                }
            }
        }

        warnings
    }

    fn load_templates<T: DeserializeOwned>(
        file_name: &str,
    ) -> Result<(String, Vec<T>), Box<dyn Error>> {
        let mut file = File::open(file_name)?;
        let mut json_string = String::new();
        file.read_to_string(&mut json_string)?;
        let templates: Vec<T> = serde_json::from_str(&json_string)?;
        Ok((json_string, templates))
    }
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
        names: &Res<Names>,
        commands: &mut Commands,
    ) -> Vec<ManufacturerBundle> {
        let mut manufacturers = Vec::new();
        for _ in 0..self.copies {
            let workers = self
                .workers
                .iter()
                .map(|w| {
                    commands
                        .spawn((
                            *w,
                            Wallet { money: Money(0) },
                            Person::default(),
                            TransactionLog::default(),
                            Name::new(people::generate_name(names)),
                        ))
                        .id()
                })
                .collect::<Vec<_>>();
            let manufacturer = ManufacturerBundle {
                name: Name::new(self.name.clone()),
                manufacturer: Manufacturer {
                    production_cycle: production_cycles.get(&self.production_cycle)
                        .cloned()
                        .unwrap_or_else(|| panic!("{} not found, make sure production cycle with this name is defined in production_cycles.json", self.production_cycle)),
                    assets: Inventory {
                        items: HashMap::new(),
                        items_to_sell: Default::default(),
                    },
                    hired_workers: workers,
                },
                wallet: Wallet {
                    money: self.money,
                },
                sell_strategy: self.sell_strategy,
                transaction_log: TransactionLog::default(),
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

pub fn init_people(mut names: ResMut<Names>, mut needs: ResMut<Needs>) {
    names.load();
    needs.load();
}

pub fn init_manufacturers(
    mut commands: Commands,
    mut templates: ResMut<Templates>,
    names: Res<Names>,
) {
    templates.load();
    let production_cycles = templates
        .clone()
        .production_cycles
        .into_iter()
        .map(|p| p.to_production_cycle())
        .collect::<HashMap<_, _>>();
    info!("Loaded {} production cycles", production_cycles.len());
    info!(
        "Loaded {} manufacturer templates",
        templates.manufacturers.len()
    );
    for template in templates.clone().manufacturers {
        let manufacturers =
            template.to_manufacturer(production_cycles.clone(), &names, &mut commands);
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
                        outstanding_orders: HashMap::new(),
                    },
                ));
            }
        }
    }
}
