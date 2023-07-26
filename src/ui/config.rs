use std::{fmt::Display, fs};

use bevy::prelude::*;
use bevy_egui::{
    egui::{self, emath::Numeric, Ui},
    EguiContexts,
};
use enum_display_derive::Display;

use crate::config::{Config, ConfigValue, CONFIG_PATH, DEFAULT_CONFIG_PATH};

#[derive(PartialEq, Eq, Display)]
pub enum SettingsPanel {
    Init,
    People,
    Business,
    Goverment,
}

#[derive(Resource)]
pub struct UiState {
    pub open_settings_panel: SettingsPanel,
}

pub fn settings(
    mut egui_context: EguiContexts,
    mut config: ResMut<Config>,
    mut state: ResMut<UiState>,
) {
    egui::Window::new("Config").show(egui_context.ctx_mut(), |ui| {
        ui.collapsing("Instructions", |ui| {
            ui.label("Most of the values you adjust here will take effect immediately.");
            ui.label("You can hover over the option name to see an extended tooltip of what it does.");
            ui.label("If you wish to change the value precisely you can drag the numeric value or double click to edit it.");
            ui.label(format!("If range of the values is too small you can edit the {} file and edit the matching \"range\" entry or you can just remove it completely.", CONFIG_PATH));
        });
        ui.horizontal(|ui| {
            add_settings_panel(ui, &mut state.open_settings_panel, SettingsPanel::Init);
            add_settings_panel(ui, &mut state.open_settings_panel, SettingsPanel::People);
            add_settings_panel(ui, &mut state.open_settings_panel, SettingsPanel::Business);
            add_settings_panel(ui, &mut state.open_settings_panel, SettingsPanel::Goverment);
            let space_left = ui.available_size() - egui::Vec2 { x: 100.0, y: 0.0 };
            ui.allocate_space(space_left);
            if ui.button("Default").clicked() {
                let data = fs::read_to_string(DEFAULT_CONFIG_PATH).expect("Unable to read config file");
                let default_config: Config = serde_json::from_str(&data).expect("Unable to parse config file");
                *config = default_config;
            }
            if ui.button("Save").clicked() {
                let file_content = serde_json::to_string_pretty(config.as_ref())
                    .expect("Unable to serialize configuration for saving!");
                fs::write(CONFIG_PATH, file_content).expect("Unable to save config data!");
            }
        });
        ui.separator();
        match state.open_settings_panel {
            SettingsPanel::Init => add_options_grid(ui, |ui| {
                draw_config_value(ui, &mut config.init.people.poor);
                draw_config_value(ui, &mut config.init.people.rich);
                draw_config_value(ui, &mut config.init.people.poor_starting_money);
                draw_config_value(ui, &mut config.init.people.rich_starting_money);
            }),
            SettingsPanel::People =>
                add_options_grid(ui, |ui| {
                    draw_config_value(ui, &mut config.people.max_buy_orders_per_day);
                    draw_config_value(ui, &mut config.people.discount_rate);
                }),
            SettingsPanel::Business => add_options_grid(ui, |ui| {
                draw_config_value(ui, &mut config.business.prices.max_change_per_day);
                draw_config_value(ui, &mut config.business.prices.sell_history_to_consider);
                draw_config_value(ui, &mut config.business.goal_produced_cycles_count);
                draw_config_value(ui, &mut config.business.keep_resources_for_cycles_amount);
                draw_config_value(ui, &mut config.business.min_days_between_staff_change);
                draw_config_value(ui, &mut config.business.money_to_create_business);
                draw_config_value(ui, &mut config.business.monthly_dividend);
                draw_config_value(ui, &mut config.business.new_worker_salary);
                draw_config_value(ui, &mut config.business.market.amount_of_sell_orders_seen);
                draw_config_value(ui, &mut config.business.market.amount_of_sell_orders_to_choose_best_price_from);
            }),
            SettingsPanel::Goverment => add_options_grid(ui, |ui| {
                draw_config_value(ui, &mut config.goverment.min_time_between_business_creation);
            })
        }
    });
}

fn _draw_bool_config_value(ui: &mut Ui, value: &mut ConfigValue<bool>) {
    let label = ui.label(&value.name);
    if let Some(hint) = &value.description {
        label.on_hover_text(hint);
    }
    ui.checkbox(&mut value.value, "");
}

fn add_settings_panel(ui: &mut Ui, value: &mut SettingsPanel, label: SettingsPanel) {
    let text = label.to_string();
    ui.selectable_value(value, label, text);
}

pub fn add_options_grid<R>(ui: &mut Ui, f: impl FnOnce(&mut Ui) -> R) {
    egui::Grid::new("options_grid")
        .num_columns(2)
        .spacing([40.0, 4.0])
        .striped(true)
        .show(ui, f);
}

pub fn draw_config_value<T: Numeric>(ui: &mut Ui, value: &mut ConfigValue<T>) {
    let label = ui.label(&value.name);
    if let Some(hint) = &value.description {
        label.on_hover_text(hint);
    }
    if let Some((min, max)) = value.range {
        ui.add(egui::Slider::new(&mut value.value, min..=max));
    } else {
        ui.add(egui::DragValue::new(&mut value.value).speed(0.1));
    }
    ui.end_row();
}
