use bevy::prelude::*;

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub enum AppState {
    #[default]
    Boot,
    Title,
    Gameplay,
    Menu,
    SaveLoad,
    Gallery,
    Settings,
}
