use bevy::prelude::*;
use crate::resources::AffectionMap;

pub struct AffectionPlugin;

impl Plugin for AffectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AffectionMap>();
    }
}
