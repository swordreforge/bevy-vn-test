use bevy::prelude::*;
use crate::state::AppState;
use crate::resources::UnlockState;

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnlockState>()
            .add_systems(OnEnter(AppState::Gallery), setup_gallery)
            .add_systems(OnExit(AppState::Gallery), cleanup_gallery);
    }
}

#[derive(Component)]
struct GalleryScreen;

fn setup_gallery(mut commands: Commands) {
    commands.spawn((
        GalleryScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
    )).with_child((
        GalleryScreen,
        Text::new("CG Gallery"),
        TextFont { font_size: 36.0, ..default() },
        TextColor(Color::WHITE),
    )).with_child((
        GalleryScreen,
        Text::new("Coming Soon"),
        TextFont { font_size: 24.0, ..default() },
        TextColor(Color::srgb(0.5, 0.5, 0.5)),
    ));
}

fn cleanup_gallery(mut commands: Commands, query: Query<Entity, With<GalleryScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
