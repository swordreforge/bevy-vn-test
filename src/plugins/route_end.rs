use bevy::prelude::*;
use crate::resources::{CompletedRoute, GameFont, RouteConfig};
use crate::state::AppState;

pub struct RouteEndPlugin;

#[derive(Component)]
struct RouteEndScreen;

#[derive(Component)]
struct RouteEndTitleBtn;

#[derive(Component)]
struct RouteEndSelectionBtn;

impl Plugin for RouteEndPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CompletedRoute>()
            .add_systems(OnEnter(AppState::RouteEnd), setup_route_end)
            .add_systems(Update, (
                handle_title_button,
                handle_selection_button,
                handle_route_end_keyboard,
            ).run_if(in_state(AppState::RouteEnd)))
            .add_systems(OnExit(AppState::RouteEnd), cleanup_route_end);
    }
}

fn setup_route_end(
    mut commands: Commands,
    game_font: Res<GameFont>,
    config: Res<RouteConfig>,
    completed: Res<CompletedRoute>,
) {
    let route_name = completed.0.as_deref()
        .and_then(|s| config.find_by_script(s))
        .map(|e| e.name.as_str())
        .unwrap_or("");

    commands.spawn((
        RouteEndScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(20.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
        ZIndex(5),
    )).with_children(|root| {
        root.spawn((
            Text::new("Route Complete!"),
            TextFont { font: game_font.0.clone(), font_size: 36.0, ..default() },
            TextColor(Color::srgb(0.8, 0.6, 0.2)),
        ));

        if !route_name.is_empty() {
            root.spawn((
                Text::new(route_name),
                TextFont { font: game_font.0.clone(), font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.8)),
            ));
        }

        root.spawn((
            RouteEndSelectionBtn,
            Button,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
        )).with_child((
            Text::new("Route Selection"),
            TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
            TextColor(Color::WHITE),
        ));

        root.spawn((
            RouteEndTitleBtn,
            Button,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
        )).with_child((
            Text::new("Back to Title"),
            TextFont { font: game_font.0.clone(), font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn handle_title_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RouteEndTitleBtn>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Title);
        }
    }
}

fn handle_selection_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RouteEndSelectionBtn>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::RouteSelection);
        }
    }
}

fn handle_route_end_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Title);
    }
}

fn cleanup_route_end(
    mut commands: Commands,
    query: Query<Entity, With<RouteEndScreen>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
