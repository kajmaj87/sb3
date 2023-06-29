use crate::business::SellOrder;
use crate::{Counter, Days};
use bevy::prelude::{Query, Res};
use bevy_egui::egui::plot::{BoxElem, BoxPlot, BoxSpread, Legend, Plot};
use bevy_egui::egui::{Color32, Window};
use bevy_egui::EguiContexts;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
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

        let mut i = 0;
        let mut box_plots = vec![];
        for (item_type, prices) in grouped_orders {
            let len = prices.len();
            i += 1;
            let mut prices = prices;
            prices.sort_unstable();

            let min = *prices.first().unwrap();
            let max = *prices.last().unwrap();
            let median = prices[len / 2];
            let p25 = prices[(len as f32 * 0.25).round() as usize];
            let p75 = prices[(len as f32 * 0.75).round() as usize];
            let len = prices.len();
            let avg = prices.iter().sum::<u64>() / len as u64;

            box_plots.push(
                BoxPlot::new(vec![BoxElem::new(
                    i as f64,
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
                .color(string_to_rgb(item_type.name.as_str())),
            );
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
