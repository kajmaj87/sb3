use crate::commands::GameCommand;
use bevy::{input::Input, prelude::*};

pub fn input_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut game_commands: EventWriter<GameCommand>,
) {
    if keyboard_input.pressed(KeyCode::Grave) {
        game_commands.send(GameCommand::SetSpeed(0.0));
    }
    if keyboard_input.pressed(KeyCode::Key1) {
        game_commands.send(GameCommand::SetSpeed(1.0));
    }
    if keyboard_input.pressed(KeyCode::Key2) {
        game_commands.send(GameCommand::SetSpeed(2.0));
    }
    if keyboard_input.pressed(KeyCode::Key3) {
        game_commands.send(GameCommand::SetSpeed(4.0));
    }
    if keyboard_input.pressed(KeyCode::Key4) {
        game_commands.send(GameCommand::SetSpeed(8.0));
    }
    if keyboard_input.pressed(KeyCode::Key5) {
        game_commands.send(GameCommand::SetSpeed(16.0));
    }
    if keyboard_input.pressed(KeyCode::Key6) {
        game_commands.send(GameCommand::SetSpeed(32.0));
    }
    if keyboard_input.just_pressed(KeyCode::Return) {
        game_commands.send(GameCommand::AdvanceDay);
    }
}
