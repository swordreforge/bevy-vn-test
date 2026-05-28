# View Mask UiMaterial 方案 B — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current alpha fade-in of View name cards with a proper mask transition using `UiMaterial` + WGSL fragment shader, matching the original Artemis `trans(e, {fade=reveal_time, rule=mask})` behavior.

**Architecture:** New `ViewMaskMaterial` struct implementing `UiMaterial` in Bevy 0.18, taking the name card texture, the rule mask texture (view_mask01/02/03), and a progress uniform. The fragment shader compares per-pixel mask values against a threshold driven by progress, producing a dissolve-style reveal. The existing `ViewState` state machine's `RevealName` phase drives the progress timer.

**Tech Stack:** Bevy 0.18 `UiMaterial` (via `bevy::ui_render`, feature `"bevy_ui_render"`), WGSL fragment shader, `AsBindGroup` derive, `MaterialNode<ViewMaskMaterial>` component

**Assets:**
- `assets/images/rule/view_mask01.png` (1280×720, grayscale rule mask)
- `assets/images/rule/view_mask02.png`
- `assets/images/rule/view_mask03.png`

**View entry mask mapping:**
| ViewEntry.mask_file | Shader mask texture |
|---|---|
| `view_mask01` | `images/rule/view_mask01.png` |
| `view_mask02` | `images/rule/view_mask02.png` |
| `view_mask03` | `images/rule/view_mask03.png` |

---

### Task 1: Create WGSL fragment shader

**Files:**
- Create: `assets/shaders/view_mask.wgsl`

- [ ] **Step 1: Write the shader**

```wgsl
#define_import_path bevy_vn::view_mask

#import bevy_ui::ui_vertex_output UiVertexOutput

@group(1) @binding(0)
var name_texture: texture_2d<f32>;
@group(1) @binding(1)
var name_sampler: sampler;
@group(1) @binding(2)
var mask_texture: texture_2d<f32>;
@group(1) @binding(3)
var mask_sampler: sampler;
@group(1) @binding(4)
var<uniform> progress: f32;
@group(1) @binding(5)
var<uniform> name_left: f32;
@group(1) @binding(6)
var<uniform> name_top: f32;

const SCREEN_W: f32 = 1280.0;
const SCREEN_H: f32 = 720.0;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let name_color = textureSample(name_texture, name_sampler, in.uv);
    if name_color.a < 0.01 {
        return vec4<f32>(0.0);
    }
    let mask_uv = vec2<f32>(
        (name_left + in.uv.x * in.size.x) / SCREEN_W,
        (name_top + in.uv.y * in.size.y) / SCREEN_H,
    );
    let mask_val = textureSample(mask_texture, mask_sampler, mask_uv).r;
    let threshold = 1.0 - progress;
    let opacity = smoothstep(threshold - 0.01, threshold + 0.01, mask_val);
    return vec4<f32>(name_color.rgb, name_color.a * opacity);
}
```

Note: Bevy UI uses Y-down texture coordinates (same as screen), so no UV flip needed. The mask is a full-screen 1280×720 image; `in.uv` maps within the node's rect, and `in.size` gives the node's pixel dimensions.

The three separate `#[uniform(4..6)]` fields in the Rust `AsBindGroup` derive map to three separate `var<uniform>` WGSL variables at the corresponding binding indices.

- [ ] **Step 2: Verify syntax**

The shader compiles at GPU pipeline creation time. No pre-check possible, but the structure follows Bevy 0.18's UiMaterial WGSL convention exactly (matches `bevy_ui::ui_material.wgsl` pattern).

- [ ] **Step 3: Commit**

```bash
git add assets/shaders/view_mask.wgsl
git commit -m "feat: add ViewMaskMaterial WGSL fragment shader for rule mask transition"
```

---

### Task 2: Create ViewMaskMaterial Rust type

**Files:**
- Create: `src/plugins/event_system/view_material.rs`

- [ ] **Step 1: Add `"bevy_ui_render"` to bevy features in Cargo.toml**

```toml
bevy = { version = "0.18", features = ["bevy_asset", "bevy_audio", "vorbis", "jpeg", "bevy_ui_render"] }
```

- [ ] **Step 2: Write the material struct and UiMaterial impl**

```rust
use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::ui_render::prelude::*;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ViewMaskMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub name_texture: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub mask_texture: Handle<Image>,
    #[uniform(4)]
    pub progress: f32,
    #[uniform(5)]
    pub name_left: f32,
    #[uniform(6)]
    pub name_top: f32,
}

impl UiMaterial for ViewMaskMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/view_mask.wgsl".into()
    }
}
```

- [ ] **Step 2: Add `pub mod view_material;` to event_system/mod.rs**

```rust
pub mod view_material;
pub mod view;
pub mod view_data;
```

- [ ] **Step 3: Register the material and plugin in event_system/mod.rs**

```rust
use bevy::ui_render::UiMaterialPlugin;
use view_material::ViewMaskMaterial;

pub struct EventSystemPlugin;

impl Plugin for EventSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiMaterialPlugin::<ViewMaskMaterial>::default())
            .add_plugins(view::ViewPlugin);
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add src/plugins/event_system/view_material.rs src/plugins/event_system/mod.rs
git commit -m "feat: add ViewMaskMaterial with UiMaterial impl for rule mask transition"
```

---

### Task 3: Update ViewState to hold material handle and progress

**Files:**
- Modify: `src/plugins/event_system/view.rs`

- [ ] **Step 1: Add fields to ViewState for mask material**

```rust
#[derive(Component)]
pub struct ViewState {
    pub char_id: String,
    pub phase: ViewPhase,
    pub timer: Timer,
    pub step_idx: usize,
    pub pen_entity: Option<Entity>,
    pub name_entity: Option<Entity>,
    pub scene_entities: Vec<Entity>,
    pub entry: &'static ViewEntry,
    pub tween_entry: &'static ViewTweenEntry,
    pub mask_material: Option<Handle<ViewMaskMaterial>>,
}
```

- [ ] **Step 2: Update RevealName phase to use mask material**

Replace the alpha fade approach in `RevealName`:

```rust
ViewPhase::RevealName => {
    view.timer.tick(time.delta());
    let progress = (view.timer.elapsed_secs() / view.timer.duration().as_secs_f32()).min(1.0);
    if let Some(mat) = &view.mask_material {
        if let Some(mut materials) = materials.get_mut(mat) {
            materials.progress = progress;
        }
    }
    if view.timer.just_finished() {
        view.phase = ViewPhase::DisplayWait;
        view.timer = Timer::from_seconds(1.0, TimerMode::Once);
    }
}
```

The function signature of `advance_view` needs new system params:

```rust
fn advance_view(
    mut commands: Commands,
    time: Res<Time>,
    mut view_query: Query<(Entity, &mut ViewState)>,
    mut overlay_query: Query<(
        Entity,
        &mut BackgroundColor,
        &mut Visibility,
    ), With<crate::components::ScreenOverlayRoot>>,
    asset_server: Res<AssetServer>,
    mut view_blocking: ResMut<ViewBlocking>,
    mut materials: ResMut<Assets<ViewMaskMaterial>>,
) {
```

- [ ] **Step 3: Update PrepareScene to create ViewMaskMaterial instead of ImageNode for name card**

Replace the ImageNode spawn for the name card:

```rust
// Before (current):
let name = commands.spawn((
    ImageNode {
        image: asset_server.load(format!("{}{}.png", prefix, entry.name_file)),
        color: Color::srgba(1.0, 1.0, 1.0, 0.0),
        ..default()
    },
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(name_x),
        top: Val::Px(393.0),
        ..default()
    },
    ViewSprite,
)).id();

// After:
let name_texture: Handle<Image> = asset_server.load(format!("{}{}.png", prefix, entry.name_file));
let mask_path = format!("images/rule/{}.png", entry.mask_file);
let mask_texture: Handle<Image> = asset_server.load(&mask_path);
let mat_handle = materials.add(ViewMaskMaterial {
    name_texture,
    mask_texture,
    progress: 0.0,
    name_left: name_x as f32,
    name_top: 393.0,
});
view.mask_material = Some(mat_handle.clone());

let name = commands.spawn((
    MaterialNode(mat_handle),
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(name_x),
        top: Val::Px(393.0),
        ..default()
    },
    ViewSprite,
)).id();
```

Note: The name card image is no longer needed as `ImageNode` since the material's `name_texture` field holds it. `MaterialNode` replaces `ImageNode` entirely.

- [ ] **Step 4: Commit**

```bash
git add src/plugins/event_system/view.rs
git commit -m "feat: integrate ViewMaskMaterial into View state machine for rule mask transition"
```

---

### Task 4: Full compilation verification

- [ ] **Step 1: Run cargo check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors

- [ ] **Step 2: Fix any compilation issues**

Common issues:
- Missing `use` imports for `MaterialNode`, `UiMaterialPlugin`, `ViewMaskMaterial`
- Wrong binding indices in shader vs AsBindGroup
- Missing `ResMut<Assets<ViewMaskMaterial>>` in system params

- [ ] **Step 3: Commit any fixes**

```bash
git commit -am "fix: compilation fixes for ViewMaskMaterial integration"
```

---

### Task 5: Verify mask texture loading

- [ ] **Step 1: Run the game**

```bash
cargo run
```

Navigate to a View sequence (e.g., first character encounter).

- [ ] **Step 2: Verify mask transition**

Observe the name card reveal phase. Instead of a simple alpha fade, the name card should appear following the pattern defined by the rule mask texture (view_mask01/02/03 depending on character).

Expected visual: The name card is revealed progressively following the mask pattern (e.g., horizontal wipe for view_mask02, radial wipe for view_mask01, etc.)

- [ ] **Step 3: Fix any visual issues**

If the mask appears incorrect (e.g., wrong orientation, no effect, wrong texture coordinates), adjust:
- `(1.0 - in.uv.y)` flipping in shader (try removing it)
- `name_left` / `name_top` values (may need to account for UI transform)
- Threshold direction in shader

---

### Verification Checklist

| Check | Expected | Actual |
|-------|----------|--------|
| `cargo check` | No errors | |
| View name card revealed via mask | Smooth dissolve following mask pattern | |
| All 3 mask types work | view_mask01/02/03 all produce different effects | |
| ViewEnd with mask | Uses view_mask02 (default) | |
| Progress drives reveal | progress=0 → hidden, progress=1 → fully visible | |
| No regression on other View phases | FadeOut → PrepareScene → FadeIn → PenTween → PenWait all unchanged | |
| No regression on skip mode | Skip mode bypasses mask transition entirely | |
