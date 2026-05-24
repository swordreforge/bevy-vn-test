# Phase 3: Dialogue + Character Sprites + Backgrounds + CG

**Goal:** Render background images, character sprites (FG), and CG/event images on screen, reacting to `ScriptCmd` during gameplay. Window resolution 1280x720 to match native Artemis asset dimensions.

**Architecture:** A new `RenderingPlugin` owns all visual rendering state. `ScriptRunner` sends rendering `Message`s (Bevy 0.18) instead of no-op logging. Rendering systems react in `Update`. Sprites use a 3-slot pool (Left/Center/Right). Background uses a dual-buffer system (two entities with `ImageNode` + `Node`) for future transition support. CG uses a dedicated overlay entity. Texture loading is on-demand with a `TextureCache` resource.

**Rendering approach:** All visual elements (bg, sprites, cg) are UI `Node`-based with `position_type: PositionType::Absolute`. This keeps everything in screen-space coordinates, consistent with the existing dialogue UI, without needing a separate 2D camera. Z-order is determined by entity spawn order (bg → sprites → cg → dialogue UI).

**Window:** `1280x720` to match native Artemis asset resolution. Set via `WindowPlugin` configuration.

---

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Sprite management | Pooled entities (3 slots) | No spawn/despawn churn, clean position mapping |
| Texture loading | On-demand + cache | Simple setup, no preload delay, cache for reuse |
| Background | Dual-buffer | Enables cross-fade transitions (Phase 7) without refactoring |
| CG | Overlay entity | Separate lifecycle from bg, easy show/hide |
| Transitions | Instant (Phase 3) | Transitions deferred to Phase 7 |
| Communication | Messages (SetBgMessage, ShowFgMessage, etc.) | Decouples ScriptRunner from rendering |
| Asset path | `assets/images/{bg,fg,ev}/...` | Embedded copy of relevant Artemis assets |
| Z-order | bg → sprites → cg → dialogue UI | Standard VN layering |

---

### New Resources

```rust
/// Tracks background state with dual-buffer entities for future cross-fade
#[derive(Resource)]
pub struct BgState {
    pub current: Handle<Image>,
    pub previous: Handle<Image>,
    pub entities: [Entity; 2],     // [active, inactive]
    pub active_idx: usize,          // 0 or 1 — which entity is currently visible
}

/// Tracks which character sprite is in each position slot
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

/// On-demand texture cache: asset_path → Handle<Image>
#[derive(Resource, Default)]
pub struct TextureCache {
    pub cache: HashMap<String, Handle<Image>>,
}
```

### New Components

```rust
#[derive(Component)]
pub struct BackgroundRoot;

#[derive(Component)]
pub struct SpriteSlotMarker(pub FgPosition);

#[derive(Component)]
pub struct CgRoot;
```

### New Messages

```rust
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

---

### RenderingPlugin Systems

| System | Trigger | What it does |
|--------|---------|-------------|
| `setup_rendering` | `OnEnter(Gameplay)` | Spawn bg dual-buffer entities, 3 sprite pool entities, init resources |
| `handle_set_bg` | `Update` (msg) | Load texture, assign to active bg entity, swap buffers |
| `handle_show_fg` | `Update` (msg) | Load texture, assign to pooled slot for position, update SpriteManager |
| `handle_hide_fg` | `Update` (msg) | Clear texture from pooled slot, remove from SpriteManager |
| `handle_show_cg` | `Update` (msg) | Spawn CG overlay entity, load texture |
| `handle_hide_cg` | `Update` (msg) | Despawn CG overlay entity |
| `cleanup_rendering` | `OnExit(Gameplay)` | Despawn all rendering entities |

### ScriptRunner Changes

`process_advance` match arms for `SetBg`, `ShowFg`, `HideFg`, `ShowCg`, `HideCg` now write the corresponding message instead of logging no-op.

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
```

### Sprite Positioning

Given 1280x720 window, sprites are 780x720 PNG:

| Position | Anchor X | Notes |
|----------|----------|-------|
| `Left` | 0 | Left-aligned (character enters from left) |
| `Center` | 250 | Center-aligned (character in focus) |
| `Right` | 500 | Right-aligned (character enters from right) |
| `OffScreen` | -780 (or 1280) | Off-screen (used for slide-in transitions later) |

Sprites use `Node { position_type: PositionType::Absolute, left: Val::Px(x), .. }` with an `ImageNode` child (or `ImageNode` directly on the node). In Bevy's UI system, the `Node` provides the layout, and `ImageNode` renders the texture within it. When using `position_type: Absolute`, the node's position is set via `left`/`top` fields. The node should have explicit `width`/`height` matching the texture dimensions (780x720), or use `Val::Auto` with `image_node.size()` to match the texture size automatically.

Z-order via entity spawn order (bg → sprites → cg → dialogue UI).

### Asset Structure

```
assets/images/
├── bg/
│   └── bg_0000.jpg      (copied from game-source)
├── fg/
│   └── 001_eus/
│       ├── tati_010003.png
│       └── ...           (a few expressions for dev)
└── ev/
    └── eve_010101.png    (a sample CG for dev)
```

Phase 3 only needs 1 background, 1 character (a few expressions), and 1 CG for development.

---

### Files to Create/Modify

- **Create**: `src/plugins/rendering.rs`
- **Create**: `src/rendering_messages.rs`
- **Modify**: `src/plugins/script_runner.rs` — send messages instead of no-op
- **Modify**: `src/components.rs` — add BackgroundRoot, SpriteSlotMarker, CgRoot
- **Modify**: `src/resources.rs` — add BgState, SpriteManager, CgState, TextureCache
- **Modify**: `src/plugins/mod.rs` — add `pub mod rendering;`
- **Modify**: `src/main.rs` — register RenderingPlugin
- **Create/Modify**: `assets/scripts/test.bscript.ron` — add SetBg, ShowFg, ShowCg commands
- **Create**: `assets/images/bg/bg_0000.jpg` (copy from game-source)
- **Create**: `assets/images/fg/001_eus/tati_010003.png` and a few expressions
- **Create**: `assets/images/ev/eve_010101.png` (copy from game-source)
- **Modify**: `PROGRESS.md` — mark Phase 3 items

---

### Dependencies

- Bevy 0.18 built-in: `Image` asset type, `ImageNode` component, `AssetServer`, `Assets<Image>`
- No new crate dependencies
- `bevy::core::Name` for debug-friendly entity names
