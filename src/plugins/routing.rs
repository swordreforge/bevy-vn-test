use bevy::prelude::*;
use crate::resources::{GameFont, RouteConfig, SelectedRoute, UnlockState, ScreenTransition};
use crate::state::AppState;

pub struct RoutePlugin;

#[derive(Component)]
struct RouteScreen;

#[derive(Component)]
struct RouteButton(u32);

#[derive(Component)]
struct RouteBackButton;

impl Plugin for RoutePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::RouteSelection), setup_route_selection)
            .add_systems(Update, (
                handle_route_buttons,
                handle_back_button,
                handle_route_keyboard,
            ).run_if(in_state(AppState::RouteSelection)))
            .add_systems(OnExit(AppState::RouteSelection), cleanup_route_selection);
    }
}

const BTN_W: f32 = 160.0;
const BTN_H: f32 = 120.0;
const BTN_GAP: f32 = 20.0;

fn setup_route_selection(
    mut commands: Commands,
    game_font: Res<GameFont>,
    config: Res<RouteConfig>,
    engine: Res<crate::script::ScriptEngine>,
    unlock_state: Res<UnlockState>,
) {
    commands.spawn((
        RouteScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
        ZIndex(5),
    )).with_children(|root| {
        root.spawn((
            Text::new("Route Selection"),
            TextFont { font: game_font.0.clone(), font_size: 28.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(24.0)), ..default() },
        ));

        root.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(BTN_GAP),
                row_gap: Val::Px(BTN_GAP),
                max_width: Val::Px(3.0 * (BTN_W + BTN_GAP)),
                ..default()
            },
        )).with_children(|grid| {
            for entry in config.heroines_including_extra() {
                let unlocked = entry.always_unlocked
                    || unlock_state.is_route_cleared(&entry.name)
                    || engine.global_flags.get(&entry.unlock_flag).copied().unwrap_or(0) >= 1;
                let cleared = unlock_state.is_route_cleared(&entry.name);

                let status_text;
                let bg_color;
                if !unlocked {
                    status_text = "LOCKED";
                    bg_color = Color::srgba(0.2, 0.2, 0.25, 0.9);
                } else if cleared {
                    status_text = "CLEARED";
                    bg_color = Color::srgba(0.15, 0.3, 0.5, 0.9);
                } else {
                    status_text = "PLAY";
                    bg_color = Color::srgba(0.15, 0.5, 0.2, 0.9);
                }

                let mut entity = grid.spawn((
                    RouteButton(entry.index),
                    Node {
                        width: Val::Px(BTN_W),
                        height: Val::Px(BTN_H),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(bg_color),
                ));
                if unlocked {
                    entity.insert(Button);
                }
                entity.with_children(|btn| {
                    btn.spawn((
                        Text::new(&entry.name),
                        TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
                        TextColor(Color::srgb(0.9, 0.9, 0.95)),
                    ));
                    btn.spawn((
                        Text::new(status_text),
                        TextFont { font: game_font.0.clone(), font_size: 14.0, ..default() },
                        TextColor(if unlocked { Color::srgb(0.7, 1.0, 0.7) } else { Color::srgb(0.4, 0.4, 0.5) }),
                    ));
                });
            }
        });

        root.spawn((
            RouteBackButton,
            Button,
            Node {
                width: Val::Px(80.0),
                height: Val::Px(36.0),
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
        )).with_child((
            Text::new("← Back"),
            TextFont { font: game_font.0.clone(), font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn handle_route_buttons(
    query: Query<(&RouteButton, &Interaction), Changed<Interaction>>,
    config: Res<RouteConfig>,
    engine: Res<crate::script::ScriptEngine>,
    unlock_state: Res<UnlockState>,
    mut selected_route: ResMut<SelectedRoute>,
    mut screen_transition: ResMut<ScreenTransition>,
) {
    for (btn, interaction) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entry) = config.find_by_index(btn.0) else { continue };
        let unlocked = entry.always_unlocked
            || unlock_state.is_route_cleared(&entry.name)
            || engine.global_flags.get(&entry.unlock_flag).copied().unwrap_or(0) >= 1;
        if !unlocked { continue; }

        selected_route.0 = Some(entry.script.clone());
        screen_transition.pending_state = Some(AppState::Gameplay);
    }
}

fn handle_back_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RouteBackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
    dialogue: Res<crate::resources::DialogueState>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            let target = if dialogue.current_text.is_empty() {
                AppState::Title
            } else {
                AppState::Menu
            };
            next_state.set(target);
        }
    }
}

fn handle_route_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    dialogue: Res<crate::resources::DialogueState>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        let target = if dialogue.current_text.is_empty() {
            AppState::Title
        } else {
            AppState::Menu
        };
        next_state.set(target);
    }
}

fn cleanup_route_selection(
    mut commands: Commands,
    query: Query<Entity, With<RouteScreen>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
