use bevy::prelude::*;
use std::collections::VecDeque;

use crate::ui::main_layout::UiState;
use crate::Days;

#[derive(Component)]
pub struct Pinned {}

#[derive(Event, Clone)]
pub struct LogEvent {
    pub text: String,
    pub entity: Entity,
}

pub struct LogEntry {
    pub entry: LogEvent,
    pub day: u32,
}

#[derive(Resource, Default)]
pub struct Logs {
    pub entries: VecDeque<LogEntry>,
}

pub fn logging_system(
    mut new_logs: EventReader<LogEvent>,
    mut logs: ResMut<Logs>,
    days: Res<Days>,
) {
    for log in new_logs.iter() {
        logs.entries.push_front(LogEntry {
            entry: log.clone(),
            day: days.days as u32,
        });
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
    pins.get(log.entry.entity).is_ok() && ui_state.logs_keep_pinned
}
