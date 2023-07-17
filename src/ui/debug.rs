use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::egui::Window;
use bevy_egui::{egui, EguiContexts};
use egui_extras::{Column, TableBuilder};

use crate::business::Wallet;
use crate::money::Money;

#[derive(Resource)]
pub struct Performance {
    data: HashMap<String, VecDeque<Duration>>,
    max_entries: usize,
}

pub struct FunctionPerformance {
    pub name: String,
    pub total_duration: f64,
    pub min: Duration,
    pub p5: Duration,
    pub median: Duration,
    pub p95: Duration,
    pub max: Duration,
}

impl Performance {
    pub fn new(max_entries: usize) -> Self {
        Self {
            data: HashMap::new(),
            max_entries,
        }
    }

    pub fn add_duration(&mut self, function_name: &str, duration: Duration) {
        let entry = self
            .data
            .entry(function_name.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.max_entries));

        if entry.len() == self.max_entries {
            entry.pop_front();
        }

        entry.push_back(duration);
    }

    pub fn describe_all(&self) -> Vec<FunctionPerformance> {
        let mut function_stats: Vec<FunctionPerformance> = Vec::new();

        let total_duration_secs = &self.data.iter().fold(0.0, |acc, (_, durations)| {
            acc + durations.iter().sum::<Duration>().as_secs_f64()
        });

        for (name, durations) in &self.data {
            let count = durations.len();
            if count == 0 {
                continue;
            }

            let mut sorted_durations = durations.clone().into_iter().collect::<Vec<_>>();
            sorted_durations.sort_unstable();

            let min = sorted_durations[0];
            let p5 = sorted_durations[(count as f64 * 0.05) as usize];
            let median = sorted_durations[count / 2];
            let p95 = sorted_durations[(count as f64 * 0.95) as usize];
            let max = sorted_durations[count - 1];

            let total_duration =
                durations.iter().sum::<Duration>().as_secs_f64() / total_duration_secs * 100.0;

            function_stats.push(FunctionPerformance {
                name: name.to_string(),
                total_duration,
                min,
                p5,
                median,
                p95,
                max,
            });
        }

        function_stats.sort_by(|a, b| {
            b.total_duration
                .partial_cmp(&a.total_duration)
                .unwrap_or(Ordering::Equal)
        });

        function_stats
    }
}
pub fn debug_window(
    mut egui_context: EguiContexts,
    diagnostics: Res<DiagnosticsStore>,
    performance: Res<Performance>,
    wallets: Query<&Wallet>,
    entities: Query<Entity>,
) {
    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            Window::new("Debug").show(egui_context.ctx_mut(), |ui| {
                ui.label(format!("Rendering @{:.1}fps", average));
                ui.label(format!("Entities: {}", entities.iter().count()));
                ui.label(format!(
                    "Total Money: {}",
                    wallets.iter().fold(Money(0), |acc, w| acc + w.money)
                ));
                ui.collapsing("Performance Stats", |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::remainder())
                        .min_scrolled_height(0.0)
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("System");
                            });
                            header.col(|ui| {
                                ui.strong("Total Time (%)");
                            });
                            header.col(|ui| {
                                ui.strong("Min");
                            });
                            header.col(|ui| {
                                ui.strong("p5");
                            });
                            header.col(|ui| {
                                ui.strong("median");
                            });
                            header.col(|ui| {
                                ui.strong("p95");
                            });
                            header.col(|ui| {
                                ui.strong("max");
                            });
                        })
                        .body(|mut body| {
                            for f in performance.describe_all() {
                                body.row(18.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(f.name);
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:.2}", f.total_duration));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:#?}", f.min));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:#?}", f.p5));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:#?}", f.median));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:#?}", f.p95));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:#?}", f.max));
                                    });
                                });
                            }
                        });
                });
            });
        }
    }
}
