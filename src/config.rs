use std::fs;
use std::fs::{copy, create_dir_all, metadata};
use std::path::Path;

use crate::money::Money;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_PATH: &str = "./data/config.json";
pub const CONFIG_PATH: &str = "./run/config.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct PeopleInit {
    pub poor: ConfigValue<u32>,
    pub rich: ConfigValue<u32>,
    pub poor_starting_money: ConfigValue<Money>,
    pub rich_starting_money: ConfigValue<Money>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Init {
    pub people: PeopleInit,
}

#[derive(Serialize, Deserialize, Debug, Component)]
pub struct GameConfig {
    pub speed: ConfigValue<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct People {
    pub max_buy_orders_per_day: ConfigValue<u32>,
    pub discount_rate: ConfigValue<f64>,
    pub order_expiration_time: ConfigValue<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Prices {
    pub sell_history_to_consider: ConfigValue<usize>,
    pub max_change_per_day: ConfigValue<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Market {
    pub amount_of_sell_orders_seen: ConfigValue<f64>,
    pub amount_of_sell_orders_to_choose_best_price_from: ConfigValue<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Government {
    pub min_time_between_business_creation: ConfigValue<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Business {
    pub prices: Prices,
    pub market: Market,
    pub keep_resources_for_cycles_amount: ConfigValue<u32>,
    pub money_to_create_business: ConfigValue<Money>,
    pub new_worker_salary: ConfigValue<Money>,
    pub monthly_dividend: ConfigValue<f32>,
    pub min_days_between_staff_change: ConfigValue<u32>,
    pub goal_produced_cycles_count: ConfigValue<u32>,
}

#[derive(Serialize, Deserialize, Debug, Resource)]
pub struct Config {
    pub game: GameConfig,
    pub people: People,
    pub business: Business,
    pub government: Government,
    pub init: Init,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ConfigValue<T> {
    pub value: T,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<(T, T)>,
}

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let config_path = Path::new(CONFIG_PATH);
        let default_config_path = Path::new(DEFAULT_CONFIG_PATH);

        // Create directory if it does not exist
        if let Some(parent) = config_path.parent() {
            create_dir_all(parent).expect("Unable to create config directory");
        }

        let read_default = if !config_path.exists() {
            true
        } else {
            let config_metadata =
                metadata(config_path).expect("Unable to read config file metadata");
            let default_config_metadata =
                metadata(default_config_path).expect("Unable to read default config file metadata");

            default_config_metadata
                .modified()
                .expect("Unable to get default config file modification time")
                > config_metadata
                    .modified()
                    .expect("Unable to get config file modification time")
        };

        if read_default {
            copy(default_config_path, config_path)
                .expect("Unable to copy default config to config");
        }

        let data = fs::read_to_string(config_path).expect("Unable to read config file");
        let config: Config = serde_json::from_str(&data).expect("Unable to parse config file");
        debug!("Read configuration: {:?}", config);
        app.insert_resource(config);
    }
}
