# Phase 5: Save/Load System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Save/load system with 15-slot grid UI, JSON file I/O, state restoration, and a working menu.

**Architecture:** MenuPlugin handles `AppState::Menu` — spawns a dark overlay with menu buttons (Save/Load/Settings/Gallery/Title). SaveLoadPlugin handles `AppState::SaveLoad` — spawns 15-slot grid overlay, slot click → confirmation dialog → file I/O → state restore → return to Menu. `SaveLoadMode` resource (`bool`, true=Save) drives mode. `MenuToggleEvent` (Escape) now toggles between Gameplay↔Menu.

**Tech Stack:** Bevy 0.18, serde_json, std::fs

---

### Task 1: Add serde_json + SaveLoadMode + SaveManager I/O

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/resources.rs`

- [ ] **Step 1: Add serde_json to Cargo.toml**

Append after the `serde` line:
```
serde_json = "1"
```

- [ ] **Step 2: Add SaveLoadMode and SaveManager I/O to src/resources.rs**

After `ChoiceState` (end of file), add:

```rust
#[derive(Resource)]
pub struct SaveLoadMode(pub bool); // true = Save, false = Load
```

Add methods to `SaveManager` impl:

```rust
    pub fn refresh_from_disk(&mut self) {
        for i in 0..self.slots.len() {
            let path = format!("saves/slot_{}.json", i);
            match std::fs::read_to_string(&path) {
                Ok(json) => self.slots[i] = serde_json::from_str(&json).ok(),
                Err(_) => self.slots[i] = None,
            }
        }
    }

    pub fn save_slot(&mut self, idx: usize, data: SaveData) {
        let _ = std::fs::create_dir_all("saves");
        let path = format!("saves/slot_{}.json", idx);
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&path, &json);
        }
        self.slots[idx] = Some(data);
    }

    pub fn load_slot_from_disk(&mut self, idx: usize) -> Option<SaveData> {
        let path = format!("saves/slot_{}.json", idx);
        let json = std::fs::read_to_string(path).ok()?;
        let data: SaveData = serde_json::from_str(&json).ok()?;
        self.slots[idx] = Some(data.clone());
        Some(data)
    }
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: PASS

---

### Task 2: Add save/load UI components

**Files:**
- Modify: `src/components.rs`

- [ ] **Step 1: Add marker components**

Append:

```rust
#[derive(Component)]
pub struct SaveLoadUiRoot;

#[derive(Component)]
pub struct SaveSlot(pub usize);

#[derive(Component)]
pub struct ConfirmDialogRoot;

#[derive(Component)]
pub struct ConfirmYesButton;

#[derive(Component)]
pub struct ConfirmNoButton;
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: PASS

---

### Task 3: Create MenuPlugin

**Files:**
- Create: `src/plugins/menu.rs`

- [ ] **Step 1: Write MenuPlugin**

```rust
use bevy::prelude::*;
use crate::resources::SaveLoadMode;
use crate::state::AppState;
use crate::plugins::inputs::MenuToggleEvent;

pub struct MenuPlugin;

#[derive(Component)]
pub struct MenuUiRoot;

#[derive(Component)]
pub enum MenuButtonAction {
    Save,
    Load,
    Settings,
    Gallery,
    Title,
}

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SaveLoadMode>()
            .add_systems(OnEnter(AppState::Menu), setup_menu_ui)
            .add_systems(OnExit(AppState::Menu), cleanup_menu_ui)
            .add_systems(Update, (
                handle_menu_button_interaction,
                handle_menu_toggle,
            ));
    }
}

fn setup_menu_ui(mut commands: Commands) {
    commands.spawn((
        MenuUiRoot,
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
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        ZIndex(5),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("MENU"),
            TextFont { font_size: 36.0, ..default() },
            TextColor(Color::WHITE),
            Node { ..default() },
        ));
        let items: [(MenuButtonAction, &str); 5] = [
            (MenuButtonAction::Save, "Save"),
            (MenuButtonAction::Load, "Load"),
            (MenuButtonAction::Settings, "Settings"),
            (MenuButtonAction::Gallery, "Gallery"),
            (MenuButtonAction::Title, "Back to Title"),
        ];
        for (action, label) in items {
            parent.spawn((
                action,
                Button,
                Text::new(label),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 1.0)),
                Node { ..default() },
            ));
        }
    });
}

fn cleanup_menu_ui(mut commands: Commands, query: Query<Entity, With<MenuUiRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn handle_menu_button_interaction(
    query: Query<(&MenuButtonAction, &Interaction), Changed<Interaction>>,
    mut mode: ResMut<SaveLoadMode>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (action, interaction) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            MenuButtonAction::Save => { mode.0 = true; next_state.set(AppState::SaveLoad); }
            MenuButtonAction::Load => { mode.0 = false; next_state.set(AppState::SaveLoad); }
            MenuButtonAction::Settings => next_state.set(AppState::Settings),
            MenuButtonAction::Gallery => next_state.set(AppState::Gallery),
            MenuButtonAction::Title => next_state.set(AppState::Title),
        }
    }
}

fn handle_menu_toggle(
    mut ev: MessageReader<MenuToggleEvent>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for _ in ev.read() {
        match state.get() {
            AppState::Gameplay => next_state.set(AppState::Menu),
            AppState::Menu => next_state.set(AppState::Gameplay),
            _ => {}
        }
    }
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: PASS

---

### Task 4: Rewrite SaveLoadPlugin

**Files:**
- Modify: `src/plugins/save_load.rs`

- [ ] **Step 1: Write full SaveLoadPlugin**

Replace the entire file with:

```rust
use bevy::prelude::*;
use crate::components::*;
use crate::resources::{SaveLoadMode, SaveManager, SaveData};
use crate::state::AppState;
use crate::script::ScriptEngine;

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SaveManager::new(15))
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
            TextFont { font_size: 32.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));
        for row in 0..3 {
            parent.with_child((
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
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgb(0.4, 0.4, 0.4)),
                        Node { ..default() },
                    ));
                    if let Some(ref data) = save_mgr.slots[idx] {
                        slot.with_child((
                            Text::new(&data.scene_name),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(Color::WHITE),
                            Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
                        ));
                        slot.with_child((
                            Text::new(&data.timestamp),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.6, 0.6, 0.6)),
                            Node { margin: UiRect::top(Val::Px(2.0)), ..default() },
                        ));
                        slot.with_child((
                            Text::new(format!("line {}", data.script_line)),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.6, 0.6, 0.6)),
                            Node { ..default() },
                        ));
                    } else {
                        slot.with_child((
                            Text::new("-- EMPTY --"),
                            TextFont { font_size: 16.0, ..default() },
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
        commands.entity(entity).despawn_recursive();
    }
}

fn handle_slot_click(
    mut commands: Commands,
    query: Query<(&Interaction, &SaveSlot), Changed<Interaction>>,
    mode: Res<SaveLoadMode>,
    save_mgr: Res<SaveManager>,
    existing: Query<Entity, With<ConfirmDialogRoot>>,
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
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::WHITE),
                Node { ..default() },
            ));
            parent.with_child((
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
                    TextFont { font_size: 20.0, ..default() },
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
                    TextFont { font_size: 20.0, ..default() },
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
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    let Some(confirm) = confirm else { return };
    let idx = confirm.0;

    for interaction in &yes_query {
        if *interaction == Interaction::Pressed {
            if mode.0 {
                let data = build_save_data(&script_engine, &affection);
                save_mgr.save_slot(idx, data);
            } else if let Some(data) = save_mgr.load_slot_from_disk(idx) {
                script_engine.current_line = data.script_line;
                script_engine.call_stack = data.call_stack;
                script_engine.flags = data.flags;
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
                commands.entity(entity).despawn_recursive();
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

fn build_save_data(engine: &ScriptEngine, affection: &AffectionMap) -> SaveData {
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
        play_time: 0,
    }
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: PASS

---

### Task 5: Register MenuPlugin + adjust main.rs + build

**Files:**
- Modify: `src/plugins/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Register MenuPlugin in mod.rs**

Add `pub mod menu;` to `src/plugins/mod.rs` (alphabetically between `inputs` and `rendering` or at the end).

- [ ] **Step 2: Register MenuPlugin in main.rs**

Add `use plugins::menu::MenuPlugin;` and `.add_plugins(MenuPlugin)` after `ChoicePlugin`.

- [ ] **Step 3: cargo check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 4: cargo build**

Run: `cargo build`
Expected: Build succeeds (11-16 pre-existing warnings OK)

---

### Task 6: Smoke test

- [ ] **Step 1: Launch and verify**

```bash
timeout 5 ./target/debug/bevy-vn 2>&1 || true
```

Expected:
- Window opens at 1280×720
- Title screen → click → Gameplay
- Press Escape → Menu appears with 5 buttons
- Click Save → 15-slot grid, all empty
- Click any slot → confirm dialog → click Yes → slot shows as filled
- Escape → Menu → Load → same grid, slot shows as filled
- Return to title to verify state flow

- [ ] **Step 2: Check for critical errors**

If "Entity despawned" warnings appear, note them but continue.
No panics expected.
