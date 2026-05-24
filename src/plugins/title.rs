use bevy::prelude::*;
use crate::state::AppState;

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Title), setup_title)
            .add_systems(Update, title_click.run_if(in_state(AppState::Title)))
            .add_systems(OnExit(AppState::Title), cleanup_title);
    }
}

#[derive(Component)]
struct TitleScreen;

fn setup_title(mut commands: Commands) {
    commands.spawn((
        TitleScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::BLACK),
    )).with_child((
        TitleScreen,
        Text::new("Visual Novel Engine"),
        TextFont {
            font_size: 48.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn title_click(
    mut next_state: ResMut<NextState<AppState>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
) {
    if mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed() {
        next_state.set(AppState::Gameplay);
    }
}

fn cleanup_title(mut commands: Commands, query: Query<Entity, With<TitleScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
