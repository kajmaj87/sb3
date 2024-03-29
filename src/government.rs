use crate::config::Config;
use crate::Days;
use bevy::prelude::{Commands, Res, *};

#[derive(Component)]
pub struct BusinessPermit {}

#[derive(Component, Default)]
pub struct Government {}

#[derive(Component)]
pub struct TaxAuthority {
    pub authority: Entity,
}

pub fn create_business_permit(
    mut commands: Commands,
    permits: Query<&BusinessPermit>,
    date: Res<Days>,
    config: Res<Config>,
) {
    if permits.iter().count() == 0
        && date.days % config.government.min_time_between_business_creation.value == 1
    {
        commands.spawn(BusinessPermit {});
    }
}
