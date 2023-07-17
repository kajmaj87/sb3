use bevy::prelude::{Event, EventReader, Res, ResMut, Time};

use crate::config::Config;
use crate::Days;

const BASE_SECONDS_PER_DAY: f32 = 1.0;

#[derive(Event)]
pub enum GameCommand {
    SetSpeed(f32),
    AdvanceDay,
}
pub fn command_system(
    mut game_commands: EventReader<GameCommand>,
    mut days: ResMut<Days>,
    time: Res<Time>,
    mut config: ResMut<Config>,
) {
    for command in game_commands.iter() {
        match command {
            GameCommand::SetSpeed(value) => {
                config.game.speed.value = BASE_SECONDS_PER_DAY / value;
            }
            GameCommand::AdvanceDay => {
                if config.game.speed.value != 0.0 {
                    config.game.speed.value = 0.0;
                } else {
                    days.next_day(&time);
                }
            }
        }
    }
}
