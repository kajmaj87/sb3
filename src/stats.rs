use std::collections::{BTreeMap, HashMap};
use std::fmt;

use bevy::prelude::{debug, Query, Res, ResMut, Resource};

use crate::business::{ItemType, SellOrder};
use crate::money::Money;
use crate::Days;

#[derive(Debug)]
pub struct PriceStats {
    pub item_type: ItemType,
    pub min: Money,
    pub max: Money,
    pub median: Money,
    pub p25: Money,
    pub p75: Money,
    pub avg: Money,
    pub total_orders: usize,
    pub day: usize,
}

impl fmt::Display for PriceStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Price for {}: {}\n", self.item_type.name, self.avg)?;
        writeln!(f, "📉  MIN Price: {}", self.min)?;
        writeln!(f, "🔳  25th Percentile Price: {}", self.p25)?;
        writeln!(f, "🎯  MEDIAN Price: {}", self.median)?;
        writeln!(f, "🔳  75th Percentile Price: {}", self.p75)?;
        writeln!(f, "📈  MAX Price: {}", self.max)?;
        writeln!(f, "🔵  AVERAGE Price: {}", self.avg)?;
        writeln!(f, "📊  Total Orders: {}", self.total_orders)?;
        write!(f, "🗓  Day: {}", self.day)
    }
}

#[derive(Resource, Default)]
pub struct PriceHistory {
    pub prices: HashMap<ItemType, Vec<PriceStats>>,
}

pub fn add_sell_orders_to_history(
    mut history: ResMut<PriceHistory>,
    days: Res<Days>,
    sell_orders: Query<&SellOrder>,
) {
    let mut grouped_orders = BTreeMap::new();
    debug!("Adding sell orders to history");

    for sell_order in sell_orders.iter() {
        grouped_orders
            .entry(sell_order.item_type.clone())
            .or_insert_with(Vec::new)
            .push(sell_order);
    }
    for (item_type, sell_order) in grouped_orders.iter() {
        let mut prices = sell_order.iter().map(|o| o.price).collect::<Vec<_>>();
        prices.sort_unstable();

        let min = *prices.first().unwrap();
        let max = *prices.last().unwrap();
        let median = prices[prices.len() / 2];
        let p25 = prices[(prices.len() as f32 * 0.25).floor() as usize];
        let p75 = prices[(prices.len() as f32 * 0.75).floor() as usize];
        let len = prices.len();
        let avg = prices.iter().sum::<Money>() / len;

        let stats = PriceStats {
            item_type: item_type.clone(),
            day: days.days,
            min,
            max,
            median,
            p25,
            p75,
            avg,
            total_orders: len,
        };
        history
            .prices
            .entry(item_type.clone())
            .or_insert_with(Vec::new)
            .push(stats);
    }
}
