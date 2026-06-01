use bevy::prelude::*;
use crate::resources::{AfterStoryEntry, AfterStoryGroup, GameFont, RouteConfig, SelectedRoute};
use crate::state::AppState;

pub struct AfterStoryPlugin;

#[derive(Component)]
struct AfterStoryRoot;

#[derive(Component)]
struct AfterStoryGroupButton(usize);

#[derive(Component)]
struct AfterStoryEntryButton(usize);

#[derive(Component)]
struct AfterStoryBackBtn;

impl Plugin for AfterStoryPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::AfterStory), setup_after_story)
            .add_systems(Update, (
                handle_group_buttons,
                handle_entry_buttons,
                handle_back_button,
                handle_keyboard,
            ).run_if(in_state(AppState::AfterStory)))
            .add_systems(OnExit(AppState::AfterStory), cleanup_after_story);
    }
}

fn setup_after_story(
    mut commands: Commands,
    game_font: Res<GameFont>,
    config: Res<RouteConfig>,
    engine: Res<crate::script::ScriptEngine>,
    group: Res<AfterStoryGroup>,
) {
    build_ui(&mut commands, &game_font.0, &config, &engine, group.0);
}

fn build_ui(
    commands: &mut Commands,
    font: &Handle<Font>,
    config: &RouteConfig,
    engine: &crate::script::ScriptEngine,
    group_idx: Option<usize>,
) {
    let is_level_2 = group_idx.is_some();
    let title_text = if is_level_2 { "After Story" } else { "After Stories" };

    commands.spawn((
        AfterStoryRoot,
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
            Text::new(title_text),
            TextFont { font: font.clone(), font_size: 28.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(24.0)), ..default() },
        ));

            if let Some(idx) = group_idx {
            let entries: Vec<&AfterStoryEntry> = if idx < config.heroines.len() {
                config.heroines[idx].after_stories.iter().collect()
            } else if idx == config.heroines.len() {
                config.extra_after_stories.iter().collect()
            } else {
                config.bonus_skits.iter().collect()
            };

            let unlocked = if idx < config.heroines.len() {
                let entry = &config.heroines[idx];
                entry.always_unlocked
                    || engine.global_flags.get(&entry.unlock_flag).copied().unwrap_or(0) >= 1
            } else if idx == config.heroines.len() {
                engine.global_flags.get(&config.all_routes_cleared_flag).copied().unwrap_or(0) >= 1
            } else {
                engine.global_flags.get(&config.full_completion_flag).copied().unwrap_or(0) >= 1
            };

            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                ..default()
            }).with_children(|list| {
                for (i, after) in entries.iter().enumerate() {
                    let bg = if unlocked {
                        Color::srgba(0.15, 0.4, 0.3, 0.9)
                    } else {
                        Color::srgba(0.2, 0.2, 0.25, 0.9)
                    };
                    let mut entity = list.spawn((
                        AfterStoryEntryButton(i),
                        Node {
                            width: Val::Px(280.0),
                            height: Val::Px(40.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(bg),
                    ));
                    if unlocked {
                        entity.insert(Button);
                    }
                    entity.with_children(|btn| {
                        btn.spawn((
                            Text::new(&after.name),
                            TextFont { font: font.clone(), font_size: 18.0, ..default() },
                            TextColor(Color::srgb(0.9, 0.9, 0.95)),
                        ));
                    });
                }
            });
        } else {
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            }).with_children(|list| {
                for (i, entry) in config.heroines.iter().enumerate() {
                    if entry.after_stories.is_empty() {
                        continue;
                    }
                    let unlocked = entry.always_unlocked
                        || engine.global_flags.get(&entry.unlock_flag).copied().unwrap_or(0) >= 1;
                    let status = if unlocked { "PLAY" } else { "LOCKED" };
                    let bg = if unlocked {
                        Color::srgba(0.15, 0.5, 0.2, 0.9)
                    } else {
                        Color::srgba(0.2, 0.2, 0.25, 0.9)
                    };

                    let mut entity = list.spawn((
                        AfterStoryGroupButton(i),
                        Node {
                            width: Val::Px(280.0),
                            height: Val::Px(48.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(16.0)),
                            ..default()
                        },
                        BackgroundColor(bg),
                    ));
                    if unlocked {
                        entity.insert(Button);
                    }
                    entity.with_children(|btn| {
                        btn.spawn((
                            Text::new(&entry.name),
                            TextFont { font: font.clone(), font_size: 20.0, ..default() },
                            TextColor(Color::srgb(0.9, 0.9, 0.95)),
                        ));
                        btn.spawn((
                            Text::new(status),
                            TextFont { font: font.clone(), font_size: 16.0, ..default() },
                            TextColor(if unlocked { Color::srgb(0.7, 1.0, 0.7) } else { Color::srgb(0.4, 0.4, 0.5) }),
                            Node::default(),
                        ));
                    });
                }

                if !config.extra_after_stories.is_empty() {
                    let unlocked = engine.global_flags.get(&config.all_routes_cleared_flag).copied().unwrap_or(0) >= 1;
                    let status = if unlocked { "PLAY" } else { "LOCKED" };
                    let bg = if unlocked {
                        Color::srgba(0.15, 0.3, 0.5, 0.9)
                    } else {
                        Color::srgba(0.2, 0.2, 0.25, 0.9)
                    };
                    let extra_idx = config.heroines.len();

                    let mut entity = list.spawn((
                        AfterStoryGroupButton(extra_idx),
                        Node {
                            width: Val::Px(280.0),
                            height: Val::Px(48.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(16.0)),
                            ..default()
                        },
                        BackgroundColor(bg),
                    ));
                    if unlocked {
                        entity.insert(Button);
                    }
                    entity.with_children(|btn| {
                        btn.spawn((
                            Text::new("终章"),
                            TextFont { font: font.clone(), font_size: 20.0, ..default() },
                            TextColor(Color::srgb(0.9, 0.9, 0.95)),
                        ));
                        btn.spawn((
                            Text::new(status),
                            TextFont { font: font.clone(), font_size: 16.0, ..default() },
                            TextColor(if unlocked { Color::srgb(0.7, 1.0, 0.7) } else { Color::srgb(0.4, 0.4, 0.5) }),
                            Node::default(),
                        ));
                    });
                }

                if !config.bonus_skits.is_empty() {
                    let unlocked = engine.global_flags.get(&config.full_completion_flag).copied().unwrap_or(0) >= 1;
                    let status = if unlocked { "PLAY" } else { "LOCKED" };
                    let bg = if unlocked {
                        Color::srgba(0.5, 0.2, 0.4, 0.9)
                    } else {
                        Color::srgba(0.2, 0.2, 0.25, 0.9)
                    };
                    let bonus_idx = config.heroines.len() + 1;

                    let mut entity = list.spawn((
                        AfterStoryGroupButton(bonus_idx),
                        Node {
                            width: Val::Px(280.0),
                            height: Val::Px(48.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(16.0)),
                            ..default()
                        },
                        BackgroundColor(bg),
                    ));
                    if unlocked {
                        entity.insert(Button);
                    }
                    entity.with_children(|btn| {
                        btn.spawn((
                            Text::new("小劇場"),
                            TextFont { font: font.clone(), font_size: 20.0, ..default() },
                            TextColor(Color::srgb(0.9, 0.9, 0.95)),
                        ));
                        btn.spawn((
                            Text::new(status),
                            TextFont { font: font.clone(), font_size: 16.0, ..default() },
                            TextColor(if unlocked { Color::srgb(0.7, 1.0, 0.7) } else { Color::srgb(0.4, 0.4, 0.5) }),
                            Node::default(),
                        ));
                    });
                }
            });
        }

        root.spawn((
            AfterStoryBackBtn,
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
            TextFont { font: font.clone(), font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn handle_group_buttons(
    mut commands: Commands,
    query: Query<(&AfterStoryGroupButton, &Interaction), Changed<Interaction>>,
    mut group: ResMut<AfterStoryGroup>,
    engine: Res<crate::script::ScriptEngine>,
    config: Res<RouteConfig>,
    root_query: Query<Entity, With<AfterStoryRoot>>,
    game_font: Res<GameFont>,
) {
    for (btn, interaction) in &query {
        if *interaction != Interaction::Pressed { continue; }
        let idx = btn.0;
        let unlocked = if idx < config.heroines.len() {
            let entry = &config.heroines[idx];
            entry.always_unlocked
                || engine.global_flags.get(&entry.unlock_flag).copied().unwrap_or(0) >= 1
        } else if idx == config.heroines.len() {
            engine.global_flags.get(&config.all_routes_cleared_flag).copied().unwrap_or(0) >= 1
        } else {
            engine.global_flags.get(&config.full_completion_flag).copied().unwrap_or(0) >= 1
        };
        if !unlocked { continue; }
        group.0 = Some(idx);
        for entity in &root_query {
            commands.entity(entity).despawn();
        }
        build_ui(&mut commands, &game_font.0, &config, &engine, group.0);
    }
}

fn handle_entry_buttons(
    query: Query<(&AfterStoryEntryButton, &Interaction), Changed<Interaction>>,
    mut selected_route: ResMut<SelectedRoute>,
    mut next_state: ResMut<NextState<AppState>>,
    group: Res<AfterStoryGroup>,
    config: Res<RouteConfig>,
) {
    for (btn, interaction) in &query {
        if *interaction != Interaction::Pressed { continue; }
        let group_idx = match group.0 { Some(idx) => idx, None => continue };
        let entries: Vec<&AfterStoryEntry> = if group_idx < config.heroines.len() {
            config.heroines[group_idx].after_stories.iter().collect()
        } else if group_idx == config.heroines.len() {
            config.extra_after_stories.iter().collect()
        } else {
            config.bonus_skits.iter().collect()
        };
        if btn.0 >= entries.len() { continue; }
        selected_route.0 = Some(entries[btn.0].script.clone());
        selected_route.1 = true;
        next_state.set(AppState::Gameplay);
    }
}

fn handle_back_button(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<AfterStoryBackBtn>)>,
    mut group: ResMut<AfterStoryGroup>,
    mut next_state: ResMut<NextState<AppState>>,
    root_query: Query<Entity, With<AfterStoryRoot>>,
    config: Res<RouteConfig>,
    game_font: Res<GameFont>,
    engine: Res<crate::script::ScriptEngine>,
) {
    for interaction in &query {
        if *interaction != Interaction::Pressed { continue; }
        if group.0.is_some() {
            group.0 = None;
            for entity in &root_query {
                commands.entity(entity).despawn();
            }
            build_ui(&mut commands, &game_font.0, &config, &engine, group.0);
        } else {
            next_state.set(AppState::Title);
        }
    }
}

fn handle_keyboard(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut group: ResMut<AfterStoryGroup>,
    mut next_state: ResMut<NextState<AppState>>,
    root_query: Query<Entity, With<AfterStoryRoot>>,
    config: Res<RouteConfig>,
    game_font: Res<GameFont>,
    engine: Res<crate::script::ScriptEngine>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if group.0.is_some() {
            group.0 = None;
            for entity in &root_query {
                commands.entity(entity).despawn();
            }
            build_ui(&mut commands, &game_font.0, &config, &engine, group.0);
        } else {
            next_state.set(AppState::Title);
        }
    }
}

fn cleanup_after_story(
    mut commands: Commands,
    query: Query<Entity, With<AfterStoryRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
