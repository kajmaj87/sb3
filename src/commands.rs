use crate::config::Config;
use crate::Days;
use bevy::prelude::{EventReader, Res, ResMut, Time};

const BASE_SECONDS_PER_DAY: f32 = 1.0;

pub enum GameCommand {
    SetSpeed(f32),
    AdvanceDay,
    // ... add more commands here as needed
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
                if config.game.speed.value == 0.0 {
                    days.next_day(&time);
                }
            }
        }
    }
}
