use bevy::prelude::{EventWriter, Res, ResMut, Resource};
use bevy_egui::egui::{Align, Hyperlink, Layout, SidePanel, TopBottomPanel, Widget};
use bevy_egui::EguiContexts;

use macros::measured;

use crate::commands::GameCommand;
use crate::ui::debug::Performance;
use crate::ui::logs::LoggingFilterType;
use crate::ui::manufacturers::ManufacturerSort;
use crate::ui::people::PeopleSort;
use crate::{BuildInfo, Days};

#[measured]
pub fn render_panels(
    mut egui_context: EguiContexts,
    days: Res<Days>,
    build_info: Res<BuildInfo>,
    mut game_commands: EventWriter<GameCommand>,
) {
    TopBottomPanel::top("top_panel").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("Space Business v{}", build_info.version));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button("x32")
                    .on_hover_text("[key: 6] Set the game speed to x32k days per second")
                    .clicked()
                {
                    game_commands.send(GameCommand::SetSpeed(16.0));
                }
                if ui
                    .button("x16")
                    .on_hover_text("[key: 5] Set the game speed to x16 days per second")
                    .clicked()
                {
                    game_commands.send(GameCommand::SetSpeed(16.0));
                }
                if ui
                    .button("x8")
                    .on_hover_text("[key: 4] Set the game speed to x8 days per second")
                    .clicked()
                {
                    game_commands.send(GameCommand::SetSpeed(8.0));
                }
                if ui
                    .button("x4")
                    .on_hover_text("[key: 3] Set the game speed to x4 days per second")
                    .clicked()
                {
                    game_commands.send(GameCommand::SetSpeed(4.0));
                }
                if ui
                    .button("x2")
                    .on_hover_text("[key: 2] Set the game speed to x2 days per second")
                    .clicked()
                {
                    game_commands.send(GameCommand::SetSpeed(2.0));
                }
                if ui
                    .button("x1")
                    .on_hover_text("[key: 1] Set the game speed to x1 day per second")
                    .clicked()
                {
                    game_commands.send(GameCommand::SetSpeed(1.0));
                }
                if ui
                    .button("N")
                    .on_hover_text("[key: ENTER] Advance to next day")
                    .clicked()
                {
                    game_commands.send(GameCommand::AdvanceDay);
                }
                if ui
                    .button("P")
                    .on_hover_text("[key: `] Pause the game")
                    .clicked()
                {
                    game_commands.send(GameCommand::SetSpeed(0.0));
                }
                ui.label(format!("Days: {}", days.days));
            });
        });
    });

    TopBottomPanel::bottom("bottom_panel").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("Build at: {}", build_info.timestamp));
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Branch: ");
                let link = Hyperlink::from_label_and_url(
                    build_info.branch_name.as_str(),
                    format!(
                        "https://github.com/kajmaj87/sb3/tree/{}",
                        build_info.branch_name
                    ),
                );
                link.ui(ui);
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Commit: ");
                let link = Hyperlink::from_label_and_url(
                    build_info.commit_hash.as_str(),
                    format!(
                        "https://github.com/kajmaj87/sb3/commit/{}",
                        build_info.commit_hash
                    ),
                );
                link.ui(ui);
            });
        });
    });
    SidePanel::left("left_panel")
        .resizable(true)
        .max_width(200.0)
        .show(egui_context.ctx_mut(), |ui| {
            ui.label("Left panel");
        });
    SidePanel::right("right_panel").show(egui_context.ctx_mut(), |ui| {
        ui.label("Right panel");
    });
}

#[derive(Resource)]
pub struct UiState {
    pub manufacturers: ManufacturerSort,
    pub manufacturers_pinned: bool,
    pub people: PeopleSort,
    pub people_pinned: bool,
    pub logging_filter: String,
    pub logging_filter_type: LoggingFilterType,
    pub max_log_lines: usize,
    pub fuzzy_match_threshold: i64,
    pub fuzzy_match_order: bool,
    pub regex_error: Option<String>,
}
