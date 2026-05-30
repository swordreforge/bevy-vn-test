# Route Branching System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** RouteConfig-driven route selection UI + route completion tracking.

**Architecture:** RON config (routes.ron) → RouteConfig resource → RoutePlugin (native Bevy UI) + script_runner reads config for RouteFlag/completion tracking. Route selection → SelectedRoute resource → Gameplay loads route script. Halt at route end → mark cleared in UnlockState.

**Tech Stack:** Bevy 0.18, RON serialization, Bevy UI (Node/Button/Text)

---

### Task 1: Create routes.ron + RouteConfig + RouteEntry

**Files:**
- Create: `assets/routes.ron`
- Modify: `src/resources.rs:258-265` (UnlockState area)

- [ ] **Step 1: Create assets/routes.ron**

```ron
(
    heroines: [
        (index: 1, name: "Fione", script: "aiy10010", unlock_flag: 51, hero_work: 1, ending_flags: [151, 152, 153]),
        (index: 2, name: "Eris", script: "aiy20010", unlock_flag: 52, hero_work: 2, ending_flags: [154, 155, 156]),
        (index: 3, name: "Colette", script: "aiy30010", unlock_flag: 53, hero_work: 3, ending_flags: [157, 158, 159]),
        (index: 4, name: "Lysia", script: "aiy40010", unlock_flag: 54, hero_work: 4, ending_flags: [160, 161, 162]),
        (index: 5, name: "Lavi", script: "aiy50010", unlock_flag: 55, hero_work: 5, ending_flags: [163, 164, 165]),
    ],
    extra: (
        index: 6,
        name: "After Story",
        script: "aiy00010",
        always_unlocked: true,
    ),
    route_unlock_flags: [103, 105, 107, 108, 110, 111],
    all_routes_cleared_flag: 113,
    full_completion_flag: 114,
    ending_flag_range: (151, 167),
    ending_count: 22,
)
```

- [ ] **Step 2: Add RouteEntry struct + RouteConfig Resource + SelectedRoute + load_routes() to resources.rs**

Insert after `UnlockState` block (after line 265):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub index: u32,
    pub name: String,
    pub script: String,
    pub unlock_flag: u32,
    pub hero_work: Option<u32>,
    pub ending_flags: Vec<u32>,
    pub always_unlocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub heroines: Vec<RouteEntry>,
    pub extra: RouteEntry,
    pub route_unlock_flags: Vec<u32>,
    pub all_routes_cleared_flag: u32,
    pub full_completion_flag: u32,
    pub ending_flag_range: (u32, u32),
    pub ending_count: u32,
}

impl RouteConfig {
    pub fn heroines_including_extra(&self) -> impl Iterator<Item = &RouteEntry> {
        self.heroines.iter().chain(std::iter::once(&self.extra))
    }

    pub fn find_by_index(&self, index: u32) -> Option<&RouteEntry> {
        self.heroines_including_extra().find(|e| e.index == index)
    }

    pub fn find_by_script(&self, script: &str) -> Option<&RouteEntry> {
        self.heroines_including_extra().find(|e| script.starts_with(&e.script))
    }
}

#[derive(Resource, Default)]
pub struct SelectedRoute(pub Option<String>);
```

Add `use serde::{Deserialize, Serialize};` import at top of resources.rs (should already be there).

Add `UnlockState` method:
```rust
impl UnlockState {
    pub fn is_route_cleared(&self, name: &str) -> bool {
        self.routes_cleared.contains(name)
    }

    pub fn mark_route_cleared(&mut self, name: &str) {
        self.routes_cleared.insert(name.to_string());
    }
}
```

- [ ] **Step 3: Check cargo check**

```bash
cargo check 2>&1
```
Expected: compiles (may have dead_code warnings on new types, that's OK)

---

### Task 2: Add RouteSelection state + modify enter_gameplay

**Files:**
- Modify: `src/state.rs:4-14`
- Modify: `src/plugins/script_runner.rs:98-103`

- [ ] **Step 1: Add RouteSelection to AppState**

`src/state.rs`:
```rust
pub enum AppState {
    #[default]
    Boot,
    Splash,
    Title,
    Gameplay,
    RouteSelection,
    Menu,
    SaveLoad,
    Gallery,
    Settings,
    Backlog,
}
```

- [ ] **Step 2: Modify start_script_execution to handle SelectedRoute**

`src/plugins/script_runner.rs` — replace the existing function:

```rust
fn start_script_execution(
    mut dialogue: ResMut<DialogueState>,
    mut engine: ResMut<ScriptEngine>,
    mut selected_route: ResMut<SelectedRoute>,
) {
    dialogue.current_text.clear();
    dialogue.current_speaker = None;
    dialogue.text_progress = 0;
    dialogue.is_displaying = false;

    if let Some(script) = selected_route.0.take() {
        engine.flags.clear();
        engine.global_flags.clear();
        engine.local_work.clear();
        engine.local_flags.clear();
        engine.dialogue_idx = 0;
        engine.finished = false;
        engine.call_stack.clear();
        engine.current_script = script;
        engine.current_line = 0;
        info!("Starting route script: {}", engine.current_script);
    }
}
```

- [ ] **Step 3: Add use for SelectedRoute**

```rust
use crate::resources::SelectedRoute;
```

Add at the top of script_runner.rs with the existing imports.

- [ ] **Step 4: Check cargo check**

```bash
cargo check 2>&1
```
Expected: compiles clean

---

### Task 3: Create RoutePlugin with full route selection UI

**Files:**
- Create: `src/plugins/routing.rs`

- [ ] **Step 1: Create routing.rs with RoutePlugin + components + setup + handlers + cleanup**

Full file:

```rust
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
                    || engine.global_flags.get(&entry.unlock_flag).copied().unwrap_or(0) >= 1;
                let cleared = unlock_state.is_route_cleared(&entry.name);
                let status_text = if !unlocked { "LOCKED" } else if cleared { "CLEARED" } else { "PLAY" };
                let bg_color = if !unlocked {
                    Color::srgba(0.2, 0.2, 0.25, 0.9)
                } else if cleared {
                    Color::srgba(0.15, 0.3, 0.5, 0.9)
                } else {
                    Color::srgba(0.15, 0.5, 0.2, 0.9)
                };

                grid.spawn((
                    RouteButton(entry.index),
                    Button,
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
                )).with_children(|btn| {
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

fn cleanup_route_selection(
    mut commands: Commands,
    query: Query<Entity, With<RouteScreen>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
```

- [ ] **Step 2: Add mod routing to plugins/mod.rs**

`src/plugins/mod.rs` — add line after `pub mod gallery;` (line 12):
```rust
pub mod routing;
```

- [ ] **Step 3: Check cargo check**

```bash
cargo check 2>&1
```
Expected: compiles clean

---

### Task 4: Add Route Selection entry to Title screen (+) Menu screen

**Files:**
- Modify: `src/plugins/title.rs:14-20` and `:70`
- Modify: `src/plugins/menu.rs:12-19` and `:58`

- [ ] **Step 1: Modify TitlePlugin**

`src/plugins/title.rs` — add Route variant to TitleButtonAction:
```rust
enum TitleButtonAction {
    NewGame,
    LoadGame,
    Settings,
    RouteSelection,
    Gallery,
    Exit,
}
```

Add button to the items list (replace the 5-element array with 6):
```rust
let items: [(TitleButtonAction, &str); 6] = [
    (TitleButtonAction::NewGame, "New Game"),
    (TitleButtonAction::LoadGame, "Load Game"),
    (TitleButtonAction::RouteSelection, "Routes"),
    (TitleButtonAction::Settings, "Settings"),
    (TitleButtonAction::Gallery, "Gallery"),
    (TitleButtonAction::Exit, "Exit"),
];
```

Add match arm in handle_title_buttons (after Settings):
```rust
TitleButtonAction::RouteSelection => {
    next_state.set(AppState::RouteSelection);
}
```

- [ ] **Step 2: Modify MenuPlugin**

`src/plugins/menu.rs` — add Route variant:
```rust
pub enum MenuButtonAction {
    Save,
    Load,
    RouteSelection,
    Settings,
    Gallery,
    Backlog,
    Title,
}
```

Add to items list (replace the 6-element array with 7):
```rust
let items: [(MenuButtonAction, &str); 7] = [
    (MenuButtonAction::Save, "Save"),
    (MenuButtonAction::Load, "Load"),
    (MenuButtonAction::Backlog, "Backlog"),
    (MenuButtonAction::RouteSelection, "Routes"),
    (MenuButtonAction::Settings, "Settings"),
    (MenuButtonAction::Gallery, "Gallery"),
    (MenuButtonAction::Title, "Back to Title"),
];
```

Add match arm in handle_menu_button_interaction (after Backlog):
```rust
MenuButtonAction::RouteSelection => next_state.set(AppState::RouteSelection),
```

- [ ] **Step 3: Check cargo check**

```bash
cargo check 2>&1
```
Expected: compiles clean

---

### Task 5: Refactor RouteFlag to use RouteConfig

**Files:**
- Modify: `src/plugins/script_runner.rs:661-679` (skip path)
- Modify: `src/plugins/script_runner.rs:1194-1213` (normal path)

- [ ] **Step 1: Replace hardcoded RouteFlag in both paths**

Skip path (~line 661):
```rust
Some(ScriptCmd::RouteFlag) => {
    let config = config.borrow();
    let count = config.route_unlock_flags.iter()
        .filter(|&f| engine.global_flags.get(f).copied().unwrap_or(0) >= 1)
        .count();
    if count == config.route_unlock_flags.len() {
        engine.global_flags.insert(config.all_routes_cleared_flag, 1);
    }
    if engine.global_flags.get(&config.full_completion_flag) != Some(&1) {
        let all_clear = (config.ending_flag_range.0..=config.ending_flag_range.1)
            .chain(std::iter::once(config.all_routes_cleared_flag))
            .all(|f| engine.global_flags.get(&f).copied().unwrap_or(0) >= 1);
        if all_clear {
            engine.global_flags.insert(config.full_completion_flag, 1);
        }
    }
}
```

Normal path (~line 1194):
```rust
Some(ScriptCmd::RouteFlag) => {
    let config = config.borrow();
    let count = config.route_unlock_flags.iter()
        .filter(|&f| engine.global_flags.get(f).copied().unwrap_or(0) >= 1)
        .count();
    if count == config.route_unlock_flags.len() {
        engine.global_flags.insert(config.all_routes_cleared_flag, 1);
    }
    if engine.global_flags.get(&config.full_completion_flag) != Some(&1) {
        let all_clear = (config.ending_flag_range.0..=config.ending_flag_range.1)
            .chain(std::iter::once(config.all_routes_cleared_flag))
            .all(|f| engine.global_flags.get(&f).copied().unwrap_or(0) >= 1);
        if all_clear {
            engine.global_flags.insert(config.full_completion_flag, 1);
        }
    }
}
```

- [ ] **Step 2: Add config field to ProcessAdvanceParams**

`src/plugins/script_runner.rs:43-76` — add to the struct:
```rust
    config: Res<'w, RouteConfig>,
```

And in the destructuring at line 152:
```rust
    ref mut config,
```

Note: the RouteFlag handling already uses `engine.global_flags` directly — with `config` available via the params struct, replace `engine.global_flags.get(&113)` with `config.all_routes_cleared_flag`, etc.

- [ ] **Step 3: Check cargo check**

```bash
cargo check 2>&1
```

to the function parameters (or extract from the params struct).

Let me check the actual function signatures to know exactly where to add.

- [ ] **Step 3: Check cargo check**

```bash
cargo check 2>&1
```
Expected: compiles clean

---

### Task 6: Route completion tracking on Halt

**Files:**
- Modify: `src/plugins/script_runner.rs:342-346` (skip Halt)
- Modify: `src/plugins/script_runner.rs:807-811` (normal Halt)
- Modify: `src/plugins/script_runner.rs`

- [ ] **Step 1: Add detect_route_completion to ScriptEngine**

`src/script.rs` — add method to `impl ScriptEngine`:
```rust
pub fn detect_route_completion(&self, config: &RouteConfig) -> Option<String> {
    config.find_by_script(&self.current_script).map(|e| e.name.clone())
}
```

- [ ] **Step 2: Call it in skip path Halt handler**

Replace the skip Halt handler (~line 342):
```rust
Some(ScriptCmd::Halt) => {
    if let Some(name) = engine.detect_route_completion(&config) {
        unlock_state.mark_route_cleared(&name);
    }
    engine.call_stack.clear();
    engine.current_script.clear();
    engine.current_line = 0;
}
```

- [ ] **Step 3: Call it in normal path Halt handler**

Replace the normal Halt handler (~line 807):
```rust
Some(ScriptCmd::Halt) => {
    if let Some(name) = engine.detect_route_completion(&config) {
        unlock_state.mark_route_cleared(&name);
    }
    engine.call_stack.clear();
    engine.current_script.clear();
    engine.current_line = 0;
}
```

- [ ] **Step 4: Add config field to ProcessAdvanceParams** (if not already added in Task 5)

`unlock_state` is already in `ProcessAdvanceParams` (line 49). If `config` was added in Task 5, it's already available. Otherwise add:
```rust
    config: Res<'w, RouteConfig>,
```
to the struct definition (line 43-76) and `ref mut config,` in the destructuring (line 152).

- [ ] **Step 5: Check cargo check**

```bash
cargo check 2>&1
```
Expected: compiles clean

---

### Task 7: Wire up in lib.rs + register everything

**Files:**
- Modify: `src/lib.rs:39-73`

- [ ] **Step 1: Add route plugin registration and RouteConfig init**

`src/lib.rs`:
```rust
use plugins::routing::RoutePlugin;
use resources::{load_routes, RouteConfig, SelectedRoute};
```

After `use plugins::gallery::GalleryPlugin;` (line 31), add: `use plugins::routing::RoutePlugin;`

After existing `.add_plugins(...)` chain and before `.init_resource::<ObjFileIndex>()`, add:
```rust
.init_resource::<SelectedRoute>()
```

Add RoutePlugin to plugin chain:
```rust
.add_plugins(RoutePlugin)
```

- [ ] **Step 2: Load RouteConfig inline in build_app() + init resources**

In `src/lib.rs`, in `build_app()` before `.init_resource::<ObjFileIndex>()`, add:
```rust
.insert_resource(
    ron::from_str::<RouteConfig>(include_str!("../assets/routes.ron"))
        .expect("Failed to parse routes.ron")
)
.init_resource::<SelectedRoute>()
```

Add use imports at top:
```rust
use resources::{GameFont, ObjFileIndex, RouteConfig, SelectedRoute};
```

- [ ] **Step 3: Check cargo check**

```bash
cargo check 2>&1
```
Expected: compiles clean

---

### Task 8: Add use import to resources.rs + final verification

**Files:**
- Modify: `src/resources.rs:1-6`

- [ ] **Step 1: Ensure all imports are correct for new Serialize/Deserialize needs**

Verify that `src/resources.rs` already has `use serde::{Deserialize, Serialize};` at the top.

- [ ] **Step 2: Final cargo check + full build**

```bash
cargo check 2>&1
```
Expected: compiles with no errors

```bash
cargo test -p artemis-export 2>&1
```
Expected: 76/76 tests pass

- [ ] **Step 3: Quick smoke test**

```bash
cargo run 2>&1 &
sleep 5
kill %1 2>/dev/null || true
```
Expected: starts without panic (show splash + title)
