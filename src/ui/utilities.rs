use std::collections::HashMap;

use bevy::prelude::Entity;
use bevy_egui::egui::Ui;

use crate::business::ItemType;

pub(crate) fn label_with_hover_text(ui: &mut Ui, amount: usize, hover_text: &str) {
    let label = ui.label(amount.to_string());
    if amount > 0 {
        label.on_hover_text(hover_text);
    }
}

pub(crate) fn count_items(items: &HashMap<ItemType, Vec<Entity>>) -> usize {
    items.values().map(|x| x.len()).sum()
}

pub(crate) fn items_to_string(items: &HashMap<ItemType, Vec<Entity>>) -> String {
    items
        .iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(item_type, items)| format!("{}: {}", item_type.name, items.len()))
        .collect::<Vec<_>>()
        .join("\n")
}
