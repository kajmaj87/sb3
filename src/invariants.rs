use crate::business::{Manufacturer, Worker};
use bevy::prelude::*;

pub fn each_hired_worker_should_have_correct_employer(
    manufacturers: Query<(Entity, &Manufacturer)>,
    workers: Query<&Worker>,
    names: Query<&Name>,
) {
    for (employer, manufacturer) in manufacturers.iter() {
        for worker_entity in manufacturer.hired_workers.iter() {
            if let Ok(worker) = workers.get(*worker_entity) {
                let worker_name = names.get(*worker_entity).unwrap();
                worker
                    .employed_at
                    .unwrap_or_else(|| panic!("Worker {} should have an employer", worker_name));
                let worker_employer = worker.employed_at.unwrap();
                let worker_employer_name = names.get(worker_employer).unwrap();
                let employer_name = names.get(employer).unwrap();
                assert_eq!(
                    worker.employed_at,
                    Some(employer),
                    "{} should be employed at {} but believes is employed at {}",
                    worker_name,
                    employer_name,
                    worker_employer_name
                );
            }
        }
    }
}
