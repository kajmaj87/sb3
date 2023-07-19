use crate::business::ItemType;
use bevy::prelude::*;
use std::collections::VecDeque;
use std::fmt::Display;
use std::fmt::Formatter;

use crate::money::Money;
use crate::ui::main_layout::UiState;
use crate::Days;

#[derive(Component)]
pub struct Pinned {}

#[derive(Event, Clone)]
pub enum LogEvent {
    Generic {
        text: String,
        entity: Entity,
    },
    Trade {
        buyer: Entity,
        seller: Entity,
        item_type: ItemType,
        price: Money,
    },
    Salary {
        employer: Entity,
        worker: Entity,
        salary: Money,
    },
}

pub struct LogEntry {
    pub text: String,
    pub entity: Entity,
    pub name: Option<String>,
    pub day: u32,
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "Day {}: {}: {}", self.day, name, self.text)
        } else {
            write!(f, "Day {}: {}", self.day, self.text)
        }
    }
}

#[derive(Resource, Default)]
pub struct Logs {
    pub entries: VecDeque<LogEntry>,
}

pub fn logging_system(
    mut new_logs: EventReader<LogEvent>,
    mut logs: ResMut<Logs>,
    names: Query<&Name>,
    days: Res<Days>,
) {
    for log in new_logs.iter() {
        match log {
            LogEvent::Generic { text, entity } => {
                logs.entries.push_front(LogEntry {
                    entity: *entity,
                    text: text.clone(),
                    name: names.get(*entity).ok().map(|n| n.to_string()),
                    day: days.days as u32,
                });
            }
            LogEvent::Trade {
                buyer,
                seller,
                item_type,
                price,
            } => {
                let buyer_name = names.get(*buyer).ok().map(|n| n.to_string());
                let seller_name = names.get(*seller).ok().map(|n| n.to_string());
                logs.entries.push_front(LogEntry {
                    entity: *buyer,
                    text: format!(
                        "I bought {} for {} from {}",
                        item_type,
                        price,
                        seller_name.clone().unwrap_or("UNKNOWN".to_string())
                    ),
                    name: buyer_name.clone(),
                    day: days.days as u32,
                });
                logs.entries.push_front(LogEntry {
                    entity: *seller,
                    text: format!(
                        "I sold {} for {} to {}",
                        item_type,
                        price,
                        buyer_name.unwrap_or("UNKNOWN".to_string())
                    ),
                    name: seller_name,
                    day: days.days as u32,
                });
            }
            LogEvent::Salary {
                employer,
                worker,
                salary,
            } => {
                let employer_name = names.get(*employer).ok().map(|n| n.to_string());
                let worker_name = names.get(*worker).ok().map(|n| n.to_string());
                logs.entries.push_front(LogEntry {
                    entity: *employer,
                    text: format!(
                        "I paid {} to {}",
                        salary,
                        worker_name.clone().unwrap_or("UNKNOWN".to_string())
                    ),
                    name: employer_name.clone(),
                    day: days.days as u32,
                });
                logs.entries.push_front(LogEntry {
                    entity: *worker,
                    text: format!(
                        "I received {} from {}",
                        salary,
                        employer_name.unwrap_or("UNKNOWN".to_string())
                    ),
                    name: worker_name,
                    day: days.days as u32,
                });
            }
        }
    }
}

pub fn delete_old_logs_system(
    mut logs: ResMut<Logs>,
    days: Res<Days>,
    pins: Query<&Pinned>,
    ui_state: Res<UiState>,
) {
    let day = days.days as u32;
    if ui_state.logs_delete_unpinned_old {
        logs.entries.retain(|log| {
            keep_pinned(log, &ui_state, &pins) || is_still_young(log, day, &ui_state)
        });
    }
}

fn is_still_young(log: &LogEntry, day: u32, ui_state: &UiState) -> bool {
    day - log.day < ui_state.logs_delete_unpinned_older_than
}

fn keep_pinned(log: &LogEntry, ui_state: &UiState, pins: &Query<&Pinned>) -> bool {
    pins.get(log.entity).is_ok() && ui_state.logs_keep_pinned
}
