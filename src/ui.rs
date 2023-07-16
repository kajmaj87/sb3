use crate::business::{BuyOrder, ItemType, Manufacturer, SellOrder, Wallet};
use crate::commands::GameCommand;
use crate::debug_ui::Performance;
use crate::init::{ManufacturerTemplate, ProductionCycleTemplate, TemplateType, Templates};
use crate::money::Money;
use crate::people::Person;
use crate::stats::PriceHistory;
use crate::{BuildInfo, Days};
use bevy::core::Name;
use bevy::prelude::{Entity, EventWriter, Query, Res, ResMut, Resource};
use bevy_egui::egui::plot::{
    BoxElem, BoxPlot, BoxSpread, Legend, Line, LineStyle, Plot, PlotPoints,
};
use bevy_egui::egui::{
    Align, Button, Color32, Hyperlink, Layout, SidePanel, TopBottomPanel, Widget, Window,
};
use bevy_egui::{egui, EguiContexts};
use egui_extras::{Column, TableBuilder};
use macros::measured;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::process::Command;

#[measured]
pub fn render_template_editor(mut egui_context: EguiContexts, mut templates: ResMut<Templates>) {
    Window::new("Template editor").show(egui_context.ctx_mut(), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
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
                egui::TextEdit::multiline(text)
                    .font(egui::TextStyle::Monospace) // for cursor height
                    .code_editor()
                    .desired_rows(10)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
                // .layouter(&mut layouter),
            );
        });
    });
}

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

#[measured]
pub fn render_todays_prices(mut egui_context: EguiContexts, sell_orders: Query<&SellOrder>) {
    Window::new("Prices").show(egui_context.ctx_mut(), |ui| {
        let mut grouped_orders = BTreeMap::new();

        for sell_order in sell_orders.iter() {
            grouped_orders
                .entry(sell_order.item_type.clone())
                .or_insert_with(Vec::new)
                .push(sell_order.price);
        }

        let mut i = 0;
        let mut box_plots = vec![];
        for (item_type, prices) in grouped_orders {
            let len = prices.len();
            i += 1;
            let mut prices = prices;
            prices.sort_unstable();

            let min = prices.first().unwrap().as_u64();
            let max = prices.last().unwrap().as_u64();
            let median = prices[len / 2].as_u64();
            let p25 = prices[(len as f32 * 0.25).floor() as usize].as_u64();
            let p75 = prices[(len as f32 * 0.75).floor() as usize].as_u64();
            let len = prices.len();
            let avg = (prices.iter().sum::<Money>() / len).as_u64();
            box_plots.push(create_box_plot(
                &item_type, i, min, p25, median, p75, max, len, avg,
            ));
        }
        Plot::new("Prices today")
            .legend(Legend::default())
            .show(ui, |ui| {
                for box_plot in box_plots.drain(..) {
                    ui.box_plot(box_plot);
                }
            });
    });
}

#[allow(clippy::too_many_arguments)]
fn create_box_plot(
    item_type: &ItemType,
    x: u64,
    min: u64,
    p25: u64,
    median: u64,
    p75: u64,
    max: u64,
    len: usize,
    avg: u64,
) -> BoxPlot {
    BoxPlot::new(vec![BoxElem::new(
        x as f64,
        BoxSpread::new(
            min as f64,
            p25 as f64,
            median as f64,
            p75 as f64,
            max as f64,
        ),
    )
    .name(format!("Total Items: {}\nAvg: {}", len, avg))])
    .name(item_type.name.as_str())
    .color(string_to_rgb(item_type.name.as_str()))
}

pub fn render_price_history(history: Res<PriceHistory>, mut egui_context: EguiContexts) {
    Window::new("Price History").show(egui_context.ctx_mut(), |ui| {
        let mut line_avg = HashMap::new();
        let mut line_p25 = HashMap::new();
        let mut line_p75 = HashMap::new();
        for (item_type, price_history) in history.prices.iter() {
            let mut avgs = vec![];
            let mut p25s = vec![];
            let mut p75s = vec![];
            for prices in price_history.iter() {
                // let len = prices.total_orders;
                // let min = prices.min;
                // let max = prices.max;
                // let median = prices.median;
                let p25 = prices.p25;
                let p75 = prices.p75;
                let avg = prices.avg;
                let day = prices.day;
                avgs.push([day as f64, avg as f64]);
                p25s.push([day as f64, p25 as f64]);
                p75s.push([day as f64, p75 as f64]);
            }
            line_avg.insert(item_type.clone(), avgs);
            line_p25.insert(item_type.clone(), p25s);
            line_p75.insert(item_type.clone(), p75s);
        }
        Plot::new("Price history")
            .legend(Legend::default())
            .show(ui, |ui| {
                for (item_type, points) in line_avg {
                    ui.line(
                        Line::new(PlotPoints::new(points))
                            .color(string_to_rgb(item_type.name.as_str()))
                            .name(item_type.name.as_str()),
                    );
                }
                for (item_type, points) in line_p25 {
                    ui.line(
                        Line::new(PlotPoints::new(points))
                            .color(string_to_rgb(item_type.name.as_str()))
                            .name(item_type.name.as_str())
                            .style(LineStyle::Dashed { length: 7.0 }),
                    );
                }
                for (item_type, points) in line_p75 {
                    ui.line(
                        Line::new(PlotPoints::new(points))
                            .color(string_to_rgb(item_type.name.as_str()))
                            .name(item_type.name.as_str())
                            .style(LineStyle::Dashed { length: 7.0 }),
                    );
                }
            });
    });
}

#[measured]
pub fn render_manufacturers_stats(
    mut egui_context: EguiContexts,
    manufacturers: Query<(Entity, &Name, &Wallet, &Manufacturer)>,
    sell_orders: Query<&SellOrder>,
    buy_orders: Query<&BuyOrder>,
    mut sort_order: ResMut<SortOrder>,
) {
    Window::new("Manufacturers").show(egui_context.ctx_mut(), |ui| {
        let mut owner_counts: HashMap<Entity, usize> = HashMap::new();

        for order in sell_orders.iter() {
            *owner_counts.entry(order.seller).or_insert(0) += 1;
        }
        let table = TableBuilder::new(ui)
            // .striped(self.striped)
            // .resizable(self.resizable)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto())
            .column(Column::initial(80.0).range(80.0..=200.0))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .min_scrolled_height(0.0);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    if ui.button("Name").clicked() {
                        sort_order.manufacturers = ManufacturerSort::Name;
                    }
                });
                header.col(|ui| {
                    if ui.button("Money").clicked() {
                        sort_order.manufacturers = ManufacturerSort::Money;
                    }
                });
                header.col(|ui| {
                    if ui.button("Workers").clicked() {
                        sort_order.manufacturers = ManufacturerSort::Workers;
                    }
                });
                header.col(|ui| {
                    if ui.button("Items").clicked() {
                        sort_order.manufacturers = ManufacturerSort::Items;
                    }
                });
                header.col(|ui| {
                    if ui.button("Items to sell").clicked() {
                        sort_order.manufacturers = ManufacturerSort::ItemsToSell;
                    }
                });
                header.col(|ui| {
                    if ui.button("On market").clicked() {
                        sort_order.manufacturers = ManufacturerSort::OnMarket;
                    }
                });
                header.col(|ui| {
                    if ui.button("Buy orders").clicked() {
                        sort_order.manufacturers = ManufacturerSort::BuyOrders;
                    }
                });
            })
            .body(|mut body| {
                let buy_order_by_type: HashMap<ItemType, usize> = buy_orders
                    .iter()
                    .map(|x| x.item_type.clone())
                    .fold(HashMap::new(), |mut acc, x| {
                        *acc.entry(x).or_insert(0) += 1;
                        acc
                    });
                let mut rows = manufacturers
                    .iter()
                    .map(|(entity, name, wallet, manufacturer)| ManufacturerRow {
                        name: name.to_string(),
                        money: wallet.money,
                        workers: manufacturer.hired_workers.len(),
                        items: manufacturer
                            .assets
                            .items
                            .values()
                            .map(|x| x.len())
                            .sum::<usize>(),
                        items_to_sell: manufacturer.assets.items_to_sell.len(),
                        on_market: *owner_counts.get(&entity).unwrap_or(&0),
                        buy_orders: *buy_order_by_type
                            .get(&manufacturer.production_cycle.output.0)
                            .unwrap_or(&0),
                    })
                    .collect::<Vec<_>>();
                match sort_order.manufacturers {
                    ManufacturerSort::Name => {
                        rows.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap())
                    }
                    ManufacturerSort::Money => {
                        rows.sort_by(|a, b| b.money.partial_cmp(&a.money).unwrap())
                    }
                    ManufacturerSort::Workers => {
                        rows.sort_by(|a, b| b.workers.partial_cmp(&a.workers).unwrap())
                    }
                    ManufacturerSort::Items => {
                        rows.sort_by(|a, b| b.items.partial_cmp(&a.items).unwrap())
                    }
                    ManufacturerSort::ItemsToSell => {
                        rows.sort_by(|a, b| b.items_to_sell.partial_cmp(&a.items_to_sell).unwrap())
                    }
                    ManufacturerSort::OnMarket => {
                        rows.sort_by(|a, b| b.on_market.partial_cmp(&a.on_market).unwrap())
                    }
                    ManufacturerSort::BuyOrders => {
                        rows.sort_by(|a, b| b.buy_orders.partial_cmp(&a.buy_orders).unwrap())
                    }
                }

                for r in rows.iter() {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(&r.name);
                        });
                        row.col(|ui| {
                            ui.label(&r.money.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&r.workers.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&r.items.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&r.items_to_sell.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&r.on_market.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&r.buy_orders.to_string());
                        });
                    });
                }
            });
    });
}

#[measured]
pub fn render_people_stats(
    mut egui_context: EguiContexts,
    people: Query<(Entity, &Name, &Wallet, &Person)>,
    mut sort_order: ResMut<SortOrder>,
) {
    Window::new("People").show(egui_context.ctx_mut(), |ui| {
        let table = TableBuilder::new(ui)
            // .striped(self.striped)
            // .resizable(self.resizable)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::initial(80.0).range(80.0..=200.0))
            .min_scrolled_height(0.0);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    if ui.button("Name").clicked() {
                        sort_order.people = PeopleSort::Name;
                    }
                });
                header.col(|ui| {
                    if ui.button("Money").clicked() {
                        sort_order.people = PeopleSort::Money;
                    }
                });
                header.col(|ui| {
                    if ui.button("Items").clicked() {
                        sort_order.people = PeopleSort::Items;
                    }
                });
                header.col(|ui| {
                    if ui.button("Utility").clicked() {
                        sort_order.people = PeopleSort::Utility;
                    }
                });
            })
            .body(|mut body| {
                let mut rows = people
                    .iter()
                    .map(|(_, name, wallet, person)| PersonRow {
                        name: name.to_string(),
                        money: wallet.money,
                        items: person.assets.items.len(),
                        utility: person.utility,
                    })
                    .collect::<Vec<_>>();
                match sort_order.people {
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
                }

                for r in rows.iter() {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(&r.name);
                        });
                        row.col(|ui| {
                            ui.label(&r.money.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&r.items.to_string());
                        });
                        row.col(|ui| {
                            ui.label(&r.utility.to_string());
                        });
                    });
                }
            });
    });
}
#[derive(Resource)]
pub struct SortOrder {
    pub manufacturers: ManufacturerSort,
    pub people: PeopleSort,
}

pub enum ManufacturerSort {
    Name,
    Money,
    Workers,
    Items,
    ItemsToSell,
    OnMarket,
    BuyOrders,
}

pub enum PeopleSort {
    Name,
    Money,
    Items,
    Utility,
}

struct ManufacturerRow {
    pub name: String,
    pub money: Money,
    pub workers: usize,
    pub items: usize,
    pub items_to_sell: usize,
    pub on_market: usize,
    buy_orders: usize,
}

struct PersonRow {
    pub name: String,
    pub money: Money,
    items: usize,
    utility: f64,
}

// pub fn create_histogram(name: &str, values: &[u64], bins: u32) -> BarChart {
//     let mut histogram = HashMap::new();
//     let max = values.iter().max().unwrap_or(&0);
//     let min = values.iter().min().unwrap_or(&0);
//     let range = max - min + 1;
//     let bin_width = (range as f64 / bins as f64).ceil() as u64;
//     for &value in values {
//         *histogram.entry((value - min) / bin_width).or_insert(0) += 1;
//     }
//     let histogram: Vec<Bar> = histogram
//         .into_iter()
//         .map(|(bin, count)| {
//             Bar::new((bin * bin_width + min) as f64, count as f64).width(bin_width as f64)
//         })
//         .collect();
//     BarChart::new(histogram)
//         .color(Color32::LIGHT_BLUE)
//         .name(name)
// }

fn string_to_rgb(input: &str) -> Color32 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let hash = hasher.finish();

    let r = (hash >> 16) as u8;
    let g = (hash >> 8) as u8;
    let b = hash as u8;

    Color32::from_rgb(r, g, b)
}
