# AnimateSprite (帧动画系统) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the AnimateSprite frame animation system matching Artemis engine's `AnimateSprite` tag

**Architecture:** New `AnimatedSprite` component on existing DrawSprite-style entities, driven by `handle_animate_sprite` handler and `advance_animated_sprites` per-frame system. Reuses `SpriteOverlayManager`, `TextureCache`, `SpriteAnchor`, `SpriteBlendMode`.

**Tech Stack:** Rust, Bevy ECS, bevy_simple_messages

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src/script.rs:220` | Add `AnimateSprite` variant to `ScriptCmd` enum |
| `src/rendering_messages.rs:97` | Add `AnimateSpriteMessage` struct |
| `src/components.rs:211` | Add `AnimatedSprite` component |
| `src/plugins/rendering.rs:91` | Register message + add systems; add `handle_animate_sprite` handler and `advance_animated_sprites` system |
| `src/plugins/script_runner.rs:645` | Handle AnimateSprite in both normal and skip modes |
| `tools/artemis-export/src/mapper.rs` | Add AnimateSprite tag mapping |

---

### Task 1: Add ScriptCmd::AnimateSprite variant

**Files:**
- Modify: `src/script.rs:209` (before View variant)

- [ ] **Step 1: Add the variant**

Insert before the `View` variant at line 209:

```rust
    AnimateSprite {
        id: String,
        file: String,
        max: u32,
        frame_time: u64,
        style: u32,
        x: f32,
        y: f32,
        z: i32,
        anchor_x: f32,
        anchor_y: f32,
        rotation: f32,
        draw: u32,
        alpha: i32,
        priority: i32,
        wait: bool,
    },
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check 2>&1 | head -20`
Expected: success, no errors

- [ ] **Step 3: Commit**

```
git add src/script.rs
git commit -m "feat: add AnimateSprite ScriptCmd variant"
```

---

### Task 2: Add AnimateSpriteMessage and AnimatedSprite component

**Files:**
- Modify: `src/rendering_messages.rs:96` (after MoveSpriteMessage)
- Modify: `src/components.rs:211` (after BgScroll)

- [ ] **Step 1: Add AnimateSpriteMessage**

Append to `src/rendering_messages.rs`:

```rust
#[derive(Message)]
pub struct AnimateSpriteMessage {
    pub id: String,
    pub file: String,
    pub max: u32,
    pub frame_time: u64,
    pub style: u32,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub rotation: f32,
    pub draw: u32,
    pub alpha: i32,
    pub priority: i32,
    pub wait: bool,
}
```

- [ ] **Step 2: Add AnimatedSprite component**

Append to `src/components.rs` after `BgScroll`:

```rust
#[derive(Component)]
pub struct AnimatedSprite {
    pub frames: Vec<Handle<Image>>,
    pub current_frame: usize,
    pub timer: Timer,
    pub max_frames: usize,
    pub finished: bool,
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check 2>&1 | head -20`
Expected: success, no errors

- [ ] **Step 4: Commit**

```
git add src/rendering_messages.rs src/components.rs
git commit -m "feat: add AnimateSpriteMessage and AnimatedSprite component"
```

---

### Task 3: Register message + add rendering handler and animation system

**Files:**
- Modify: `src/plugins/rendering.rs`

- [ ] **Step 1: Import AnimateSpriteMessage**

Add to the imports at line 9:

```rust
    AnimateSpriteMessage,
```

Change from:

```rust
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowFaceMessage, HideFaceMessage,
    ShowCgMessage, HideCgMessage, ScrollBgMessage,
    DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage,
};
```

To:

```rust
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowFaceMessage, HideFaceMessage,
    ShowCgMessage, HideCgMessage, ScrollBgMessage,
    AnimateSpriteMessage, DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage,
};
```

- [ ] **Step 2: Register message and add systems in plugin build**

In `src/plugins/rendering.rs`, add after `MoveSpriteMessage` registration (line 58):

```rust
            .add_message::<AnimateSpriteMessage>()
```

In the Update system set (after `handle_scroll_bg`, around line 89), add before the closing `)))`:

```rust
                handle_animate_sprite,
                advance_animated_sprites,
```

The Update block should now look like:

```rust
            .add_systems(Update, (
                handle_show_face,
                handle_hide_face,
                handle_show_cg,
                handle_hide_cg,
                handle_draw_sprite,
                handle_fade_sprite,
                handle_move_sprite,
                update_sprite_tweens,
                center_sprite_overlays,
                update_overlay_tween,
                quake_update,
                update_bg_scroll,
                handle_scroll_bg,
                handle_animate_sprite,
                advance_animated_sprites,
            ).chain().run_if(in_state(AppState::Gameplay)));
```

- [ ] **Step 3: Add `handle_animate_sprite` function**

Add before the closing `}` of the module (before `fn update_text_reveal` etc., around line 890). The exact location is after `handle_scroll_bg` and before `update_text_reveal` (which is in script_runner.rs — actually just before the final `}` of `rendering.rs`).

Add at the end of `src/plugins/rendering.rs` (before the final closing `}`):

```rust
fn handle_animate_sprite(
    mut msg: MessageReader<AnimateSpriteMessage>,
    mut overlay_mgr: ResMut<SpriteOverlayManager>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    obj_index: Res<ObjFileIndex>,
) {
    for msg in msg.read() {
        if let Some(&entity) = overlay_mgr.sprites.get(&msg.id) {
            commands.entity(entity).despawn();
            overlay_mgr.sprites.remove(&msg.id);
        }

        let blend = match msg.draw {
            1 => SpriteBlendMode::Add,
            3 => SpriteBlendMode::Multiply,
            4 => SpriteBlendMode::Screen,
            _ => SpriteBlendMode::Normal,
        };

        let alpha = (msg.alpha as f32 / 255.0).clamp(0.0, 1.0);
        let scale = sprite_depth_scale(msg.z);
        let rot_rad = msg.rotation.to_radians();

        let mut frames = Vec::with_capacity(msg.max as usize);
        for i in 0..msg.max {
            let path = format!("images/anime/{}_{:02}.png", msg.file, i + 1);
            let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
                asset_server.load(&path)
            }).clone();
            frames.push(handle);
        }

        let frame_secs = (msg.frame_time as f32 / 1000.0).max(0.016);
        let timer = Timer::from_seconds(frame_secs, TimerMode::Repeating);

        let entity = commands.spawn((
            SpriteOverlay { id: msg.id.clone(), blend_mode: blend },
            Node {
                width: Val::Auto,
                height: Val::Auto,
                position_type: PositionType::Absolute,
                left: Val::Px(msg.x),
                top: Val::Px(msg.y),
                ..default()
            },
            ImageNode {
                image: frames[0].clone(),
                color: Color::srgba(1.0, 1.0, 1.0, alpha),
                ..default()
            },
            SpriteAnchor {
                anchor_x: msg.anchor_x,
                anchor_y: msg.anchor_y,
                target_x: msg.x,
                target_y: msg.y,
            },
            Transform::from_scale(Vec3::splat(scale)).with_rotation(Quat::from_rotation_z(rot_rad)),
            Visibility::Visible,
            ZIndex((1 + msg.priority.max(0) as i32).min(2)),
            AnimatedSprite {
                frames,
                current_frame: 0,
                timer,
                max_frames: msg.max as usize,
                finished: false,
            },
        )).id();
        overlay_mgr.sprites.insert(msg.id.clone(), entity);
    }
}

fn advance_animated_sprites(
    time: Res<Time>,
    mut query: Query<(Entity, &mut AnimatedSprite, &mut ImageNode)>,
    mut commands: Commands,
) {
    for (entity, mut anim, mut image) in query.iter_mut() {
        if anim.finished || anim.max_frames <= 1 {
            continue;
        }

        anim.timer.tick(time.delta());
        while anim.timer.just_finished() && !anim.finished {
            anim.current_frame += 1;
            if anim.current_frame >= anim.max_frames {
                anim.finished = true;
            } else {
                image.image = anim.frames[anim.current_frame].clone();
            }
        }
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: success, no errors

- [ ] **Step 5: Commit**

```
git add src/plugins/rendering.rs
git commit -m "feat: add AnimateSprite rendering handler and frame animation system"
```

---

### Task 4: Add script runner handling

**Files:**
- Modify: `src/plugins/script_runner.rs`

- [ ] **Step 1: Import AnimateSpriteMessage**

Add to the rendering_messages import at line 20:

```rust
    AnimateSpriteMessage, DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage, ScrollBgMessage,
```

Change from:

```rust
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage,
    ShowFaceMessage, HideFaceMessage,
    ShowCgMessage, HideCgMessage,
    DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage, ScrollBgMessage,
};
```

To:

```rust
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage,
    ShowFaceMessage, HideFaceMessage,
    ShowCgMessage, HideCgMessage,
    AnimateSpriteMessage, DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage, ScrollBgMessage,
};
```

- [ ] **Step 2: Add animate_sprite_writer to ProcessAdvanceParams**

Add after `scroll_bg_writer` in `ProcessAdvanceParams` (line 65):

```rust
    animate_sprite_writer: MessageWriter<'w, 's, AnimateSpriteMessage>,
```

And in the destructuring block (around line 168), add:

```rust
        ref mut animate_sprite_writer,
```

- [ ] **Step 3: Add skip-mode handler**

In the skip mode match block, after `ScrollBg` handler (around line 400), add:

```rust
                    Some(ScriptCmd::AnimateSprite { id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, .. }) => {
                        animate_sprite_writer.write(AnimateSpriteMessage { id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, wait: false });
                    }
```

- [ ] **Step 4: Add normal-mode handler**

In the normal mode match block, after `ScrollBg` handler (around line 645), add:

```rust
                Some(ScriptCmd::AnimateSprite { id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, wait }) => {
                    animate_sprite_writer.write(AnimateSpriteMessage { id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, wait });
                    if wait {
                        let total_ms = max as u64 * frame_time;
                        auto_skip.auto_timer = Some(Timer::from_seconds(total_ms as f32 / 1000.0, TimerMode::Once));
                        break;
                    }
                }
```

- [ ] **Step 5: Add catch-all handler**

In the skip mode `_ =>` catch-all (around line 471), the existing match already catches unhandled variants. No change needed since `AnimateSprite` is explicitly handled above.

In normal mode, the existing `_ =>` catch-all (line 787) also already exists. No change needed.

- [ ] **Step 6: Verify compilation**

Run: `cargo check 2>&1 | head -30`
Expected: success, no errors

- [ ] **Step 7: Commit**

```
git add src/plugins/script_runner.rs
git commit -m "feat: add AnimateSprite handler in script runner"
```

---

### Task 5: Add mapper entry

**Files:**
- Modify: `tools/artemis-export/src/mapper.rs`

- [ ] **Step 1: Add AnimateSprite mapping**

Find the match arm for `"DrawSprite"` or similar tag in `map_command` function. Add before or after existing sprite-related entries:

```rust
            "AnimateSprite" => {
                let id = attrs.get("0").cloned().unwrap_or_default();
                let file = attrs.get("1").cloned().unwrap_or_default();
                let max: u32 = attrs.get("2").and_then(|v| v.parse().ok()).unwrap_or(1);
                let frame_time: u64 = attrs.get("3").and_then(|v| v.parse().ok()).unwrap_or(200);
                let style: u32 = attrs.get("4").and_then(|v| v.parse().ok()).unwrap_or(0);
                let x: f32 = attrs.get("6").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let y: f32 = attrs.get("7").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let z: i32 = attrs.get("8").and_then(|v| v.parse().ok()).unwrap_or(0);
                let anchor_x: f32 = attrs.get("9").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let anchor_y: f32 = attrs.get("10").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let rotation: f32 = attrs.get("11").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let draw: u32 = attrs.get("14").and_then(|v| v.parse().ok()).unwrap_or(0);
                let alpha: i32 = attrs.get("15").and_then(|v| v.parse().ok()).unwrap_or(255);
                let priority: i32 = attrs.get("16").and_then(|v| v.parse().ok()).unwrap_or(0);
                let wait: bool = attrs.get("18").map_or(false, |v| v == "1");
                Some(vec![ScriptCmd::AnimateSprite { id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, wait }])
            }
```

- [ ] **Step 2: Verify mapper compilation**

Run: `cargo check -p artemis-export 2>&1 | head -20`
Expected: success, no errors

- [ ] **Step 3: Commit**

```
git add tools/artemis-export/src/mapper.rs
git commit -m "feat: add AnimateSprite tag mapping in artemis-export mapper"
```

---

### Task 6: Full compilation verification

- [ ] **Step 1: Full check**

Run: `cargo check 2>&1`
Expected: success, 0 errors

- [ ] **Step 2: Run tests**

Run: `cargo test 2>&1`
Expected: all tests pass

- [ ] **Step 3: Mapper tests**

Run: `cargo test -p artemis-export 2>&1 | tail -20`
Expected: all tests pass

- [ ] **Step 4: Commit any final fixes**

```
git add -A
git commit -m "fix: address compilation warnings and test failures"
```
