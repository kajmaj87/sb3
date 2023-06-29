use crate::business::SellOrder;
use crate::{Counter, Days};
use bevy::prelude::{default, Query, Res};
use bevy_egui::egui::plot::{Bar, BarChart, Corner, Legend, Plot};
use bevy_egui::egui::{Color32, Window};
use bevy_egui::EguiContexts;
use std::collections::{BTreeMap, HashMap};

pub fn render_prices(
    mut egui_context: EguiContexts,
    sell_orders: Query<&SellOrder>,
    days: Res<Days>,
    counter: Res<Counter>,
) {
    Window::new("Prices").show(egui_context.ctx_mut(), |ui| {
        ui.label(format!("Days: {}", days.days));
        ui.label(format!("Count: {}", counter.0));

        let mut grouped_orders = BTreeMap::new();

        for sell_order in sell_orders.iter() {
            grouped_orders
                .entry(sell_order.item_type.clone())
                .or_insert_with(Vec::new)
                .push(sell_order.price);
        }

        for (item_type, prices) in grouped_orders {
            let len = prices.len();
            let mut prices = prices;
            prices.sort_unstable();

            let min = *prices.first().unwrap();
            let max = *prices.last().unwrap();
            let median = prices[len / 2];
            let p10 = prices[(len as f32 * 0.1).round() as usize];
            let p90 = prices[(len as f32 * 0.9).round() as usize];

            ui.label(format!(
                "ItemType: {}\nMin: {} p10: {} Median: {} p90: {} Max: {}, total: {}, avg. {}",
                item_type.name,
                min,
                p10,
                median,
                p90,
                max,
                len,
                prices.iter().sum::<u64>() / len as u64
            ));
            Plot::new(item_type.name.clone())
                .view_aspect(2.0)
                .legend(Legend {
                    position: Corner::LeftTop,
                    ..default()
                })
                .show(ui, |plot_ui| {
                    plot_ui.bar_chart(create_histogram(
                        format!("{} prices", item_type.name).as_str(),
                        &prices,
                        20,
                    ));
                });
        }
    });
}

pub fn create_histogram(name: &str, values: &[u64], bins: u32) -> BarChart {
    let mut histogram = HashMap::new();
    let max = values.iter().max().unwrap_or(&0);
    let min = values.iter().min().unwrap_or(&0);
    let range = max - min + 1;
    let bin_width = (range as f64 / bins as f64).ceil() as u64;
    for &value in values {
        *histogram.entry((value - min) / bin_width).or_insert(0) += 1;
    }
    let histogram: Vec<Bar> = histogram
        .into_iter()
        .map(|(bin, count)| {
            Bar::new((bin * bin_width + min) as f64, count as f64).width(bin_width as f64)
        })
        .collect();
    BarChart::new(histogram)
        .color(Color32::LIGHT_BLUE)
        .name(name)
}
