use bevy::prelude::*;
use crate::components::*;
use crate::resources::{GameFont, SaveLoadMode, SaveManager, SaveData, AffectionMap, UnlockState};
use crate::state::AppState;
use crate::script::ScriptEngine;

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SaveManager::new(15))
            .add_systems(Startup, |mut mgr: ResMut<SaveManager>| mgr.refresh_from_disk())
            .add_systems(OnEnter(AppState::SaveLoad), setup_save_load_ui)
            .add_systems(OnExit(AppState::SaveLoad), cleanup_save_load_ui)
            .add_systems(Update, (
                handle_slot_click,
                handle_confirm,
                handle_save_load_escape,
            ));
    }
}

const SLOT_FILLED: Color = Color::srgba(0.12, 0.12, 0.12, 0.95);
const SLOT_EMPTY: Color = Color::srgba(0.08, 0.08, 0.08, 0.95);
const SLOT_DISABLED: Color = Color::srgba(0.04, 0.04, 0.04, 0.95);

#[derive(Resource)]
struct ConfirmState(usize);

fn setup_save_load_ui(
    mut commands: Commands,
    mode: Res<SaveLoadMode>,
    save_mgr: Res<SaveManager>,
    game_font: Res<GameFont>,
) {
    commands.spawn((
        SaveLoadUiRoot,
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
    )).with_children(|parent| {
        parent.spawn((
            Text::new(if mode.0 { "SAVE" } else { "LOAD" }),
            TextFont { font: game_font.0.clone(), font_size: 32.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));
        for row in 0..3 {
            parent.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
            )).with_children(|row_parent| {
                for col in 0..5 {
                    let idx = row * 5 + col;
                    let has_data = save_mgr.slots[idx].is_some();
                    let clickable = mode.0 || has_data;
                    let mut slot = row_parent.spawn((
                        SaveSlot(idx),
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(130.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(if has_data { SLOT_FILLED } else { SLOT_EMPTY }),
                    ));
                    if !clickable {
                        slot.insert(BackgroundColor(SLOT_DISABLED));
                    }
                    slot.with_child((
                        Text::new(format!("{}", idx + 1)),
                        TextFont { font: game_font.0.clone(), font_size: 14.0, ..default() },
                        TextColor(Color::srgb(0.4, 0.4, 0.4)),
                        Node { ..default() },
                    ));
                    if let Some(ref data) = save_mgr.slots[idx] {
                        slot.with_child((
                            Text::new(&data.scene_name),
                            TextFont { font: game_font.0.clone(), font_size: 16.0, ..default() },
                            TextColor(Color::WHITE),
                            Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
                        ));
                        slot.with_child((
                            Text::new(&data.timestamp),
                            TextFont { font: game_font.0.clone(), font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.6, 0.6, 0.6)),
                            Node { margin: UiRect::top(Val::Px(2.0)), ..default() },
                        ));
                        slot.with_child((
                            Text::new(format!("line {}", data.script_line)),
                            TextFont { font: game_font.0.clone(), font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.6, 0.6, 0.6)),
                            Node { ..default() },
                        ));
                    } else {
                        slot.with_child((
                            Text::new("-- EMPTY --"),
                            TextFont { font: game_font.0.clone(), font_size: 16.0, ..default() },
                            TextColor(Color::srgb(0.3, 0.3, 0.3)),
                            Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
                        ));
                    }
                }
            });
        }
    });
}

fn cleanup_save_load_ui(mut commands: Commands, roots: Query<Entity, Or<(With<SaveLoadUiRoot>, With<ConfirmDialogRoot>)>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

fn handle_slot_click(
    mut commands: Commands,
    query: Query<(&Interaction, &SaveSlot), Changed<Interaction>>,
    mode: Res<SaveLoadMode>,
    save_mgr: Res<SaveManager>,
    existing: Query<Entity, With<ConfirmDialogRoot>>,
    game_font: Res<GameFont>,
) {
    if !existing.is_empty() {
        return;
    }
    for (interaction, slot) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let idx = slot.0;
        let has_data = save_mgr.slots[idx].is_some();
        if !mode.0 && !has_data {
            continue;
        }
        let text = if has_data {
            format!("{} slot {}?", if mode.0 { "Overwrite" } else { "Load" }, idx + 1)
        } else {
            format!("Save to slot {}?", idx + 1)
        };
        commands.insert_resource(ConfirmState(idx));
        commands.spawn((
            ConfirmDialogRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ZIndex(6),
        )).with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont { font: game_font.0.clone(), font_size: 24.0, ..default() },
                TextColor(Color::WHITE),
                Node { ..default() },
            ));
            parent.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(24.0),
                    ..default()
                },
            )).with_children(|row| {
                row.spawn((
                    ConfirmYesButton,
                    Button,
                    Text::new("Yes"),
                    TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.6, 1.0, 0.6)),
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.3, 0.2, 0.9)),
                ));
                row.spawn((
                    ConfirmNoButton,
                    Button,
                    Text::new("No"),
                    TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.6, 0.6)),
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.3, 0.2, 0.2, 0.9)),
                ));
            });
        });
    }
}

fn handle_confirm(
    confirm: Option<Res<ConfirmState>>,
    yes_query: Query<&Interaction, (With<ConfirmYesButton>, Changed<Interaction>)>,
    no_query: Query<&Interaction, (With<ConfirmNoButton>, Changed<Interaction>)>,
    confirm_dialogs: Query<Entity, With<ConfirmDialogRoot>>,
    mode: Res<SaveLoadMode>,
    mut save_mgr: ResMut<SaveManager>,
    mut script_engine: ResMut<ScriptEngine>,
    affection: Res<AffectionMap>,
    mut unlock_state: ResMut<UnlockState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    let Some(confirm) = confirm else { return };
    let idx = confirm.0;

    for interaction in &yes_query {
        if *interaction == Interaction::Pressed {
            if mode.0 {
                let data = build_save_data(&script_engine, &affection, &unlock_state);
                save_mgr.save_slot(idx, data);
            } else if let Some(data) = save_mgr.load_slot_from_disk(idx) {
                script_engine.current_line = data.script_line;
                script_engine.call_stack = data.call_stack;
                script_engine.flags = data.flags;
                *unlock_state = data.unlock_state;
            }
            commands.remove_resource::<ConfirmState>();
            if mode.0 {
                next_state.set(AppState::Menu);
            } else {
                next_state.set(AppState::Gameplay);
            }
            return;
        }
    }

    for interaction in &no_query {
        if *interaction == Interaction::Pressed {
            for entity in &confirm_dialogs {
                commands.entity(entity).despawn();
            }
            commands.remove_resource::<ConfirmState>();
            return;
        }
    }
}

fn handle_save_load_escape(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) && *state == AppState::SaveLoad {
        next_state.set(AppState::Menu);
    }
}

fn build_save_data(engine: &ScriptEngine, affection: &AffectionMap, unlock_state: &UnlockState) -> SaveData {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default();
    SaveData {
        version: 1,
        timestamp,
        scene_name: engine.current_script.clone(),
        script_path: format!("{}.bscript.ron", engine.current_script),
        script_line: engine.current_line,
        call_stack: engine.call_stack.clone(),
        flags: engine.flags.clone(),
        affection: affection.0.clone(),
        unlock_state: unlock_state.clone(),
        play_time: 0,
    }
}
