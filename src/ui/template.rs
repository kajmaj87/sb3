use std::process::Command;

use bevy::prelude::ResMut;
use bevy_egui::egui::{Button, ScrollArea, TextEdit, TextStyle, Window};
use bevy_egui::EguiContexts;

use macros::measured;

use crate::init::{ManufacturerTemplate, ProductionCycleTemplate, TemplateType, Templates};
use crate::ui::debug::Performance;

#[measured]
pub fn render_template_editor(mut egui_context: EguiContexts, mut templates: ResMut<Templates>) {
    Window::new("Template editor").show(egui_context.ctx_mut(), |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            let (errors, warnings) = templates.validate();
            ui.radio_value(
                &mut templates.selected_template,
                TemplateType::Manufacturers,
                "Manufacturers",
            );
            ui.radio_value(
                &mut templates.selected_template,
                TemplateType::ProductionCycles,
                "Production cycles",
            );
            let mut json_error = "".to_string();
            let (text, json_error) = {
                match templates.selected_template {
                    TemplateType::Manufacturers => {
                        let manufacturers = serde_json::from_str::<Vec<ManufacturerTemplate>>(
                            &templates.manufacturers_json,
                        );
                        match manufacturers {
                            Ok(manufacturers) => templates.manufacturers = manufacturers,
                            Err(error) => {
                                json_error = error.to_string();
                            }
                        }
                        (&mut templates.manufacturers_json, &mut json_error)
                    }
                    TemplateType::ProductionCycles => {
                        let production_cycles = serde_json::from_str::<Vec<ProductionCycleTemplate>>(
                            &templates.production_cycles_json,
                        );
                        match production_cycles {
                            Ok(production_cycles) => templates.production_cycles = production_cycles,
                            Err(error) => {
                                json_error = error.to_string();
                            }
                        }
                        (&mut templates.production_cycles_json, &mut json_error)
                    }
                }
            };
            if !json_error.is_empty() {
                ui.label(format!("JSON error: {}", json_error));
            }
            if !errors.is_empty() {
                ui.label("Errors:");
                ui.vertical(|ui| {
                    let mut sorted_errors = errors.clone();
                    sorted_errors.sort();
                    for error in sorted_errors {
                        ui.label(error);
                    }
                });
            }
            if !warnings.is_empty() {
                ui.label("Warnings:");
                ui.vertical(|ui| {
                    let mut sorted_warnings = warnings.clone();
                    sorted_warnings.sort();
                    for warning in sorted_warnings {
                        ui.label(warning);
                    }
                });
            }
            if ui.add_enabled(json_error.is_empty() && errors.is_empty(), Button::new("Save & Restart")).clicked() {
                let _ = templates.save(); // TODO: handle error
                let args: Vec<String> = std::env::args().collect();
                Command::new(&args[0])
                    .args(&args[1..])
                    .spawn()
                    .expect("Failed to restart application");

                std::process::exit(0);
            }
            ui.add(
                TextEdit::multiline(text)
                    .font(TextStyle::Monospace) // for cursor height
                    .code_editor()
                    .desired_rows(10)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
                // .layouter(&mut layouter),
            );
        });
    });
}
