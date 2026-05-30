use bevy::prelude::*;

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub enum AppState {
    #[default]
    Boot,
    Splash,
    Title,
    Gameplay,
    RouteSelection,
    Menu,
    SaveLoad,
    Gallery,
    Settings,
    Backlog,
}
