use std::fs::File;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use serde::Deserialize;
use serde_json::from_reader;

use ui::main_layout::UiState;
use ui::manufacturers::ManufacturerSort;
use ui::people::PeopleSort;

use crate::config::Config;
use crate::ui::logs::LoggingFilterType;

mod business;
mod commands;
mod config;
mod govement;
mod init;
mod invariants;
mod logs;
mod money;
mod people;
mod stats;
mod ui;
mod user_input;
mod wallet;

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
            filter: "info,wgpu_core=warn,wgpu_hal=warn,sb3=info".into(),
            level: bevy::log::Level::WARN,
        }))
        .add_plugins((EguiPlugin, config::ConfigPlugin, FrameTimeDiagnosticsPlugin))
        .insert_resource(Days {
            days: 0,
            next_turn: false,
            last_update: 0.0,
        })
        .insert_resource(stats::PriceHistory::default())
        .insert_resource(init::Templates::default())
        .insert_resource(people::Names::default())
        .insert_resource(people::Needs::default())
        .insert_resource(people::Items::default())
        .insert_resource(ui::config::UiState {
            open_settings_panel: ui::config::SettingsPanel::Init,
        })
        .insert_resource(info)
        .insert_resource(ui::debug::Performance::new(100))
        .insert_resource(UiState {
            manufacturers: ManufacturerSort::Name,
            manufacturers_pinned: false,
            people: PeopleSort::Name,
            people_pinned: false,
            logging_filter: "".to_string(),
            logging_filter_type: LoggingFilterType::Fuzzy,
            logs_delete_unpinned_old: true,
            logs_delete_unpinned_older_than: 50,
            logs_keep_pinned: true,
            logs_show_all_if_no_pins: true,
            max_log_lines: 250,
            fuzzy_match_threshold: 50,
            fuzzy_match_order: false,
            regex_error: None,
        })
        .insert_resource(logs::Logs::default())
        .add_event::<commands::GameCommand>()
        .add_event::<logs::LogEvent>()
        .add_systems(
            Startup,
            (
                init::init_templates,
                init::init_manufacturers,
                init::init_people,
            )
                .chain(),
        )
        .add_systems(Update, user_input::input_system)
        .add_systems(
            PreUpdate,
            (
                commands::command_system,
                date_update_system.run_if(should_advance_day),
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                // those system run in sequence
                business::order_expiration,
                business::salary_payout,
                business::execute_orders,
                // business::process_transactions,
                business::produce,
                (business::create_buy_orders, business::create_sell_orders), // those run in parallel
                business::assing_workers_to_businesses,
                business::fire_staff,
                business::create_job_offers,
                business::create_business,
                business::take_job_offers,
                business::update_sell_strategy_margin,
                business::update_sell_order_prices,
                business::payout_dividends,
                business::reduce_days_since_last_staff_change,
                govement::create_business_permit,
                people::consume,
                people::create_buy_orders_for_people,
                stats::add_sell_orders_to_history,
            )
                .chain()
                .run_if(next_turn),
        )
        .add_systems(Update, logs::logging_system)
        .add_systems(Update, logs::delete_old_logs_system)
        .add_systems(Update, ui::debug::debug_window)
        .add_systems(
            Update,
            (
                ui::manufacturers::render_manufacturers_stats,
                ui::people::render_people_stats,
                ui::main_layout::render_panels,
                ui::prices::render_price_history,
                ui::template::render_template_editor,
                ui::prices::render_todays_prices,
                ui::logs::render_logs,
                ui::config::settings,
            ),
        )
        .add_systems(PostUpdate, turn_end_system)
        .add_systems(
            PostUpdate,
            (
                invariants::each_hired_worker_should_have_correct_employer,
                (
                    business::merge_sell_orders,
                    business::delete_empty_sell_orders,
                )
                    .chain(),
            )
                .run_if(next_turn),
        )
        .add_systems(Last, business::bankruption)
        .run();
}

#[derive(Resource)]
pub struct Days {
    days: usize,
    next_turn: bool,
    last_update: f32,
}

impl Days {
    fn next_day(&mut self, time: &Res<Time>) {
        self.days += 1;
        self.next_turn = true;
        self.last_update = time.elapsed_seconds();
    }
}

fn date_update_system(mut days: ResMut<Days>, time: Res<Time>) {
    days.next_day(&time);
    info!("Day {} started", days.days);
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
