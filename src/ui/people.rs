use bevy::core::Name;
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};
use bevy_egui::egui::{Align, Layout, Window};
use bevy_egui::EguiContexts;
use egui_extras::{Column, TableBuilder};

use macros::measured;

use crate::business::{Manufacturer, Worker};
use crate::logs::Pinned;
use crate::money::Money;
use crate::people::Person;
use crate::ui::debug::Performance;
use crate::ui::main_layout::UiState;
use crate::ui::utilities::{count_items, items_to_string, label_with_hover_text};
use crate::wallet::Wallet;
use crate::Days;

#[allow(clippy::too_many_arguments)]
#[measured]
pub fn render_people_stats(
    mut egui_context: EguiContexts,
    people: Query<(Entity, &Name, &Wallet, &Person)>,
    workers: Query<&Worker>,
    manufacturers: Query<(Entity, &Name, &Manufacturer)>,
    mut ui_state: ResMut<UiState>,
    pinned: Query<&Pinned>,
    mut commands: Commands,
    date: Res<Days>,
) {
    Window::new("People").show(egui_context.ctx_mut(), |ui| {
        let total_money = people
            .iter()
            .map(|(_, _, wallet, _)| wallet.money())
            .sum::<Money>();
        ui.label(format!("Total people money: {}", total_money));
        let employment = workers.iter().count() as f32 / people.iter().count() as f32;
        ui.label(format!(
            "Unemployment rate: {:.2}%",
            (1.0 - employment) * 100.0
        ));
        let table = TableBuilder::new(ui)
            // .striped(self.striped)
            // .resizable(self.resizable)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .min_scrolled_height(0.0);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    if ui.button("Pin").clicked() {
                        ui_state.people_pinned = !ui_state.people_pinned;
                    }
                });
                header.col(|ui| {
                    if ui.button("Name").clicked() {
                        ui_state.people = PeopleSort::Name;
                    }
                });
                header.col(|ui| {
                    if ui.button("Money").clicked() {
                        ui_state.people = PeopleSort::Money;
                    }
                });
                header.col(|ui| {
                    if ui.button("Items").clicked() {
                        ui_state.people = PeopleSort::Items;
                    }
                });
                header.col(|ui| {
                    if ui.button("Utility").clicked() {
                        ui_state.people = PeopleSort::Utility;
                    }
                });
                header.col(|ui| {
                    if ui.button("Employer").clicked() {
                        ui_state.people = PeopleSort::Employer;
                    }
                });
                header.col(|ui| {
                    if ui.button("Salary").clicked() {
                        ui_state.people = PeopleSort::Salary;
                    }
                });
            })
            .body(|mut body| {
                let mut rows = people
                    .iter()
                    .map(|(entity, name, wallet, person)| PersonRow {
                        entity,
                        pinned: pinned.get(entity).is_ok(),
                        name: name.to_string(),
                        money: wallet.money(),
                        money_text: wallet.get_summary(date.days, 30, 30),
                        items: count_items(&person.assets.items),
                        items_text: items_to_string(&person.assets.items),
                        utility: person.utility,
                        employed_at: workers
                            .get(entity)
                            .and_then(|w| {
                                if let Some(employer) = w.employed_at {
                                    manufacturers
                                        .get(employer)
                                        .map(|(_, name, _)| name.to_string())
                                } else {
                                    Ok("<<UNEMPLOYED>>".to_string())
                                }
                            })
                            .unwrap_or_else(|_| "<<UNEMPLOYED>>".to_string()),
                        salary: workers.get(entity).map(|w| w.salary).unwrap_or(Money(0)),
                    })
                    .collect::<Vec<_>>();
                match ui_state.people {
                    PeopleSort::Name => rows.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap()),
                    PeopleSort::Money => {
                        rows.sort_by(|a, b| b.money.partial_cmp(&a.money).unwrap())
                    }
                    PeopleSort::Items => {
                        rows.sort_by(|a, b| b.items.partial_cmp(&a.items).unwrap())
                    }
                    PeopleSort::Utility => {
                        rows.sort_by(|a, b| b.utility.partial_cmp(&a.utility).unwrap())
                    }
                    PeopleSort::Employer => {
                        rows.sort_by(|a, b| a.employed_at.partial_cmp(&b.employed_at).unwrap())
                    }
                    PeopleSort::Salary => {
                        rows.sort_by(|a, b| b.salary.partial_cmp(&a.salary).unwrap())
                    }
                }

                for r in rows.iter().filter(|r| r.pinned || !ui_state.people_pinned) {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            if r.pinned {
                                if ui.button("U").on_hover_text("Unpin this person").clicked() {
                                    commands.entity(r.entity).remove::<Pinned>();
                                }
                            } else if ui.button("P").on_hover_text("Pin this person").clicked() {
                                commands.entity(r.entity).insert(Pinned {});
                            }
                        });
                        row.col(|ui| {
                            ui.label(&r.name);
                        });
                        row.col(|ui| {
                            ui.label(&r.money.to_string()).on_hover_text(&r.money_text);
                        });
                        row.col(|ui| {
                            label_with_hover_text(ui, r.items, &r.items_text);
                        });
                        row.col(|ui| {
                            ui.label(&r.utility.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&r.employed_at);
                        });
                        row.col(|ui| {
                            ui.label(&r.salary.to_string());
                        });
                    });
                }
            });
    });
}

pub enum PeopleSort {
    Name,
    Money,
    Items,
    Utility,
    Employer,
    Salary,
}

struct PersonRow {
    entity: Entity,
    name: String,
    money: Money,
    money_text: String,
    items: usize,
    utility: f64,
    items_text: String,
    employed_at: String,
    salary: Money,
    pinned: bool,
}
