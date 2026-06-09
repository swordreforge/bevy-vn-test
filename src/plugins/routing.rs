use bevy::prelude::*;
use crate::resources::{GameFont, RouteConfig, RouteEntry, SelectedRoute, UnlockState, ScreenTransition};
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

const CARD_W: f32 = 400.0;
const CARD_H: f32 = 56.0;
const ROW_GAP: f32 = 6.0;
const LINE_X: f32 = 19.0;
const DOT_SIZE: f32 = 10.0;
const CONN_W: f32 = 40.0;

fn setup_route_selection(
    mut commands: Commands,
    game_font: Res<GameFont>,
    config: Res<RouteConfig>,
    engine: Res<crate::script::ScriptEngine>,
    unlock_state: Res<UnlockState>,
) {
    let entry_count = config.heroines.len();
    let map_h = entry_count as f32 * CARD_H + (entry_count as f32 - 1.0) * ROW_GAP;

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
            Text::new("Route Map"),
            TextFont { font: game_font.0.clone(), font_size: 28.0, ..default() },
            TextColor(Color::srgb(0.9, 0.9, 0.95)),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));

        root.spawn((
            Node {
                width: Val::Px(CARD_W + CONN_W),
                height: Val::Px(map_h),
                ..default()
            },
        )).with_children(|stack| {
            stack.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(LINE_X),
                    top: Val::Px(0.0),
                    width: Val::Px(2.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.35, 0.35, 0.5, 0.3)),
            ));

            stack.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(ROW_GAP),
                    padding: UiRect::left(Val::Px(CONN_W)),
                    ..default()
                },
            )).with_children(|cards_col| {
                for entry in &config.heroines {
                    let (status_text, unlocked, cleared) = entry_status(entry, &engine, &unlock_state);
                    let accent = heroine_color(entry.index);
                    let bg = card_bg(unlocked, cleared);

                    let mut row = cards_col.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(CARD_H),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ));

                    row.with_children(|r| {
                        r.spawn((
                            Node {
                                width: Val::Px(CONN_W),
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                        )).with_children(|dot_area| {
                            dot_area.spawn((
                                Node {
                                    width: Val::Px(DOT_SIZE),
                                    height: Val::Px(DOT_SIZE),
                                    ..default()
                                },
                                BackgroundColor(accent),
                            ));
                        });

                        let ch_label = chapter_label(entry.index);
                        let mut card = r.spawn((
                            RouteButton(entry.index),
                            Node {
                                width: Val::Px(CARD_W),
                                height: Val::Px(CARD_H),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(Val::Px(10.0)),
                                ..default()
                            },
                            BackgroundColor(bg),
                        ));
                        if unlocked {
                            card.insert(Button);
                        }

                        card.with_children(|c| {
                            c.spawn((
                                Node {
                                    width: Val::Px(4.0),
                                    height: Val::Percent(100.0),
                                    margin: UiRect::right(Val::Px(10.0)),
                                    ..default()
                                },
                                BackgroundColor(accent),
                            ));

                            c.spawn((
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    flex_grow: 1.0,
                                    ..default()
                                },
                            )).with_children(|info| {
                                info.spawn((
                                    Text::new(&entry.name),
                                    TextFont { font: game_font.0.clone(), font_size: 18.0, ..default() },
                                    TextColor(Color::srgb(0.9, 0.9, 0.95)),
                                ));
                                info.spawn((
                                    Text::new(ch_label),
                                    TextFont { font: game_font.0.clone(), font_size: 12.0, ..default() },
                                    TextColor(Color::srgb(0.6, 0.6, 0.7)),
                                ));
                            });

                            c.spawn((
                                Text::new(status_text),
                                TextFont { font: game_font.0.clone(), font_size: 14.0, ..default() },
                                TextColor(status_text_color(unlocked, cleared)),
                            ));
                        });
                    });
                }
            });
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

fn entry_status<'a>(entry: &RouteEntry, engine: &crate::script::ScriptEngine, unlock_state: &UnlockState) -> (&'a str, bool, bool) {
    let unlocked = entry.always_unlocked
        || unlock_state.is_route_cleared(&entry.name)
        || engine.global_flags.get(&entry.unlock_flag).copied().unwrap_or(0) >= 1;
    let cleared = unlock_state.is_route_cleared(&entry.name);
    if !unlocked {
        ("LOCKED", false, false)
    } else if cleared {
        ("CLEARED", true, true)
    } else {
        ("PLAY", true, false)
    }
}

fn chapter_label(index: u32) -> &'static str {
    match index {
        1 => "Chapter 1",
        2 => "Chapter 2",
        3 | 5 => "Chapter 3",
        4 => "Chapter 4",
        6 => "Chapter 5",
        _ => "",
    }
}

fn heroine_color(index: u32) -> Color {
    match index {
        1 => Color::srgba(1.0, 0.3, 0.3, 1.0),
        2 => Color::srgba(0.3, 0.6, 1.0, 1.0),
        3 => Color::srgba(0.3, 1.0, 0.4, 1.0),
        4 => Color::srgba(0.7, 0.3, 1.0, 1.0),
        5 => Color::srgba(1.0, 0.8, 0.2, 1.0),
        6 => Color::srgba(0.3, 1.0, 0.9, 1.0),
        _ => Color::srgba(0.5, 0.5, 0.5, 1.0),
    }
}

fn card_bg(unlocked: bool, cleared: bool) -> Color {
    if !unlocked {
        Color::srgba(0.2, 0.2, 0.25, 0.9)
    } else if cleared {
        Color::srgba(0.15, 0.3, 0.5, 0.9)
    } else {
        Color::srgba(0.15, 0.5, 0.2, 0.9)
    }
}

fn status_text_color(unlocked: bool, cleared: bool) -> Color {
    if !unlocked {
        Color::srgb(0.4, 0.4, 0.5)
    } else if cleared {
        Color::srgb(0.4, 0.7, 1.0)
    } else {
        Color::srgb(0.3, 1.0, 0.4)
    }
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

        selected_route.start_script = Some(entry.script.clone());
        selected_route.route_name = Some(entry.name.clone());
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
