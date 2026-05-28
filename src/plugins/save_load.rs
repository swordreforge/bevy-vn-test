use bevy::prelude::*;
use std::collections::HashSet;
use crate::components::*;
use crate::resources::{BgmManager, BgmXManager, GameFont, SaveLoadMode, SaveLoadPage, SaveManager, SaveData, AffectionMap, Settings, UnlockState};
use crate::state::AppState;
use crate::script::{ScriptCmd, ScriptEngine};
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowCgMessage, HideCgMessage,
};
use crate::audio_messages::{
    PlayBgmMessage, PlayBgmXMessage, StopBgmMessage,
};
use crate::plugins::event_system::{ViewPhase, ViewState};
use crate::plugins::event_system::view_data;

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SaveManager::new(75))
            .init_resource::<SaveLoadPage>()
            .add_systems(Startup, |mut mgr: ResMut<SaveManager>| mgr.refresh_from_disk())
            .add_systems(OnEnter(AppState::SaveLoad), setup_save_load_ui)
            .add_systems(OnExit(AppState::SaveLoad), cleanup_save_load_ui)
            .add_systems(Update, (
                handle_slot_click,
                handle_confirm,
                handle_save_load_escape,
                handle_save_load_page_nav,
            ))
            .add_systems(Update, (
                process_scene_restore,
                process_load_restore,
            ).run_if(in_state(AppState::Gameplay)));
    }
}

const SLOT_FILLED: Color = Color::srgba(0.12, 0.12, 0.12, 0.95);
const SLOT_EMPTY: Color = Color::srgba(0.08, 0.08, 0.08, 0.95);
const SLOT_DISABLED: Color = Color::srgba(0.04, 0.04, 0.04, 0.95);
const SLOTS_PER_PAGE: usize = 15;

#[derive(Resource)]
struct PendingSceneRestore(Vec<ScriptCmd>);

#[derive(Resource)]
struct PendingLoadRestore {
    bgm_id: Option<String>,
    bgmx_id: Option<String>,
    view_char_id: Option<String>,
    window_color_idx: i32,
}

#[derive(Resource)]
struct ConfirmState(usize);

fn setup_save_load_ui(
    mut commands: Commands,
    mode: Res<SaveLoadMode>,
    save_mgr: Res<SaveManager>,
    game_font: Res<GameFont>,
    mut page: ResMut<SaveLoadPage>,
) {
    page.0 = 0;
    let total_pages = ((save_mgr.slots.len() + SLOTS_PER_PAGE - 1) / SLOTS_PER_PAGE).max(1);

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

        parent.spawn((
            SaveLoadSlotGrid,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        )).with_children(|grid| {
            for row in 0..3 {
                grid.spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        margin: UiRect::vertical(Val::Px(6.0)),
                        ..default()
                    },
                )).with_children(|row_parent| {
                    for col in 0..5 {
                        let idx = page.0 * SLOTS_PER_PAGE + row * 5 + col;
                        if idx >= save_mgr.slots.len() { continue; }
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

        parent.spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(16.0),
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
        )).with_children(|nav| {
            nav.spawn((
                SaveLoadPageLeftBtn,
                Button,
                Node {
                    width: Val::Px(36.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
            )).with_child((
                Text::new("◀"),
                TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
                TextColor(Color::WHITE),
            ));

            nav.spawn((
                SaveLoadPageText,
                Text::new(format!("Page {}/{}", page.0 + 1, total_pages)),
                TextFont { font: game_font.0.clone(), font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.8)),
            ));

            nav.spawn((
                SaveLoadPageRightBtn,
                Button,
                Node {
                    width: Val::Px(36.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
            )).with_child((
                Text::new("▶"),
                TextFont { font: game_font.0.clone(), font_size: 20.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
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
    mut affection: ResMut<AffectionMap>,
    mut unlock_state: ResMut<UnlockState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    settings: Res<Settings>,
    bgm: Res<BgmManager>,
    bgmx: Res<BgmXManager>,
    view_query: Query<&ViewState>,
) {
    let Some(confirm) = confirm else { return };
    let idx = confirm.0;

    for interaction in &yes_query {
        if *interaction == Interaction::Pressed {
            if mode.0 {
                let data = build_save_data(
                    &script_engine, &affection, &unlock_state,
                    &settings, &bgm, &bgmx,
                    view_query.single().ok(),
                );
                save_mgr.save_slot(idx, data);
            } else if let Some(data) = save_mgr.load_slot_from_disk(idx) {
                let scene_name = data.scene_name.clone();
                let script_line = data.script_line;
                let call_stack = data.call_stack.clone();
                let flags = data.flags.clone();
                let global_flags = data.global_flags.clone();
                let unlocked = data.unlock_state.clone();
                let aff = data.affection.clone();

                script_engine.current_script = scene_name;
                script_engine.current_line = script_line;
                script_engine.call_stack = call_stack;
                script_engine.flags = flags;
                script_engine.global_flags = global_flags;
                *unlock_state = unlocked;
                *affection = AffectionMap(aff);

                commands.insert_resource(PendingLoadRestore {
                    bgm_id: data.bgm_id.clone(),
                    bgmx_id: data.bgmx_id.clone(),
                    view_char_id: data.view_char_id.clone(),
                    window_color_idx: data.window_color_idx,
                });

                let cmds = collect_scene_restore(&script_engine);
                if !cmds.is_empty() {
                    commands.insert_resource(PendingSceneRestore(cmds));
                }
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

fn handle_save_load_page_nav(
    keys: Res<ButtonInput<KeyCode>>,
    mut page: ResMut<SaveLoadPage>,
    save_mgr: Res<SaveManager>,
    grid_query: Query<Entity, With<SaveLoadSlotGrid>>,
    children_query: Query<&Children, With<SaveLoadSlotGrid>>,
    page_text_query: Query<Entity, With<SaveLoadPageText>>,
    left_btn_query: Query<&Interaction, (Changed<Interaction>, With<SaveLoadPageLeftBtn>)>,
    right_btn_query: Query<&Interaction, (Changed<Interaction>, With<SaveLoadPageRightBtn>)>,
    dialogs: Query<Entity, With<ConfirmDialogRoot>>,
    mut commands: Commands,
    game_font: Res<GameFont>,
    mode: Res<SaveLoadMode>,
) {
    if !dialogs.is_empty() {
        return;
    }

    let total_pages = ((save_mgr.slots.len() + SLOTS_PER_PAGE - 1) / SLOTS_PER_PAGE).max(1);
    let old_page = page.0;

    if keys.just_pressed(KeyCode::ArrowLeft) {
        page.0 = if page.0 == 0 { total_pages - 1 } else { page.0 - 1 };
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        page.0 = (page.0 + 1) % total_pages;
    }

    for interaction in &left_btn_query {
        if *interaction == Interaction::Pressed {
            page.0 = if page.0 == 0 { total_pages - 1 } else { page.0 - 1 };
        }
    }
    for interaction in &right_btn_query {
        if *interaction == Interaction::Pressed {
            page.0 = (page.0 + 1) % total_pages;
        }
    }

    if page.0 != old_page {
        for children in &children_query {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        for entity in &grid_query {
            commands.entity(entity).with_children(|grid| {
                for row in 0..3 {
                    grid.spawn((
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(12.0),
                            margin: UiRect::vertical(Val::Px(6.0)),
                            ..default()
                        },
                    )).with_children(|row_parent| {
                        for col in 0..5 {
                            let idx = page.0 * SLOTS_PER_PAGE + row * 5 + col;
                            if idx >= save_mgr.slots.len() { continue; }
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

        for entity in &page_text_query {
            commands.entity(entity).insert(Text::new(
                format!("Page {}/{}", page.0 + 1, total_pages),
            ));
        }
    }
}

fn collect_scene_restore(engine: &ScriptEngine) -> Vec<ScriptCmd> {
    let mut result = Vec::new();
    let mut found_bg = false;
    let mut found_cg = false;
    let mut found_sprites: HashSet<String> = HashSet::new();

    scan_script_backwards(engine, &engine.current_script, engine.current_line,
        &mut result, &mut found_bg, &mut found_cg, &mut found_sprites);

    for (script_name, return_line) in engine.call_stack.iter().rev() {
        scan_script_backwards(engine, script_name, *return_line,
            &mut result, &mut found_bg, &mut found_cg, &mut found_sprites);
    }

    result.reverse();
    result
}

fn scan_script_backwards(
    engine: &ScriptEngine,
    script_name: &str,
    up_to_line: usize,
    result: &mut Vec<ScriptCmd>,
    found_bg: &mut bool,
    found_cg: &mut bool,
    found_sprites: &mut HashSet<String>,
) {
    let Some(script) = engine.scripts.get(script_name) else { return };
    let end = up_to_line.min(script.len());
    for i in (0..end).rev() {
        match &script[i] {
            ScriptCmd::SetBg { file, .. } if !*found_bg => {
                result.push(ScriptCmd::SetBg { file: file.clone(), transition: None, duration: None });
                *found_bg = true;
            }
            ScriptCmd::ShowCg { file, .. } if !*found_cg => {
                result.push(ScriptCmd::ShowCg { file: file.clone(), transition: None });
                *found_cg = true;
            }
            ScriptCmd::HideCg { .. } if !*found_cg => {
                result.push(ScriptCmd::HideCg { transition: None });
                *found_cg = true;
            }
            ScriptCmd::ShowFg { char_id, expression, position, .. }
                if !found_sprites.contains(char_id) =>
            {
                result.push(ScriptCmd::ShowFg {
                    char_id: char_id.clone(),
                    expression: expression.clone(),
                    position: position.clone(),
                    transition: None,
                });
                found_sprites.insert(char_id.clone());
            }
            ScriptCmd::HideFg { char_id, .. }
                if !found_sprites.contains(char_id) =>
            {
                result.push(ScriptCmd::HideFg { char_id: char_id.clone(), transition: None });
                found_sprites.insert(char_id.clone());
            }
            _ => {}
        }
    }
}

fn process_scene_restore(
    pending: Option<Res<PendingSceneRestore>>,
    mut commands: Commands,
    mut set_bg_writer: MessageWriter<SetBgMessage>,
    mut show_fg_writer: MessageWriter<ShowFgMessage>,
    mut hide_fg_writer: MessageWriter<HideFgMessage>,
    mut show_cg_writer: MessageWriter<ShowCgMessage>,
    mut hide_cg_writer: MessageWriter<HideCgMessage>,
    _play_bgm_writer: MessageWriter<PlayBgmMessage>,
    _stop_bgm_writer: MessageWriter<StopBgmMessage>,
) {
    let Some(pending) = pending else { return };
    for cmd in &pending.0 {
        match cmd {
            ScriptCmd::SetBg { file, .. } => {
                set_bg_writer.write(SetBgMessage { file: file.clone(), transition: None, duration: None });
            }
            ScriptCmd::ShowFg { char_id, expression, position, .. } => {
                show_fg_writer.write(ShowFgMessage {
                    char_id: char_id.clone(),
                    expression: expression.clone(),
                    position: position.clone(),
                    transition: None,
                    duration: None,
                });
            }
            ScriptCmd::HideFg { char_id, .. } => {
                hide_fg_writer.write(HideFgMessage { char_id: char_id.clone(), transition: None, duration: None });
            }
            ScriptCmd::ShowCg { file, .. } => {
                show_cg_writer.write(ShowCgMessage { file: file.clone(), transition: None, duration: None });
            }
            ScriptCmd::HideCg { .. } => {
                hide_cg_writer.write(HideCgMessage { transition: None, duration: None });
            }
            _ => {}
        }
    }
    commands.remove_resource::<PendingSceneRestore>();
}

fn process_load_restore(
    pending: Option<Res<PendingLoadRestore>>,
    mut commands: Commands,
    mut play_bgm: MessageWriter<PlayBgmMessage>,
    mut play_bgmx: MessageWriter<PlayBgmXMessage>,
) {
    let Some(pending) = pending else { return };

    if let Some(bgm_id) = &pending.bgm_id {
        play_bgm.write(PlayBgmMessage { id: bgm_id.clone(), volume: None, fade_in: None });
    }
    if let Some(bgmx_id) = &pending.bgmx_id {
        play_bgmx.write(PlayBgmXMessage { id: bgmx_id.clone(), volume: None, fade_in: None });
    }
    if let Some(char_id) = &pending.view_char_id {
        if let Some(entry) = view_data::lookup_view_entry(char_id) {
            let tween_entry = view_data::lookup_tween_entry(entry.pen_type)
                .unwrap_or_else(|| view_data::lookup_tween_entry(2).unwrap());
            commands.spawn(ViewState {
                char_id: char_id.clone(),
                phase: ViewPhase::FadeOut,
                timer: Timer::from_seconds(1.0, TimerMode::Once),
                step_idx: 0,
                pen_entity: None,
                name_entity: None,
                mask_material: None,
                scene_entities: Vec::new(),
                entry,
                tween_entry,
            });
        }
    }
    let wc = pending.window_color_idx;
    commands.queue(move |world: &mut World| {
        let mut settings = world.resource_mut::<Settings>();
        settings.window_color_idx = wc;
    });
    commands.remove_resource::<PendingLoadRestore>();
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

fn build_save_data(
    engine: &ScriptEngine,
    affection: &AffectionMap,
    unlock_state: &UnlockState,
    settings: &Settings,
    bgm: &BgmManager,
    bgmx: &BgmXManager,
    view_state: Option<&ViewState>,
) -> SaveData {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_default();
    SaveData {
        version: 2,
        timestamp,
        scene_name: engine.current_script.clone(),
        script_path: format!("{}.bscript.ron", engine.current_script),
        script_line: engine.current_line,
        call_stack: engine.call_stack.clone(),
        flags: engine.flags.clone(),
        global_flags: engine.global_flags.clone(),
        affection: affection.0.clone(),
        unlock_state: unlock_state.clone(),
        play_time: 0,
        window_color_idx: settings.window_color_idx,
        view_char_id: view_state.map(|vs| vs.char_id.clone()),
        bgm_id: bgm.current_id.clone(),
        bgmx_id: bgmx.current_id.clone(),
    }
}
