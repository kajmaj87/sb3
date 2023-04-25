use bevy::{input::Input, prelude::*};

use crate::config::Config;
use crate::Days;

const BASE_SECONDS_PER_DAY: f32 = 1.0;

pub fn input_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut days: ResMut<Days>,
    time: Res<Time>,
    mut config: ResMut<Config>,
) {
    if keyboard_input.pressed(KeyCode::Grave) {
        config.game.speed.value = 0.0;
    }
    if keyboard_input.pressed(KeyCode::Key1) {
        config.game.speed.value = BASE_SECONDS_PER_DAY;
    }
    if keyboard_input.pressed(KeyCode::Key2) {
        config.game.speed.value = BASE_SECONDS_PER_DAY / 2.0;
    }
    if keyboard_input.pressed(KeyCode::Key3) {
        config.game.speed.value = BASE_SECONDS_PER_DAY / 4.0;
    }
    if keyboard_input.pressed(KeyCode::Key4) {
        config.game.speed.value = BASE_SECONDS_PER_DAY / 8.0;
    }
    if keyboard_input.pressed(KeyCode::Key5) {
        config.game.speed.value = BASE_SECONDS_PER_DAY / 16.0;
    }
    if keyboard_input.pressed(KeyCode::Key6) {
        config.game.speed.value = BASE_SECONDS_PER_DAY / 32.0;
    }
    if keyboard_input.just_pressed(KeyCode::Return) && config.game.speed.value == 0.0 {
        days.next_day(time);
    }
}
