use bevy::prelude::*;
use rand::prelude::SliceRandom;
use serde::Deserialize;

#[derive(Debug, Deserialize, Resource, Default, Clone)]
pub struct Names {
    first_names: Vec<String>,
    nicknames: Vec<String>,
    last_names: Vec<String>,
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

#[derive(Component)]
pub struct Person {}
