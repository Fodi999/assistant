//! Allowed entity actions per severity.

use crate::domain::scene::{EntityAction, MaterialTheme};

pub fn actions_for_theme(theme: MaterialTheme) -> Vec<EntityAction> {
    match theme {
        MaterialTheme::Expired | MaterialTheme::Critical => {
            vec![EntityAction::WriteOff, EntityAction::OpenDetails]
        }
        MaterialTheme::Warning => vec![
            EntityAction::UseToday,
            EntityAction::WriteOff,
            EntityAction::OpenDetails,
        ],
        _ => vec![
            EntityAction::UseToday,
            EntityAction::OpenDetails,
            EntityAction::WriteOff,
        ],
    }
}
