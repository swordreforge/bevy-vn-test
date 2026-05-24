# Phase 3: Dialogue + Character Sprites + Backgrounds + CG Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render background images, character sprites (FG), and CG/event images on screen, driven by ScriptCmd during gameplay.

**Architecture:** New RenderingPlugin owning all visual state. ScriptRunner sends rendering Messages instead of no-op logging. Background uses dual-buffer entities for future cross-fade. Sprites use a 3-slot pooled entity approach (Left/Center/Right). CG is a dedicated overlay entity. Texture loading: on-demand via AssetServer, cached in TextureCache. Window: 1280x720.

**Tech Stack:** Rust, Bevy 0.18 (ImageNode, AssetServer, Node, ZIndex, Visibility, Display), RON.

---

### File Structure

| File | Action | Purpose |
|------|--------|---------|
| `src/rendering_messages.rs` | Create | Message types for rendering commands |
| `src/plugins/rendering.rs` | Create | RenderingPlugin (bg/sprite/cg systems) |
| `src/components.rs` | Modify | Add BackgroundRoot, SpriteSlotMarker, CgRoot |
| `src/resources.rs` | Modify | Add BgState, SpriteManager, CgState, TextureCache |
| `src/plugins/script_runner.rs` | Modify | Send rendering messages instead of no-op |
| `src/plugins/mod.rs` | Modify | Add rendering module |
| `src/main.rs` | Modify | Register RenderingPlugin, set window size |
| `assets/scripts/test.bscript.ron` | Modify | Add SetBg, ShowFg, ShowCg commands |
| `assets/images/bg/bg_0000.jpg` | Create | Sample background (copy from game-source) |
| `assets/images/fg/001_eus/tati_010003.png` | Create | Sample sprite (copy from game-source) |
| `assets/images/fg/001_eus/tati_010101.png` | Create | Sample sprite expression (copy from game-source) |
| `assets/images/ev/eve_010101.png` | Create | Sample CG (copy from game-source) |

---

### Task 1: Resources, Messages, and Components

**Files:**
- Create: `src/rendering_messages.rs`
- Modify: `src/components.rs`
- Modify: `src/resources.rs`

- [ ] **Step 1: Create rendering_messages.rs**

```rust
// src/rendering_messages.rs
use bevy::prelude::*;
use crate::script::FgPosition;

#[derive(Message)]
pub struct SetBgMessage {
    pub file: String,
}

#[derive(Message)]
pub struct ShowFgMessage {
    pub char_id: String,
    pub expression: String,
    pub position: FgPosition,
}

#[derive(Message)]
pub struct HideFgMessage {
    pub char_id: String,
}

#[derive(Message)]
pub struct ShowCgMessage {
    pub file: String,
}

#[derive(Message)]
pub struct HideCgMessage;
```

- [ ] **Step 2: Add rendering components to components.rs**

```rust
// Append to src/components.rs:

#[derive(Component)]
pub struct BackgroundRoot;

#[derive(Component)]
pub struct SpriteSlotMarker(pub FgPosition);

#[derive(Component)]
pub struct CgRoot;
```

- [ ] **Step 3: Add rendering resources to resources.rs**

```rust
// Append to src/resources.rs:

use bevy::render::view::Visibility;

/// Tracks background state with dual-buffer entities
#[derive(Resource)]
pub struct BgState {
    pub entities: [Entity; 2],
    pub active_idx: usize,
}

impl Default for BgState {
    fn default() -> Self {
        Self {
            entities: [Entity::PLACEHOLDER; 2],
            active_idx: 0,
        }
    }
}

/// Tracks which character sprite occupies each position slot
#[derive(Resource, Default)]
pub struct SpriteManager {
    pub slots: HashMap<FgPosition, SpriteSlotInfo>,
}

pub struct SpriteSlotInfo {
    pub char_id: String,
    pub expression: String,
    pub entity: Entity,
    pub texture: Option<Handle<Image>>,
}

/// Tracks CG overlay state
#[derive(Resource, Default)]
pub struct CgState {
    pub active: bool,
    pub entity: Option<Entity>,
    pub texture: Option<Handle<Image>>,
}

/// On-demand texture cache
#[derive(Resource, Default)]
pub struct TextureCache {
    pub cache: HashMap<String, Handle<Image>>,
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: compilation succeeds (new types exist but may have dead_code warnings since they aren't used yet).

---

### Task 2: RenderingPlugin — Setup & Cleanup

**Files:**
- Create: `src/plugins/rendering.rs`

- [ ] **Step 1: Create RenderingPlugin skeleton with setup and cleanup systems**

```rust
// src/plugins/rendering.rs
use bevy::prelude::*;
use bevy::render::view::Visibility;
use crate::components::*;
use crate::resources::{BgState, CgState, SpriteManager, TextureCache};
use crate::script::FgPosition;
use crate::state::AppState;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SetBgMessage>()
            .add_message::<ShowFgMessage>()
            .add_message::<HideFgMessage>()
            .add_message::<ShowCgMessage>()
            .add_message::<HideCgMessage>()
            .init_resource::<BgState>()
            .init_resource::<SpriteManager>()
            .init_resource::<CgState>()
            .init_resource::<TextureCache>()
            .add_systems(OnEnter(AppState::Gameplay), setup_rendering)
            .add_systems(OnExit(AppState::Gameplay), cleanup_rendering);
    }
}

use crate::rendering_messages::*;

fn setup_rendering(mut commands: Commands, mut bg_state: ResMut<BgState>, mut sprite_mgr: ResMut<SpriteManager>) {
    // Spawn dual-buffer background entities
    let bg_a = commands.spawn((
        BackgroundRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        Visibility::Visible,
        ZIndex(0),
    )).id();

    let bg_b = commands.spawn((
        BackgroundRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        Visibility::Hidden,
        ZIndex(0),
    )).id();

    bg_state.entities = [bg_a, bg_b];
    bg_state.active_idx = 0;

    // Spawn 3 pooled sprite entities (Left, Center, Right)
    let positions = [
        (FgPosition::Left, Val::Px(0.0)),
        (FgPosition::Center, Val::Px(250.0)),
        (FgPosition::Right, Val::Px(500.0)),
    ];

    for (pos, left_val) in &positions {
        let entity = commands.spawn((
            SpriteSlotMarker(pos.clone()),
            Node {
                width: Val::Px(780.0),
                height: Val::Px(720.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: *left_val,
                ..default()
            },
            Visibility::Hidden,
            ZIndex(1),
        )).id();

        sprite_mgr.slots.insert(pos.clone(), SpriteSlotInfo {
            char_id: String::new(),
            expression: String::new(),
            entity,
            texture: None,
        });
    }
}

fn cleanup_rendering(mut commands: Commands, query: Query<Entity, Or<(With<BackgroundRoot>, With<SpriteSlotMarker>, With<CgRoot>)>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
```

- [ ] **Step 2: Add all necessary imports at the top of rendering.rs**

The file needs these imports. Make sure all types are available:

```rust
use crate::rendering_messages::*;
```

And the rendering_messages module needs to be declared somewhere. We'll do that in mod.rs:

```rust
// src/plugins/mod.rs
pub mod rendering;
pub mod rendering_messages;
```

Wait — `rendering_messages` should be a top-level module, not under plugins. Let me fix:

```rust
// src/main.rs
mod rendering_messages;
```

- [ ] **Step 3: Add rendering module to mod.rs and main.rs**

In `src/plugins/mod.rs`, add:
```rust
pub mod rendering;
```

In `src/main.rs`, add after `mod events;`:
```rust
mod rendering_messages;
```

And register the plugin after `GalleryPlugin`:
```rust
use plugins::rendering::RenderingPlugin;

// in main():
        .add_plugins(RenderingPlugin)
```

Add imports for all the rendering messages in rendering.rs:
```rust
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowCgMessage, HideCgMessage,
};
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: compilation succeeds.

---

### Task 3: Background handling (SetBg)

**Files:**
- Modify: `src/plugins/rendering.rs`

- [ ] **Step 1: Add handle_set_bg system to RenderingPlugin**

```rust
fn handle_set_bg(
    mut msg: MessageReader<SetBgMessage>,
    mut bg_state: ResMut<BgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Visibility)>,
) {
    for msg in msg.read() {
        let path = format!("images/bg/{}", msg.file);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        // Inactive buffer gets the new texture
        let inactive_idx = 1 - bg_state.active_idx;
        let inactive_entity = bg_state.entities[inactive_idx];

        if let Ok((mut image_node, mut vis)) = query.get_mut(inactive_entity) {
            image_node.image = handle;
            *vis = Visibility::Visible;
        }

        // Hide the previously active buffer
        let active_entity = bg_state.entities[bg_state.active_idx];
        if let Ok((_, mut vis)) = query.get_mut(active_entity) {
            *vis = Visibility::Hidden;
        }

        // Swap active index
        bg_state.active_idx = inactive_idx;
    }
}
```

Register the system in `RenderingPlugin::build()`:
```rust
.add_systems(Update, handle_set_bg.run_if(in_state(AppState::Gameplay)))
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compilation succeeds.

---

### Task 4: Sprite handling (ShowFg / HideFg)

**Files:**
- Modify: `src/plugins/rendering.rs`

- [ ] **Step 1: Add sprite handling systems**

```rust
fn handle_show_fg(
    mut msg: MessageReader<ShowFgMessage>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Visibility)>,
) {
    for msg in msg.read() {
        let slot = sprite_mgr.slots.get_mut(&msg.position);
        if let Some(slot) = slot {
            let path = format!("images/fg/{}/tati_{}.png", msg.char_id, msg.expression);
            let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
                asset_server.load(&path)
            }).clone();

            slot.char_id = msg.char_id.clone();
            slot.expression = msg.expression.clone();
            slot.texture = Some(handle.clone());

            if let Ok((mut image_node, mut vis)) = query.get_mut(slot.entity) {
                image_node.image = handle;
                *vis = Visibility::Visible;
            }
        } else {
            warn!("No sprite slot for position: {:?}", msg.position);
        }
    }
}

fn handle_hide_fg(
    mut msg: MessageReader<HideFgMessage>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut query: Query<(&mut ImageNode, &mut Visibility)>,
) {
    for msg in msg.read() {
        // Find slot by char_id
        let slot = sprite_mgr.slots.values_mut()
            .find(|s| s.char_id == msg.char_id);

        if let Some(slot) = slot {
            slot.char_id.clear();
            slot.expression.clear();
            slot.texture = None;

            if let Ok((mut image_node, mut vis)) = query.get_mut(slot.entity) {
                image_node.image = Handle::default();
                *vis = Visibility::Hidden;
            }
        }
    }
}
```

Register both in `RenderingPlugin::build()`:
```rust
.add_systems(Update, (
    handle_set_bg,
    handle_show_fg,
    handle_hide_fg,
).run_if(in_state(AppState::Gameplay)))
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compilation succeeds.

---

### Task 5: CG handling (ShowCg / HideCg)

**Files:**
- Modify: `src/plugins/rendering.rs`

- [ ] **Step 1: Add CG handling systems**

```rust
fn handle_show_cg(
    mut msg: MessageReader<ShowCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in msg.read() {
        // Despawn existing CG if any
        if let Some(entity) = cg_state.entity.take() {
            commands.entity(entity).despawn();
        }

        let path = format!("images/ev/{}", msg.file);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        let entity = commands.spawn((
            CgRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                ..default()
            },
            ImageNode::new(handle.clone()),
            Visibility::Visible,
            ZIndex(2),
        )).id();

        cg_state.active = true;
        cg_state.entity = Some(entity);
        cg_state.texture = Some(handle);
    }
}

fn handle_hide_cg(
    mut msg: MessageReader<HideCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut commands: Commands,
) {
    for _ in msg.read() {
        if let Some(entity) = cg_state.entity.take() {
            commands.entity(entity).despawn();
        }
        cg_state.active = false;
        cg_state.texture = None;
    }
}
```

Register in `RenderingPlugin::build()`:
```rust
.add_systems(Update, (
    handle_set_bg,
    handle_show_fg,
    handle_hide_fg,
    handle_show_cg,
    handle_hide_cg,
).run_if(in_state(AppState::Gameplay)))
```

Also add `ImageNode` import to the bg and sprite setup in `setup_rendering`. The bg entities currently only have `BackgroundColor` — add `ImageNode::default()` to them too since `handle_set_bg` queries for `&mut ImageNode`:

Update the bg entity spawns in `setup_rendering`:
```rust
let bg_a = commands.spawn((
    BackgroundRoot,
    Node { ... },
    BackgroundColor(Color::BLACK),
    ImageNode::default(),   // <-- ADD THIS
    Visibility::Visible,
    ZIndex(0),
)).id();

let bg_b = commands.spawn((
    BackgroundRoot,
    Node { ... },
    BackgroundColor(Color::BLACK),
    ImageNode::default(),   // <-- ADD THIS
    Visibility::Hidden,
    ZIndex(0),
)).id();
```

Similarly add `ImageNode::default()` to the sprite pool entities in `setup_rendering` since `handle_show_fg` queries for `&mut ImageNode`:

```rust
let entity = commands.spawn((
    SpriteSlotMarker(pos.clone()),
    Node { ... },
    ImageNode::default(),   // <-- ADD THIS
    Visibility::Hidden,
    ZIndex(1),
)).id();
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compilation succeeds.

---

### Task 6: ScriptRunner integration — send rendering messages

**Files:**
- Modify: `src/plugins/script_runner.rs`

- [ ] **Step 1: Update process_advance to send rendering messages**

Add message writers to `process_advance` parameters:

```rust
fn process_advance(
    mut advance_ev: MessageReader<AdvanceEvent>,
    mut engine: ResMut<ScriptEngine>,
    mut dialogue: ResMut<DialogueState>,
    mut affection: ResMut<AffectionMap>,
    mut set_bg_writer: MessageWriter<SetBgMessage>,
    mut show_fg_writer: MessageWriter<ShowFgMessage>,
    mut hide_fg_writer: MessageWriter<HideFgMessage>,
    mut show_cg_writer: MessageWriter<ShowCgMessage>,
    mut hide_cg_writer: MessageWriter<HideCgMessage>,
) {
```

Add imports at top:
```rust
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowCgMessage, HideCgMessage,
};
```

Replace the `Some(cmd) => { info!("Script cmd (no-op): {:?}", cmd); }` catch-all with specific arms:

```rust
Some(ScriptCmd::SetBg { file, transition: _, duration: _ }) => {
    set_bg_writer.send(SetBgMessage { file });
}
Some(ScriptCmd::ShowFg { char_id, expression, position, transition: _ }) => {
    show_fg_writer.send(ShowFgMessage { char_id, expression, position });
}
Some(ScriptCmd::HideFg { char_id, transition: _ }) => {
    hide_fg_writer.send(HideFgMessage { char_id });
}
Some(ScriptCmd::ShowCg { file, transition: _ }) => {
    show_cg_writer.send(ShowCgMessage { file });
}
Some(ScriptCmd::HideCg { transition: _ }) => {
    hide_cg_writer.send(HideCgMessage);
}
// Keep no-op log for truly unsupported commands
Some(cmd) => {
    info!("Script cmd (no-op): {:?}", cmd);
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compilation succeeds.

---

### Task 7: Window config + update test script + copy assets

**Files:**
- Modify: `src/main.rs`
- Modify: `assets/scripts/test.bscript.ron`

- [ ] **Step 1: Set window size to 1280x720 in main.rs**

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (1280.0, 720.0).into(),
                title: "Bevy VN".to_string(),
                ..default()
            }),
            ..default()
        }))
        // ... rest of setup
```

- [ ] **Step 2: Update test.bscript.ron with visual commands**

```ron
[
    SetBg(file: "bg_0000.jpg", transition: None, duration: None),
    Dialogue(speaker: Some("ナユタ"), text: "目が覚めたか。"),
    ShowFg(char_id: "001_eus", expression: "010003", position: Left, transition: None),
    Dialogue(speaker: Some("ナユタ"), text: "長い間、眠っていたようだ。"),
    Dialogue(speaker: None, text: "ここはどこだろう。周りを見渡すが、見知らぬ場所だ。"),
    AffectionChange(char_id: "nayuta", delta: 1),
    ShowFg(char_id: "001_eus", expression: "010101", position: Center, transition: None),
    Dialogue(speaker: Some("ナユタ"), text: "お前のその目は、何もかもを見透かす——"),
    Dialogue(speaker: Some("ナユタ"), text: "そう信じている。"),
    ShowCg(file: "eve_010101.png", transition: None),
    Dialogue(speaker: None, text: "— CG表示 —"),
    HideCg(transition: None),
    Dialogue(speaker: None, text: "— Fin —"),
    Jump(target: "end"),
    Label(name: "end"),
]
```

- [ ] **Step 3: Copy sample assets from game-source**

```bash
mkdir -p assets/images/bg assets/images/fg/001_eus assets/images/ev assets/images/fg/002_eri
cp /home/swordreforge/Downloads/game-source/image/bg/bg_0000.jpg assets/images/bg/
cp /home/swordreforge/Downloads/game-source/image/fg/001_eus/tati_010003.png assets/images/fg/001_eus/
cp /home/swordreforge/Downloads/game-source/image/fg/001_eus/tati_010101.png assets/images/fg/001_eus/
cp /home/swordreforge/Downloads/game-source/image/ev/eve_010101.png assets/images/ev/
```

- [ ] **Step 4: Add ZIndex(3) to dialogue UI entities**

The dialogue UI entities (DialogueBox, SpeakerNameDisplay, DialogueTextDisplay) don't set ZIndex, defaulting to ZIndex(0). Since bg=0, sprites=1, cg=2, dialogue needs ZIndex(3) to render on top.

In `src/plugins/dialogue.rs`, add `ZIndex(3)` to all three entities:

```rust
// DialogueBox entity — add ZIndex(3):
commands.spawn((
    DialogueUiRoot,
    DialogueBox,
    Node { ... },
    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
    ZIndex(3),
));

// SpeakerNameDisplay — add ZIndex(3):
commands.spawn((
    DialogueUiRoot,
    SpeakerNameDisplay,
    Text::new(""),
    TextFont { font_size: 24.0, ..default() },
    TextColor(Color::srgb(1.0, 0.8, 0.6)),
    Node { ... },
    ZIndex(3),
));

// DialogueTextDisplay — add ZIndex(3):
commands.spawn((
    DialogueUiRoot,
    DialogueTextDisplay,
    Text::new(""),
    TextFont { font_size: 20.0, ..default() },
    TextColor(Color::WHITE),
    Node { width: Val::Percent(95.0), ..default() },
    ZIndex(3),
));
```

- [ ] **Step 5: Verify final compilation with warnings check**

Run: `cargo check 2>&1`
Expected: compilation succeeds. Warnings should only be pre-existing unused resources (Settings, UnlockState fields, etc.), not new ones.

---

### Task 8: Smoke test

**Files:** None (runtime verification)

- [ ] **Step 1: Run the game for 5 seconds to verify it starts**

Run: `timeout 5 cargo run 2>&1 || true`
Expected: Window opens at 1280x720. Title screen appears. Output should show script loading. Click to enter Gameplay — background should appear, then character sprite on click.

---

### Self-Review Checklist

- **Setup_rendering**: spawns 2 bg entities, 3 sprite pool entities?
- **SetBg**: loads texture via AssetServer, assigns to inactive buffer, swaps visibility?
- **ShowFg**: finds position slot, loads/retrieves texture from cache, assigns to entity?
- **HideFg**: finds slot by char_id, clears texture and hides entity?
- **ShowCg**: despawns existing CG, spawns new overlay with texture?
- **HideCg**: despawns CG overlay entity, clears state?
- **ScriptRunner**: sends all 5 message types correctly, no breaking changes?
- **Z-order**: bg=0, sprites=1, cg=2, dialogue UI spawned earlier/later? (DialoguePlugin is added before RenderingPlugin — dialogue UI spawns first but ZIndex(3) should be used for dialogue)
- **Edge: SetBg before any FG assets loaded** → AssetServer handles async loading gracefully
- **Edge: HideFg for non-existent char_id** → find() returns None, warning, no crash
- **Edge: ShowFg while CG active** → CG is separate layer, sprite renders behind CG (ZIndex 1 vs 2) — correct visual layering
- **Edge: Rapid ShowFg/HideFg** → TextureCache prevents redundant loads
