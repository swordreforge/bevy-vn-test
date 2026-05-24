# Phase 6: Affection Condition Branching + CG Gallery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add affection-based script branching and a full CG gallery with unlock tracking.

**Architecture:** New `ScriptCmd` variants (`AffectionCondition`, `UnlockCg`) evaluated by `ScriptRunner`. Gallery plugin rewritten with 3-column grid, fullscreen viewer, and auto-unlock on `ShowCg`. Static manifest for known CG files, `UnlockState` for progress tracking.

**Tech Stack:** Bevy 0.18, RON scripts, serde, UI nodes with `ImageNode`

---

### Task 1: Add new ScriptCmd variants

**Files:**
- Modify: `src/script.rs:20-95`

- [ ] **Step 1: Add AffectionCondition and UnlockCg to ScriptCmd**

```rust
// After AffectionChange { char_id, delta },
AffectionCondition {
    char_id: String,
    value: i32,
    operator: ConditionOp,
    goto: String,
},
// After SavePoint,
UnlockCg {
    file: String,
},
```

Insert `AffectionCondition` after line 70 (`AffectionChange`) and `UnlockCg` after line 84 (`SavePoint`).

- [ ] **Step 2: cargo check**

Run: `cargo check`
Expected: succeeds with pre-existing warnings only

- [ ] **Step 3: Commit**

---

### Task 2: Add GalleryState resource and gallery components

**Files:**
- Modify: `src/resources.rs:153-160`
- Modify: `src/components.rs:44`

- [ ] **Step 1: Add GalleryState resource to resources.rs**

At the end of `src/resources.rs`, after `SaveLoadMode`:

```rust
#[derive(Resource, Default)]
pub struct GalleryState {
    pub fullscreen: Option<String>, // None = grid mode, Some(file) = fullscreen CG
}
```

- [ ] **Step 2: Add gallery components to components.rs**

At the end of `src/components.rs`, after `ConfirmNoButton`:

```rust
#[derive(Component)]
pub struct GalleryRoot;

#[derive(Component)]
pub struct GalleryThumbnail(pub String);

#[derive(Component)]
pub struct GalleryLocked;

#[derive(Component)]
pub struct GalleryFullscreen;

#[derive(Component)]
pub struct GalleryBackButton;
```

- [ ] **Step 3: cargo check**

Run: `cargo check`
Expected: succeeds

- [ ] **Step 4: Commit**

---

### Task 3: Update ScriptRunner for affection conditions and CG unlock

**Files:**
- Modify: `src/plugins/script_runner.rs`

- [ ] **Step 1: Add UnlockState to imports and system params**

Add to existing imports:

```rust
use crate::resources::UnlockState;
use crate::script::{ConditionOp, ScriptCmd, ScriptEngine, FgPosition};
```

In the `process_advance` function signature, add `mut unlock_state: ResMut<UnlockState>`:

```rust
fn process_advance(
    mut advance_ev: MessageReader<AdvanceEvent>,
    mut engine: ResMut<ScriptEngine>,
    mut dialogue: ResMut<DialogueState>,
    mut affection: ResMut<AffectionMap>,
    mut unlock_state: ResMut<UnlockState>,
    mut set_bg_writer: MessageWriter<SetBgMessage>,
    ...
)
```

- [ ] **Step 2: Add auto-unlock on ShowCg**

In the `ShowCg` match arm (around line 142-144), add after the message write:

```rust
Some(ScriptCmd::ShowCg { file, transition: _ }) => {
    show_cg_writer.write(ShowCgMessage { file: file.clone() });
    unlock_state.cg_unlocked.insert(file);
}
```

- [ ] **Step 3: Add AffectionCondition match arm**

After the `AffectionChange` match arm (around line 129-131), add:

```rust
Some(ScriptCmd::AffectionCondition { char_id, value, operator, goto }) => {
    let affection_val = affection.0.get(&char_id).copied().unwrap_or(0);
    let met = match operator {
        ConditionOp::Greater => affection_val > value,
        ConditionOp::Less => affection_val < value,
        ConditionOp::Equal => affection_val == value,
        ConditionOp::GreaterEqual => affection_val >= value,
        ConditionOp::LessEqual => affection_val <= value,
    };
    if met && !engine.jump_to_label(&goto) {
        warn!("AffectionCondition jump target not found: {}", goto);
    }
}
```

- [ ] **Step 4: Add UnlockCg match arm**

After `SavePoint` (around line 132), add:

```rust
Some(ScriptCmd::UnlockCg { file }) => {
    unlock_state.cg_unlocked.insert(file);
}
```

- [ ] **Step 5: cargo check**

Run: `cargo check`
Expected: succeeds

- [ ] **Step 6: Commit**

---

### Task 4: Rewrite Gallery plugin

**Files:**
- Modify: `src/plugins/gallery.rs`

- [ ] **Step 1: Write the full gallery module**

Replace entire `src/plugins/gallery.rs`:

```rust
use bevy::prelude::*;
use crate::state::AppState;
use crate::resources::{UnlockState, GalleryState, TextureCache};
use crate::components::*;

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnlockState>()
            .init_resource::<GalleryState>()
            .init_resource::<TextureCache>()
            .add_systems(OnEnter(AppState::Gallery), setup_gallery)
            .add_systems(Update, (
                handle_thumbnail_click,
                handle_back_button,
                handle_fullscreen_click,
                handle_gallery_escape,
            ).run_if(in_state(AppState::Gallery)))
            .add_systems(OnExit(AppState::Gallery), cleanup_gallery);
    }
}

const ALL_CG_FILES: &[&str] = &["eve_010101.png"];

#[derive(Component)]
struct GalleryScreen;

fn setup_gallery(
    mut commands: Commands,
    unlock_state: Res<UnlockState>,
    gallery_state: Res<GalleryState>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
) {
    // Root overlay
    commands.spawn((
        GalleryRoot,
        GalleryScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.95)),
        ZIndex(5),
    )).with_children(|parent| {
        // Header row
        parent.spawn((
            GalleryBackButton,
            GalleryScreen,
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
            GalleryScreen,
            Text::new("← Back"),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ));

        // Title
        parent.spawn((
            GalleryScreen,
            Text::new("CG Gallery"),
            TextFont { font_size: 28.0, ..default() },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::top(Val::Px(12.0)),
                ..default()
            },
        ));

        // Grid container
        parent.spawn((
            GalleryScreen,
            Node {
                width: Val::Percent(90.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_items: FlexStart,
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(12.0),
                margin: UiRect::top(Val::Px(20.0)),
                ..default()
            },
        )).with_children(|grid| {
            for (idx, file) in ALL_CG_FILES.iter().enumerate() {
                if unlock_state.cg_unlocked.contains(*file) {
                    let path = format!("images/ev/{}", file);
                    let handle = cache.cache.entry(path.clone())
                        .or_insert_with(|| asset_server.load(&path))
                        .clone();
                    grid.spawn((
                        GalleryThumbnail(file.to_string()),
                        GalleryScreen,
                        Button,
                        Node {
                            width: Val::Px(360.0),
                            height: Val::Px(200.0),
                            ..default()
                        },
                        ImageNode::new(handle),
                        ZIndex(5),
                    ));
                } else {
                    grid.spawn((
                        GalleryThumbnail(file.to_string()),
                        GalleryLocked,
                        GalleryScreen,
                        Node {
                            width: Val::Px(360.0),
                            height: Val::Px(200.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 1.0)),
                        ZIndex(5),
                    )).with_child((
                        GalleryScreen,
                        Text::new("🔒"),
                        TextFont { font_size: 32.0, ..default() },
                        TextColor(Color::srgb(0.3, 0.3, 0.4)),
                    ));
                }
            }
        });
    });
}

fn handle_thumbnail_click(
    mut interaction_query: Query<(&Interaction, &GalleryThumbnail), (Changed<Interaction>, With<Button>)>,
    mut gallery_state: ResMut<GalleryState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<TextureCache>,
    unlock_state: Res<UnlockState>,
) {
    for (interaction, thumbnail) in &interaction_query {
        if *interaction == Interaction::Pressed {
            if gallery_state.fullscreen.is_some() {
                return;
            }
            let file = &thumbnail.0;
            if unlock_state.cg_unlocked.contains(file) {
                gallery_state.fullscreen = Some(file.clone());
                let path = format!("images/ev/{}", file);
                let handle = cache.cache.entry(path.clone())
                    .or_insert_with(|| asset_server.load(&path))
                    .clone();
                commands.spawn((
                    GalleryFullscreen,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        ..default()
                    },
                    ImageNode::new(handle),
                    BackgroundColor(Color::BLACK),
                    Button,
                    ZIndex(6),
                ));
            }
        }
    }
}

fn handle_back_button(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<GalleryBackButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

fn handle_fullscreen_click(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<GalleryFullscreen>)>,
    mut gallery_state: ResMut<GalleryState>,
    mut commands: Commands,
    fullscreen_query: Query<Entity, With<GalleryFullscreen>>,
) {
    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            for entity in &fullscreen_query {
                commands.entity(entity).despawn();
            }
            gallery_state.fullscreen = None;
        }
    }
}

fn handle_gallery_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut gallery_state: ResMut<GalleryState>,
    mut commands: Commands,
    fullscreen_query: Query<Entity, With<GalleryFullscreen>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if gallery_state.fullscreen.is_some() {
            for entity in &fullscreen_query {
                commands.entity(entity).despawn();
            }
            gallery_state.fullscreen = None;
        } else {
            next_state.set(AppState::Menu);
        }
    }
}

fn cleanup_gallery(mut commands: Commands, query: Query<Entity, Or<(With<GalleryRoot>, With<GalleryFullscreen>, With<GalleryScreen>)>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
```

- [ ] **Step 2: cargo check**

Run: `cargo check`
Expected: succeeds

- [ ] **Step 3: Commit**

---

### Task 5: Build and smoke test

**Files:**
- No file changes

- [ ] **Step 1: cargo build**

Run: `cargo build`
Expected: succeeds

- [ ] **Step 2: Run smoke test**

Run: `timeout 8 ./target/debug/bevy-vn`
Expected: launches without panic, gallery accessible via Menu

- [ ] **Step 3: Final commit**

---

## Self-Review Checklist

- [ ] Spec coverage: AffectionCondition implemented (Task 3), UnlockCg implemented (Task 1 + 3), auto-unlock on ShowCg (Task 3), gallery grid (Task 4), fullscreen viewer (Task 4), back button (Task 4)
- [ ] No placeholders: all code blocks contain actual implementation
- [ ] Type consistency: `GalleryThumbnail(String)`, `GalleryState.fullscreen: Option<String>`, `UnlockState.cg_unlocked: HashSet<String>` all use consistent file path strings
