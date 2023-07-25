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
}

#[derive(Serialize, Deserialize, Debug, Resource)]
pub struct Config {
    pub game: GameConfig,
    pub people: People,
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
