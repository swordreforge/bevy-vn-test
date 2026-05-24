use bevy::prelude::*;
use crate::resources::Settings;
use crate::state::AppState;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Settings>()
            .add_systems(OnEnter(AppState::Settings), setup_settings_ui)
            .add_systems(OnExit(AppState::Settings), cleanup_settings);
    }
}

#[derive(Component)]
struct SettingsScreen;

fn setup_settings_ui(mut commands: Commands) {
    commands.spawn((
        SettingsScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.2, 0.95)),
    )).with_child((
        SettingsScreen,
        Text::new("Settings"),
        TextFont { font_size: 36.0, ..default() },
        TextColor(Color::WHITE),
    )).with_child((
        SettingsScreen,
        Text::new("Click or touch to return"),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::srgb(0.5, 0.5, 0.5)),
    ));
}

fn cleanup_settings(mut commands: Commands, query: Query<Entity, With<SettingsScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
