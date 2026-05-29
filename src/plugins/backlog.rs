use crate::resources::{Backlog, GameFont};
use crate::state::AppState;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

#[derive(Component)]
struct BacklogRoot;

#[derive(Component)]
struct BacklogList;

#[derive(Component)]
struct BacklogCloseBtn;

pub struct BacklogPlugin;

impl Plugin for BacklogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Backlog>()
            .add_systems(OnEnter(AppState::Backlog), setup_backlog_ui)
            .add_systems(
                Update,
                handle_backlog_scroll.run_if(in_state(AppState::Backlog)),
            )
            .add_systems(
                Update,
                handle_backlog_close.run_if(in_state(AppState::Backlog)),
            )
            .add_systems(OnExit(AppState::Backlog), cleanup_backlog_ui);
    }
}

fn setup_backlog_ui(mut commands: Commands, backlog: Res<Backlog>, game_font: Res<GameFont>) {
    commands
        .spawn((
            BacklogRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            ZIndex(5),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("BACKLOG"),
                TextFont {
                    font: game_font.0.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
            ));

            parent
                .spawn((
                    BacklogList,
                    Node {
                        width: Val::Percent(90.0),
                        height: Val::Percent(70.0),
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::scroll_y(),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)),
                ))
                .with_children(|list_parent| {
                    for entry in backlog.entries.iter().rev() {
                        let speaker_str = entry.speaker.as_deref().unwrap_or("");
                        list_parent.spawn((
                            Text::new(speaker_str),
                            TextFont {
                                font: game_font.0.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.8, 0.6)),
                            Node {
                                margin: UiRect::top(Val::Px(8.0)),
                                ..default()
                            },
                        ));
                        list_parent.spawn((
                            Text::new(&entry.text),
                            TextFont {
                                font: game_font.0.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            Node {
                                width: Val::Percent(100.0),
                                margin: UiRect::left(Val::Px(12.0)),
                                ..default()
                            },
                        ));
                    }
                });

            parent.spawn((
                BacklogCloseBtn,
                Button,
                Text::new("← Back"),
                TextFont {
                    font: game_font.0.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
            ));
        });
}

fn handle_backlog_scroll(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut query: Query<&mut ScrollPosition, With<BacklogList>>,
) {
    let Ok(mut scroll) = query.single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::ArrowUp) {
        scroll.0.y = (scroll.0.y + 40.0).min(0.0);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        scroll.0.y -= 40.0;
    }

    for ev in mouse_wheel.read() {
        scroll.0.y += ev.y * 30.0;
    }
}

fn handle_backlog_close(
    keyboard: Res<ButtonInput<KeyCode>>,
    btn_query: Query<&Interaction, (Changed<Interaction>, With<BacklogCloseBtn>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Gameplay);
        return;
    }
    for interaction in btn_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Gameplay);
        }
    }
}

fn cleanup_backlog_ui(mut commands: Commands, query: Query<Entity, With<BacklogRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
