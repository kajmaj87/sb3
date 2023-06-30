use crate::business::{ItemType, SellOrder};
use crate::money::Money;
use crate::stats::PriceHistory;
use crate::{BuildInfo, Days};
use bevy::prelude::{Query, Res};
use bevy_egui::egui::plot::{
    BoxElem, BoxPlot, BoxSpread, Legend, Line, LineStyle, Plot, PlotPoints,
};
use bevy_egui::egui::{
    Align, Color32, Hyperlink, Layout, SidePanel, TopBottomPanel, Widget, Window,
};
use bevy_egui::EguiContexts;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

pub fn render_panels(mut egui_context: EguiContexts, days: Res<Days>, build_info: Res<BuildInfo>) {
    TopBottomPanel::top("top_panel").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("Space Business v{}", build_info.version));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
