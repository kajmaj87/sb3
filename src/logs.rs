use bevy::prelude::*;
use std::collections::VecDeque;

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
