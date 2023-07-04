mod business;
mod config;
mod init;
mod money;
mod people;
mod stats;
mod ui;
mod user_input;

use crate::config::Config;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use serde::Deserialize;
use serde_json::from_reader;
use std::fs::File;

#[derive(Deserialize, Resource, Debug)]
pub struct BuildInfo {
    timestamp: String,
    version: String,
    commit_hash: String,
    branch_name: String,
}

fn main() {
    let file = File::open("build_info.json").expect("Failed to open file");
    let info: BuildInfo = from_reader(file).expect("Failed to deserialize");
    info!("Build Info: {:?}", info);
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,sb3=debug".into(),
            level: bevy::log::Level::WARN,
        }))
        .add_plugin(EguiPlugin)
        .add_plugin(config::ConfigPlugin)
        .insert_resource(Days {
            days: 0,
            next_turn: false,
            last_update: 0.0,
        })
        .insert_resource(Counter(0))
        .insert_resource(stats::PriceHistory::default())
        .insert_resource(info)
        .add_system(user_input::input_system.in_base_set(CoreSet::First))
        .add_system(
            date_update_system
                .run_if(should_advance_day)
                .in_base_set(CoreSet::PreUpdate),
        )
        .add_system(count_system.run_if(next_turn))
        .add_system(business::produce.run_if(next_turn))
        .add_system(business::create_sell_orders.run_if(next_turn))
        .add_system(business::update_sell_order_prices.run_if(next_turn))
        .add_system(business::create_buy_orders.run_if(next_turn))
        .add_system(business::execute_orders_for_manufacturers.run_if(next_turn))
        .add_system(stats::add_sell_orders_to_history.run_if(next_turn))
        .add_system(turn_end_system.in_base_set(CoreSet::PostUpdate))
        .add_system(ui::render_panels)
        .add_system(ui::render_todays_prices)
        .add_system(ui::render_price_history)
        .add_system(ui::render_manufacturers_stats)
        .add_startup_system(init::init_manufacturers)
        .run();
}

#[derive(Resource)]
pub struct Days {
    days: usize,
    next_turn: bool,
    last_update: f32,
}

impl Days {
    fn next_day(&mut self, time: Res<Time>) {
        self.days += 1;
        self.next_turn = true;
        self.last_update = time.elapsed_seconds();
    }
}

#[derive(Resource)]
pub struct Counter(usize);

fn date_update_system(mut days: ResMut<Days>, time: Res<Time>) {
    days.next_day(time);
}

fn count_system(mut counter: ResMut<Counter>) {
    counter.0 += 1;
}

fn should_advance_day(time: Res<Time>, days: Res<Days>, config: Res<Config>) -> bool {
    if config.game.speed.value == 0.0 {
        return false;
    }
    time.elapsed_seconds() - days.last_update > config.game.speed.value
}

fn turn_end_system(mut days: ResMut<Days>) {
    days.next_turn = false;
}

fn next_turn(days: Res<Days>) -> bool {
    days.next_turn
}
