use crate::Days;
use bevy::prelude::{Commands, Res, *};

#[derive(Component)]
pub struct BusinessPermit {}

pub fn create_business_permit(
    mut commands: Commands,
    permits: Query<&BusinessPermit>,
    date: Res<Days>,
) {
    if permits.iter().count() == 0 && date.days % 10 == 1 {
        commands.spawn(BusinessPermit {});
    }
}
