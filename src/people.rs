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
    }
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
