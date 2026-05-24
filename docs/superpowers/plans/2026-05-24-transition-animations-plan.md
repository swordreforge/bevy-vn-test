# Transition Animations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire cross-fade transitions for BG/FG/CG asset changes during gameplay and fade-to-black between Title↔Gameplay screens.

**Architecture:** Thread the existing `Transition` enum and duration fields from `ScriptCmd` through rendering messages into fade-aware handlers. Add per-type fade timers (BG dual-buffer cross-fade, FG/CG fade-in/out) with dedicated update systems. Add `ScreenTransition` resource + always-active system for state change fades.

**Tech Stack:** Bevy 0.18 ECS (Message/MessageReader/ResMut/Query/Commands), `Timer` for animation timing, `BackgroundColor` alpha for UI node opacity.

---

## File Structure

| File | Change |
|---|---|
| `src/rendering_messages.rs` | Add `transition`, `duration` fields to 5 message structs |
| `src/plugins/script_runner.rs` | Pass transition/duration through instead of discarding |
| `src/resources.rs` | Extend `BgState`, `SpriteSlotInfo`, `CgState` with fade state; add `ScreenTransition` resource |
| `src/plugins/rendering.rs` | Branch 5 handlers on transition type; add 3 update systems |
| `src/components.rs` | Add `TransitionOverlay` marker component |
| `src/plugins/screen_transition.rs` | NEW — `ScreenTransitionPlugin` + `handle_screen_transition` system |
| `src/plugins/title.rs` | Swap `NextState::set(Gameplay)` → `ScreenTransition.pending_state` |
| `src/plugins/menu.rs` | Swap `NextState::set(Title)` on Back to Title button |
| `src/main.rs` | Add `ScreenTransitionPlugin`, register update systems |

---

### Task 1: Thread transition data through messages and script runner

**Files:**
- Modify: `src/rendering_messages.rs`
- Modify: `src/plugins/script_runner.rs`

- [ ] **Step 1: Add transition/duration to rendering messages**

```rust
// src/rendering_messages.rs
use bevy::prelude::*;
use crate::script::{FgPosition, Transition};

#[derive(Message)]
pub struct SetBgMessage {
    pub file: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct ShowFgMessage {
    pub char_id: String,
    pub expression: String,
    pub position: FgPosition,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct HideFgMessage {
    pub char_id: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct ShowCgMessage {
    pub file: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct HideCgMessage {
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}
```

- [ ] **Step 2: Thread through script_runner.rs — normal mode (5 sites)**

Replace each `transition: _` / `duration: _` discard pattern with pass-through:

```rust
// Line ~300 — SetBg
Some(ScriptCmd::SetBg { file, transition, duration }) => {
    set_bg_writer.write(SetBgMessage { file, transition, duration });
}

// Line ~303 — ShowFg
Some(ScriptCmd::ShowFg { char_id, expression, position, transition, duration }) => {
    show_fg_writer.write(ShowFgMessage { char_id, expression, position, transition, duration });
}

// Line ~306 — HideFg
Some(ScriptCmd::HideFg { char_id, transition, duration }) => {
    hide_fg_writer.write(HideFgMessage { char_id, transition, duration });
}

// Line ~309 — ShowCg
Some(ScriptCmd::ShowCg { file, transition, duration }) => {
    show_cg_writer.write(ShowCgMessage { file: file.clone(), transition, duration });
    unlock_state.cg_unlocked.insert(file);
}

// Line ~313 — HideCg
Some(ScriptCmd::HideCg { transition, duration }) => {
    hide_cg_writer.write(HideCgMessage { transition, duration });
}
```

- [ ] **Step 3: Thread through script_runner.rs — skip mode (5 sites)**

Skip mode uses `transition: None` (no fade during skip):

```rust
// Line ~199 — SetBg (skip)
Some(ScriptCmd::SetBg { file, .. }) => {
    set_bg_writer.write(SetBgMessage { file, transition: None, duration: None });
}

// Line ~202 — ShowFg (skip)
Some(ScriptCmd::ShowFg { char_id, expression, position, .. }) => {
    show_fg_writer.write(ShowFgMessage { char_id, expression, position, transition: None, duration: None });
}

// Line ~205 — HideFg (skip)
Some(ScriptCmd::HideFg { char_id, .. }) => {
    hide_fg_writer.write(HideFgMessage { char_id, transition: None, duration: None });
}

// Line ~208 — ShowCg (skip)
Some(ScriptCmd::ShowCg { file, .. }) => {
    show_cg_writer.write(ShowCgMessage { file: file.clone(), transition: None, duration: None });
    unlock_state.cg_unlocked.insert(file);
}

// Line ~212 — HideCg (skip)
Some(ScriptCmd::HideCg { .. }) => {
    hide_cg_writer.write(HideCgMessage { transition: None, duration: None });
}
```

- [ ] **Step 4: cargo check**

Run: `cargo check`
Expected: 0 new errors (11 pre-existing warnings)

- [ ] **Step 5: Commit**

```bash
git add src/rendering_messages.rs src/plugins/script_runner.rs
git commit -m "feat: thread transition and duration through messages and script runner"
```

---

### Task 2: BG cross-fade

**Files:**
- Modify: `src/resources.rs`
- Modify: `src/plugins/rendering.rs`

- [ ] **Step 1: Extend BgState with fade timer**

Add after `BgState` in `src/resources.rs`:

```rust
pub struct BgCrossFade {
    pub timer: Timer,
}
```

Extend `BgState`:

```rust
#[derive(Resource)]
pub struct BgState {
    pub entities: [Entity; 2],
    pub active_idx: usize,
    pub fade: Option<BgCrossFade>,
}
```

Update `Default` impl:

```rust
impl Default for BgState {
    fn default() -> Self {
        Self {
            entities: [Entity::PLACEHOLDER; 2],
            active_idx: 0,
            fade: None,
        }
    }
}
```

- [ ] **Step 2: Modify handle_set_bg for fade support**

Replace the function in `src/plugins/rendering.rs`:

```rust
fn handle_set_bg(
    mut msg: MessageReader<SetBgMessage>,
    mut bg_state: ResMut<BgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
) {
    for msg in msg.read() {
        // Complete any in-progress fade instantly
        if bg_state.fade.is_some() {
            let active_entity = bg_state.entities[bg_state.active_idx];
            if let Ok((_, mut vis, _)) = query.get_mut(active_entity) {
                *vis = Visibility::Hidden;
            }
            bg_state.active_idx = 1 - bg_state.active_idx;
            bg_state.fade = None;
        }

        let path = format!("images/bg/{}", msg.file);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        let inactive_idx = 1 - bg_state.active_idx;
        let inactive_entity = bg_state.entities[inactive_idx];

        if let Ok((mut image_node, mut vis, mut bg)) = query.get_mut(inactive_entity) {
            image_node.image = handle;
            match msg.transition {
                Some(Transition::Fade) => {
                    let dur = msg.duration.unwrap_or(0.5) as f32;
                    bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
                    *vis = Visibility::Visible;
                    bg_state.fade = Some(BgCrossFade {
                        timer: Timer::from_seconds(dur, TimerMode::Once),
                    });
                }
                _ => {
                    *vis = Visibility::Visible;
                    bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0);
                    if let Ok((_, mut old_vis, _)) = query.get_mut(bg_state.entities[bg_state.active_idx]) {
                        *old_vis = Visibility::Hidden;
                    }
                    bg_state.active_idx = inactive_idx;
                    bg_state.fade = None;
                }
            }
        }
    }
}
```

Add import in `rendering.rs`:

```rust
use crate::resources::{BgState, BgCrossFade, CgState, SpriteManager, TextureCache};
use crate::script::{FgPosition, Transition};
```

- [ ] **Step 3: Add update_bg_fade system**

Add after the handlers:

```rust
fn update_bg_fade(
    time: Res<Time>,
    mut bg_state: ResMut<BgState>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    let Some(fade) = &mut bg_state.fade else { return };

    fade.timer.tick(time.delta());
    let t = fade.timer.fraction();

    let active_entity = bg_state.entities[bg_state.active_idx];
    let inactive_entity = bg_state.entities[1 - bg_state.active_idx];

    if let Ok((mut bg, _)) = query.get_mut(active_entity) {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0 - t);
    }
    if let Ok((mut bg, _)) = query.get_mut(inactive_entity) {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, t);
    }

    if fade.timer.finished() {
        if let Ok((_, mut vis)) = query.get_mut(active_entity) {
            *vis = Visibility::Hidden;
        }
        bg_state.active_idx = 1 - bg_state.active_idx;
        bg_state.fade = None;
    }
}
```

- [ ] **Step 4: Register update_bg_fade in the plugin**

Add to the Update system tuple in `RenderingPlugin::build`:

```rust
.add_systems(Update, (
    update_bg_fade,
    handle_set_bg,
    handle_show_fg,
    handle_hide_fg,
    handle_show_cg,
    handle_hide_cg,
).chain().run_if(in_state(AppState::Gameplay)))
```

- [ ] **Step 5: cargo check**

Run: `cargo check`
Expected: 0 new errors

- [ ] **Step 6: Commit**

```bash
git add src/resources.rs src/plugins/rendering.rs
git commit -m "feat: implement BG cross-fade with dual-buffer alpha lerp"
```

---

### Task 3: FG + CG fade

**Files:**
- Modify: `src/resources.rs`
- Modify: `src/plugins/rendering.rs`

- [ ] **Step 1: Extend SpriteSlotInfo and CgState with fade state**

In `src/resources.rs`, add fade structs:

```rust
pub struct SpriteFade {
    pub timer: Timer,
    pub kind: SpriteFadeKind,
}

pub enum SpriteFadeKind {
    FadeIn,
    FadeOut,
}
```

Extend `SpriteSlotInfo`:

```rust
pub struct SpriteSlotInfo {
    pub char_id: String,
    pub expression: String,
    pub entity: Entity,
    pub texture: Option<Handle<Image>>,
    pub fade: Option<SpriteFade>,
}
```

In `src/resources.rs`, add CG fade structs:

```rust
pub struct CgFade {
    pub timer: Timer,
    pub kind: CgFadeKind,
}

pub enum CgFadeKind {
    FadeIn,
    FadeOut,
}
```

Extend `CgState`:

```rust
#[derive(Resource, Default)]
pub struct CgState {
    pub active: bool,
    pub entity: Option<Entity>,
    pub texture: Option<Handle<Image>>,
    pub fade: Option<CgFade>,
}
```

- [ ] **Step 2: Modify handle_show_fg for fade support**

In `src/plugins/rendering.rs`:

```rust
fn handle_show_fg(
    mut msg: MessageReader<ShowFgMessage>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
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

            if let Ok((mut image_node, mut vis, mut bg)) = query.get_mut(slot.entity) {
                image_node.image = handle;
                match msg.transition {
                    Some(Transition::Fade) => {
                        let dur = msg.duration.unwrap_or(0.5) as f32;
                        bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
                        *vis = Visibility::Visible;
                        slot.fade = Some(SpriteFade {
                            timer: Timer::from_seconds(dur, TimerMode::Once),
                            kind: SpriteFadeKind::FadeIn,
                        });
                    }
                    _ => {
                        bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0);
                        *vis = Visibility::Visible;
                        slot.fade = None;
                    }
                }
            }
        } else {
            warn!("No sprite slot for position: {:?}", msg.position);
        }
    }
}
```

- [ ] **Step 3: Modify handle_hide_fg for fade support**

```rust
fn handle_hide_fg(
    mut msg: MessageReader<HideFgMessage>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
) {
    for msg in msg.read() {
        let slot = sprite_mgr.slots.values_mut()
            .find(|s| s.char_id == msg.char_id);

        if let Some(slot) = slot {
            match msg.transition {
                Some(Transition::Fade) => {
                    let dur = msg.duration.unwrap_or(0.5) as f32;
                    slot.fade = Some(SpriteFade {
                        timer: Timer::from_seconds(dur, TimerMode::Once),
                        kind: SpriteFadeKind::FadeOut,
                    });
                }
                _ => {
                    slot.char_id.clear();
                    slot.expression.clear();
                    slot.texture = None;
                    if let Ok((mut image_node, mut vis, _)) = query.get_mut(slot.entity) {
                        image_node.image = Handle::default();
                        *vis = Visibility::Hidden;
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Modify handle_show_cg for fade support**

```rust
fn handle_show_cg(
    mut msg: MessageReader<ShowCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in msg.read() {
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            Visibility::Visible,
            ZIndex(2),
        )).id();

        cg_state.active = true;
        cg_state.entity = Some(entity);
        cg_state.texture = Some(handle);

        match msg.transition {
            Some(Transition::Fade) => {
                let dur = msg.duration.unwrap_or(0.5) as f32;
                cg_state.fade = Some(CgFade {
                    timer: Timer::from_seconds(dur, TimerMode::Once),
                    kind: CgFadeKind::FadeIn,
                });
            }
            _ => {
                // Set full opacity immediately
                if let Ok((_, _, mut bg)) = commands.get_entity(entity).map(|e| {
                    // Can't query from commands; handled by setting BackgroundColor during spawn
                }).or(Ok(())) {
                    // Already at alpha 0 from spawn; need full alpha
                }
                // Workaround: despawn and respawn at full alpha
                commands.entity(entity).despawn();
                let entity = commands.spawn((
                    CgRoot,
                    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), position_type: PositionType::Absolute, top: Val::Px(0.0), left: Val::Px(0.0), ..default() },
                    ImageNode::new(handle.clone()),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
                    Visibility::Visible,
                    ZIndex(2),
                )).id();
                cg_state.active = true;
                cg_state.entity = Some(entity);
                cg_state.texture = Some(handle);
                cg_state.fade = None;
            }
        }
    }
}
```

Wait — despawn+respawn in the None/Instant branch is ugly. Let me restructure so Instant uses a common spawn at alpha 1 and Fade uses a common spawn at alpha 0 + fade timer.

```rust
fn handle_show_cg(
    mut msg: MessageReader<ShowCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in msg.read() {
        if let Some(entity) = cg_state.entity.take() {
            commands.entity(entity).despawn();
        }

        let path = format!("images/ev/{}", msg.file);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        let initial_alpha = match msg.transition {
            Some(Transition::Fade) => 0.0,
            _ => 1.0,
        };

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, initial_alpha)),
            Visibility::Visible,
            ZIndex(2),
        )).id();

        cg_state.active = true;
        cg_state.entity = Some(entity);
        cg_state.texture = Some(handle);

        match msg.transition {
            Some(Transition::Fade) => {
                let dur = msg.duration.unwrap_or(0.5) as f32;
                cg_state.fade = Some(CgFade {
                    timer: Timer::from_seconds(dur, TimerMode::Once),
                    kind: CgFadeKind::FadeIn,
                });
            }
            _ => {
                cg_state.fade = None;
            }
        }
    }
}
```

- [ ] **Step 5: Modify handle_hide_cg for fade support**

```rust
fn handle_hide_cg(
    mut msg: MessageReader<HideCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut commands: Commands,
) {
    for _ in msg.read() {
        match msg.transition {
            Some(Transition::Fade) => {
                if let Some(entity) = cg_state.entity {
                    let dur = msg.duration.unwrap_or(0.5) as f32;
                    cg_state.fade = Some(CgFade {
                        timer: Timer::from_seconds(dur, TimerMode::Once),
                        kind: CgFadeKind::FadeOut,
                    });
                }
            }
            _ => {
                if let Some(entity) = cg_state.entity.take() {
                    commands.entity(entity).despawn();
                }
                cg_state.active = false;
                cg_state.texture = None;
            }
        }
    }
}
```

Wait, there's a borrow issue here. `cg_state.entity` is accessed immutably in the match (to check if it's Some), but then `cg_state.entity.take()` is called later. Let me restructure:

```rust
fn handle_hide_cg(
    mut msg: MessageReader<HideCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut commands: Commands,
) {
    for _ in msg.read() {
        match msg.transition {
            Some(Transition::Fade) => {
                let dur = msg.duration.unwrap_or(0.5) as f32;
                cg_state.fade = Some(CgFade {
                    timer: Timer::from_seconds(dur, TimerMode::Once),
                    kind: CgFadeKind::FadeOut,
                });
            }
            _ => {
                if let Some(entity) = cg_state.entity.take() {
                    commands.entity(entity).despawn();
                }
                cg_state.active = false;
                cg_state.texture = None;
            }
        }
    }
}
```

Wait, but `msg` is moved in the `for _ in msg.read()`. Actually `msg` is `MessageReader<HideCgMessage>` and `.read()` returns an iterator. But `msg.transition` and `msg.duration` — `msg` is the reader, not individual messages. The `_` pattern is for the individual messages.

Let me check: `HideCgMessage` has only `transition` and `duration` fields. So:

```rust
fn handle_hide_cg(
    mut msg: MessageReader<HideCgMessage>,
    mut cg_state: ResMut<CgState>,
    mut commands: Commands,
) {
    for msg in msg.read() {
        match msg.transition {
            Some(Transition::Fade) => {
                if cg_state.entity.is_some() {
                    let dur = msg.duration.unwrap_or(0.5) as f32;
                    cg_state.fade = Some(CgFade {
                        timer: Timer::from_seconds(dur, TimerMode::Once),
                        kind: CgFadeKind::FadeOut,
                    });
                }
            }
            _ => {
                if let Some(entity) = cg_state.entity.take() {
                    commands.entity(entity).despawn();
                }
                cg_state.active = false;
                cg_state.texture = None;
            }
        }
    }
}
```

This looks better. `cg_state.entity.is_some()` borrows immutably, and the `take()` in the other branch happens only if the match didn't enter the Fade branch.

Actually wait, there may be a borrow issue. `cg_state` is `ResMut<CgState>`. In the Fade branch, I do `cg_state.fade = Some(...)` which borrows `cg_state` mutably. In the other branch, `cg_state.entity.take()` also borrows mutably. These are in different match arms so they can't both execute. The borrow checker should be fine here since only one branch runs.

Actually, I also need access to `msg.duration` and `msg.transition` inside the loop. The loop variable `msg` is a reference to the individual messages. This should be fine.

- [ ] **Step 6: Add update_fg_fade and update_cg_fade systems**

Add after `handle_hide_cg`:

```rust
fn update_fg_fade(
    time: Res<Time>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    for (_position, slot) in sprite_mgr.slots.iter_mut() {
        let Some(fade) = &mut slot.fade else { continue };

        fade.timer.tick(time.delta());
        let t = fade.timer.fraction();

        if let Ok((mut bg, _)) = query.get_mut(slot.entity) {
            let alpha = match fade.kind {
                SpriteFadeKind::FadeIn => t,
                SpriteFadeKind::FadeOut => 1.0 - t,
            };
            bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
        }

        if fade.timer.finished() {
            if matches!(fade.kind, SpriteFadeKind::FadeOut) {
                if let Ok((_, mut vis)) = query.get_mut(slot.entity) {
                    *vis = Visibility::Hidden;
                }
                slot.char_id.clear();
                slot.expression.clear();
                slot.texture = None;
            }
            slot.fade = None;
        }
    }
}

fn update_cg_fade(
    time: Res<Time>,
    mut cg_state: ResMut<CgState>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    let Some(fade) = &mut cg_state.fade else { return };

    fade.timer.tick(time.delta());
    let t = fade.timer.fraction();

    if let Some(entity) = cg_state.entity {
        if let Ok((mut bg, _)) = query.get_mut(entity) {
            let alpha = match fade.kind {
                CgFadeKind::FadeIn => t,
                CgFadeKind::FadeOut => 1.0 - t,
            };
            bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
        }

        if fade.timer.finished() {
            if matches!(fade.kind, CgFadeKind::FadeOut) {
                if let Ok((_, mut vis)) = query.get_mut(entity) {
                    *vis = Visibility::Hidden;
                }
                if let Some(entity) = cg_state.entity.take() {
                    // Entity hidden; despawn handled on next show or cleanup_rendering
                }
                cg_state.active = false;
                cg_state.texture = None;
            }
            cg_state.fade = None;
        }
    } else {
        cg_state.fade = None;
    }
}
```

Wait, `cg_state.fade` is already borrowed mutably by `let Some(fade) = &mut cg_state.fade` on line 2. Then later `cg_state.fade = None` would be a second mutable borrow. Let me restructure:

```rust
fn update_cg_fade(
    time: Res<Time>,
    mut cg_state: ResMut<CgState>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    let fade = match &mut cg_state.fade {
        Some(f) => f,
        None => return,
    };

    fade.timer.tick(time.delta());
    let t = fade.timer.fraction();

    let finished = fade.timer.finished();
    let is_fade_out = matches!(fade.kind, CgFadeKind::FadeOut);

    if let Some(entity) = cg_state.entity {
        if let Ok((mut bg, _)) = query.get_mut(entity) {
            let alpha = match fade.kind {
                CgFadeKind::FadeIn => t,
                CgFadeKind::FadeOut => 1.0 - t,
            };
            bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
        }

        if finished {
            if is_fade_out {
                if let Ok((_, mut vis)) = query.get_mut(entity) {
                    *vis = Visibility::Hidden;
                }
            }
            cg_state.fade = None;
        }
    }
}
```

Hmm, there's still a potential issue. `cg_state.entity` is used after `fade = &mut cg_state.fade`. In Rust's borrow checker, borrowing `cg_state.fade` (a field of a struct) doesn't prevent accessing other fields of the same struct. So `cg_state.entity` should be accessible. Actually, since `cg_state` is `ResMut<CgState>`, and `fade` borrows `cg_state.fade`, accessing `cg_state.entity` should work in Rust as long as the borrow checker can prove they're disjoint fields. In Rust, field-level borrows are tracked, so this should work.

But then later `cg_state.fade = None` — this requires mutable access to `cg_state.fade` which is already borrowed by `fade`. This won't compile.

Let me restructure to avoid this:

```rust
fn update_cg_fade(
    time: Res<Time>,
    mut cg_state: ResMut<CgState>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    let finished = {
        let fade = match &mut cg_state.fade {
            Some(f) => f,
            None => return,
        };

        fade.timer.tick(time.delta());
        let t = fade.timer.fraction();

        if let Some(entity) = cg_state.entity {
            if let Ok((mut bg, _)) = query.get_mut(entity) {
                let alpha = match fade.kind {
                    CgFadeKind::FadeIn => t,
                    CgFadeKind::FadeOut => 1.0 - t,
                };
                bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
            }
        }

        let finished = fade.timer.finished();
        if finished && matches!(fade.kind, CgFadeKind::FadeOut) {
            if let Some(entity) = cg_state.entity {
                if let Ok((_, mut vis)) = query.get_mut(entity) {
                    *vis = Visibility::Hidden;
                }
            }
        }
        finished
    };

    if finished {
        cg_state.fade = None;
    }
}
```

This uses a block to scope the `fade` borrow, then uses `finished` (a bool) to decide whether to clear the fade. This avoids the double borrow issue.

For the FG update, same pattern:

```rust
fn update_fg_fade(
    time: Res<Time>,
    mut sprite_mgr: ResMut<SpriteManager>,
    mut query: Query<(&mut BackgroundColor, &mut Visibility)>,
) {
    for (_position, slot) in sprite_mgr.slots.iter_mut() {
        let finished = {
            let fade = match &mut slot.fade {
                Some(f) => f,
                None => continue,
            };

            fade.timer.tick(time.delta());
            let t = fade.timer.fraction();

            if let Ok((mut bg, _)) = query.get_mut(slot.entity) {
                let alpha = match fade.kind {
                    SpriteFadeKind::FadeIn => t,
                    SpriteFadeKind::FadeOut => 1.0 - t,
                };
                bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
            }

            let finished = fade.timer.finished();
            if finished && matches!(fade.kind, SpriteFadeKind::FadeOut) {
                if let Ok((_, mut vis)) = query.get_mut(slot.entity) {
                    *vis = Visibility::Hidden;
                }
            }
            finished
        };

        if finished {
            slot.fade = None;
        }
    }
}
```

Wait, but `slot.entity` is also being accessed inside the block while `fade` borrows `slot.fade`. Since these are different fields of the same struct, this should work with field-level borrow tracking.

Actually, there's a subtlety. `sprite_mgr.slots.iter_mut()` gives `(&FgPosition, &mut SpriteSlotInfo)`. Inside the block, `fade = &mut slot.fade` borrows one field, and `slot.entity` (a Copy type) is accessed. In Rust, this should work because `slot.entity` doesn't require borrowing `slot` — it's a Copy type read.

But `query.get_mut(slot.entity)` borrows from the world (component storage), and `slot.fade` is in the resource storage. Different storages, so Bevy's system checks should pass. And in Rust, `slot.entity` and `slot.fade` are different fields so there's no borrow conflict.

Actually wait, the issue is that `sprite_mgr` is `ResMut<SpriteManager>` and `query` is a `Query`. In Bevy's system safety checks, `ResMut` and `Query` are from different world storages, so there's no conflict. Good.

Let me also fix the import in rendering.rs for the new types:

```rust
use crate::resources::{BgState, BgCrossFade, CgState, CgFade, CgFadeKind, SpriteManager, SpriteSlotInfo, SpriteFade, SpriteFadeKind, TextureCache};
```

- [ ] **Step 7: Register update systems in the plugin**

```rust
.add_systems(Update, (
    update_bg_fade,
    update_fg_fade,
    update_cg_fade,
    handle_set_bg,
    handle_show_fg,
    handle_hide_fg,
    handle_show_cg,
    handle_hide_cg,
).chain().run_if(in_state(AppState::Gameplay)))
```

- [ ] **Step 8: cargo check**

Run: `cargo check`
Expected: 0 new errors

- [ ] **Step 9: Commit**

```bash
git add src/resources.rs src/plugins/rendering.rs
git commit -m "feat: implement FG sprite and CG fade in/out transitions"
```

---

### Task 4: ScreenTransition resource + system + marker component

**Files:**
- Modify: `src/components.rs`
- Modify: `src/resources.rs`
- Create: `src/plugins/screen_transition.rs`

- [ ] **Step 1: Add TransitionOverlay marker component**

In `src/components.rs`, add after the last component:

```rust
#[derive(Component)]
pub struct TransitionOverlay;
```

- [ ] **Step 2: Add ScreenTransition resource**

In `src/resources.rs`, add:

```rust
#[derive(Resource, Default)]
pub struct ScreenTransition {
    pub overlay: Option<Entity>,
    pub phase: TransitionPhase,
    pub pending_state: Option<AppState>,
}

pub enum TransitionPhase {
    Idle,
    FadingToBlack { timer: Timer },
    FadingFromBlack { timer: Timer },
}

impl Default for TransitionPhase {
    fn default() -> Self { Self::Idle }
}
```

- [ ] **Step 3: Create screen_transition.rs plugin**

Create `src/plugins/screen_transition.rs`:

```rust
use bevy::prelude::*;
use crate::components::TransitionOverlay;
use crate::resources::ScreenTransition;
use crate::state::AppState;

pub struct ScreenTransitionPlugin;

impl Plugin for ScreenTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenTransition>()
            .add_systems(Update, handle_screen_transition);
    }
}

fn handle_screen_transition(
    time: Res<Time>,
    mut transition: ResMut<ScreenTransition>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    match transition.phase {
        TransitionPhase::Idle => {
            if let Some(ref target) = transition.pending_state {
                // Spawn fullscreen black overlay
                let entity = commands.spawn((
                    TransitionOverlay,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    ZIndex(10),
                )).id();
                transition.overlay = Some(entity);
                transition.phase = TransitionPhase::FadingToBlack {
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                };
            }
        }
        TransitionPhase::FadingToBlack { ref mut timer } => {
            timer.tick(time.delta());
            let alpha = timer.fraction();
            if let Some(entity) = transition.overlay {
                if let Ok(mut bg) = commands.get_entity(entity).map(|_| unreachable!()) {
                    // Can't update BackgroundColor via commands; use query
                }
            }
            // We need a query for BackgroundColor; add a separate system or inline query
            if timer.finished() {
                let target = transition.pending_state.take().unwrap_or(AppState::Title);
                next_state.set(target);
                transition.phase = TransitionPhase::FadingFromBlack {
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                };
            }
        }
        TransitionPhase::FadingFromBlack { ref mut timer } => {
            timer.tick(time.delta());
            if timer.finished() {
                if let Some(entity) = transition.overlay.take() {
                    commands.entity(entity).despawn();
                }
                transition.phase = TransitionPhase::Idle;
            }
        }
    }
}
```

Hmm, the alpha update for the overlay needs a `Query`, not `Commands.get_entity`. But adding a `Query<&mut BackgroundColor, With<TransitionOverlay>>` to the system while also having `commands: Commands` is fine.

Let me restructure the system to include a query for updating the overlay:

```rust
fn handle_screen_transition(
    time: Res<Time>,
    mut transition: ResMut<ScreenTransition>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mut overlay_query: Query<&mut BackgroundColor, With<TransitionOverlay>>,
) {
    match transition.phase {
        TransitionPhase::Idle => {
            if transition.pending_state.is_some() {
                let entity = commands.spawn((
                    TransitionOverlay,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    ZIndex(10),
                )).id();
                transition.overlay = Some(entity);
                transition.phase = TransitionPhase::FadingToBlack {
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                };
            }
        }
        TransitionPhase::FadingToBlack { ref mut timer } => {
            timer.tick(time.delta());
            let alpha = timer.fraction();
            if let Some(entity) = transition.overlay {
                if let Ok(mut bg) = overlay_query.get_mut(entity) {
                    bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
                }
            }
            if timer.finished() {
                let target = transition.pending_state.take().unwrap_or(AppState::Title);
                next_state.set(target);
                transition.phase = TransitionPhase::FadingFromBlack {
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                };
            }
        }
        TransitionPhase::FadingFromBlack { ref mut timer } => {
            timer.tick(time.delta());
            let alpha = 1.0 - timer.fraction();
            if let Some(entity) = transition.overlay {
                if let Ok(mut bg) = overlay_query.get_mut(entity) {
                    bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
                }
            }
            if timer.finished() {
                if let Some(entity) = transition.overlay.take() {
                    commands.entity(entity).despawn();
                }
                transition.phase = TransitionPhase::Idle;
            }
        }
    }
}
```

- [ ] **Step 4: cargo check**

Need to add the module to `mod plugins` first, but that's in Task 6. For now, just add it to check syntax:

```bash
# Temporarily, just check the file parses:
cargo check 2>&1 | head -5
```

Wait, the file won't compile until it's registered in mod.rs or main.rs. So just write the file for now and verify in Task 6.

- [ ] **Step 5: Commit**

```bash
git add src/components.rs src/resources.rs src/plugins/screen_transition.rs
git commit -m "feat: add ScreenTransition resource, overlay system, and TransitionOverlay component"
```

---

### Task 5: Swap trigger points to use ScreenTransition

**Files:**
- Modify: `src/plugins/title.rs`
- Modify: `src/plugins/menu.rs`

Only Title→Gameplay and Menu→Title need state transition fades. All other menu-to-menu transitions stay instant.

- [ ] **Step 1: Update title.rs — use ScreenTransition instead of NextState**

```rust
use bevy::prelude::*;
use crate::resources::ScreenTransition;
use crate::state::AppState;

fn title_click(
    mut screen_transition: ResMut<ScreenTransition>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
) {
    if mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed() {
        screen_transition.pending_state = Some(AppState::Gameplay);
    }
}
```

Remove the `mut next_state: ResMut<NextState<AppState>>` parameter.

- [ ] **Step 2: Update menu.rs — handle Back to Title button**

Add import:
```rust
use crate::resources::ScreenTransition;
```

Change `handle_menu_button_interaction` to use `ScreenTransition` for the Title action:

```rust
fn handle_menu_button_interaction(
    query: Query<(&MenuButtonAction, &Interaction), Changed<Interaction>>,
    mut mode: ResMut<SaveLoadMode>,
    mut next_state: ResMut<NextState<AppState>>,
    mut screen_transition: ResMut<ScreenTransition>,
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
            MenuButtonAction::Title => screen_transition.pending_state = Some(AppState::Title),
        }
    }
}
```

Note: `handle_menu_toggle` (Escape between Gameplay/Menu) stays unchanged — those are overlay toggles, no fade.

- [ ] **Step 3: cargo check**

Run: `cargo check`
Expected: 0 new errors

- [ ] **Step 4: Commit**

```bash
git add src/plugins/title.rs src/plugins/menu.rs
git commit -m "feat: wire Title->Gameplay and Menu->Title through ScreenTransition"
```

---

### Task 6: Register everything in main.rs

**Files:**
- Modify: `src/main.rs`
- Modify: (implied) `src/plugins/mod.rs` to expose `screen_transition`

- [ ] **Step 1: Add ScreenTransitionPlugin to plugins/mod.rs**

Read the current `src/plugins/mod.rs` and add:
```rust
pub mod screen_transition;
```

- [ ] **Step 2: Register in main.rs**

Add import:
```rust
use plugins::screen_transition::ScreenTransitionPlugin;
```

Add plugin registration:
```rust
.add_plugins(ScreenTransitionPlugin)
```

Place it before `RenderingPlugin` or anywhere outside state-gated plugins. Add it to the `App::new()` chain.

- [ ] **Step 3: Add TransitionOverlay marker to components module**

Already added in Task 4, just confirm `pub use` or the derive works in `main.rs`.

- [ ] **Step 4: cargo check**

Run: `cargo check`
Expected: 0 new errors

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/plugins/mod.rs
git commit -m "feat: register ScreenTransitionPlugin and update mod.rs"
```

---

### Task 7: Build + smoke test

**Files:** none

- [ ] **Step 1: cargo build**

Run: `cargo build`
Expected: Build success with 0 new errors

- [ ] **Step 2: Smoke test**

Run: `timeout 6 ./target/debug/bevy-vn 2>&1 || true`
Expected: Game launches, no panics. The script `test.bscript.ron` runs through dialogue, SetBg, ShowFg, etc. Transitions should work (visually: assets fade in/out over ~0.5s during script execution).

- [ ] **Step 3: Final commit (if any fixes needed)**

```bash
git add -A && git commit -m "fix: address smoke test issues"
```

- [ ] **Step 4: Update PROGRESS.md**

Mark Phase 7 transitions as complete.
