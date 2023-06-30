use crate::business::{ItemType, SellOrder};
use crate::money::Money;
use crate::Days;
use bevy::prelude::{debug, Query, Res, ResMut, Resource};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
pub struct PriceStats {
    pub item_type: ItemType,
    pub min: u64,
    pub max: u64,
    pub median: u64,
    pub p25: u64,
    pub p75: u64,
    pub avg: u64,
    pub total_orders: usize,
    pub day: usize,
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

        let min = prices.first().unwrap().as_u64();
        let max = prices.last().unwrap().as_u64();
        let median = prices[prices.len() / 2].as_u64();
        let p25 = prices[(prices.len() as f32 * 0.25).floor() as usize].as_u64();
        let p75 = prices[(prices.len() as f32 * 0.75).floor() as usize].as_u64();
        let len = prices.len();
        let avg = (prices.iter().sum::<Money>() / len).as_u64();

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
