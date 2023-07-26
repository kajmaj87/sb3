use std::fs;

use crate::money::Money;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const CONFIG_PATH: &str = "./data/config.json";

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
pub struct Goverment {
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
    pub goverment: Goverment,
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
        let data = fs::read_to_string(CONFIG_PATH).expect("Unable to read config file");
        let config: Config = serde_json::from_str(&data).expect("Unable to parse config file");
        debug!("Read configuration: {:?}", config);
        app.insert_resource(config);
    }
}
