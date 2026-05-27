# ScrollBG Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add background scrolling/panning support matching the original Artemis `ScrollBG` Ethornel tag.

**Architecture:** Six independent changes — ScriptCmd variant → rendering message → component → script_runner handler → rendering handler/systems → mapper update. No new files needed.

**Tech Stack:** Bevy engine, Bevy `message` API

---

### Task 1: Add `ScriptCmd::ScrollBg` variant

**Files:**
- Modify: `src/script.rs` (~line 199)

- [ ] **Step 1: Add ScrollBg to the ScriptCmd enum**

Insert before the closing `}` of the enum at line 200:

```rust
    ScrollBg {
        file: String,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        fade: u64,
        wait: bool,
    },
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check 2>&1 | head -20
```

Expected: warnings about unused variant (will be fixed in later tasks).

---

### Task 2: Add `ScrollBgMessage`

**Files:**
- Modify: `src/rendering_messages.rs` (after `HideCgMessage`, ~line 46)

- [ ] **Step 1: Add the message struct**

```rust
#[derive(Message)]
pub struct ScrollBgMessage {
    pub file: String,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub fade: u64,
    pub wait: bool,
}
```

- [ ] **Step 2: Check it compiles**

```bash
cargo check 2>&1 | head -20
```

---

### Task 3: Add `BgScroll` component

**Files:**
- Modify: `src/components.rs` (after `SpriteAnchor`, ~line 202)

- [ ] **Step 1: Add the BgScroll component**

```rust
#[derive(Component)]
pub struct BgScroll {
    pub timer: Timer,
    pub start_x: f32,
    pub end_x: f32,
    pub start_y: f32,
    pub end_y: f32,
}
```

- [ ] **Step 2: Verify**

```bash
cargo check 2>&1 | head -20
```

---

### Task 4: Wire up ScrollBG in script_runner

**Files:**
- Modify: `src/plugins/script_runner.rs`

- [ ] **Step 1: Import `ScrollBgMessage` in the rendering_messages import block**

Add `ScrollBgMessage` to line 15:

```rust
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage,
    ShowFaceMessage, HideFaceMessage,
    ShowCgMessage, HideCgMessage,
    DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage,
    ScrollBgMessage,
};
```

- [ ] **Step 2: Add `scroll_bg_writer` to `ProcessAdvanceParams`**

After `play_voice_writer` (line 62):

```rust
    scroll_bg_writer: MessageWriter<'w, 's, ScrollBgMessage>,
```

- [ ] **Step 3: Destructure `scroll_bg_writer` in `process_advance`**

After `ref mut play_voice_writer` in the destructure block (line 163):

```rust
        ref mut scroll_bg_writer,
```

- [ ] **Step 4: Add `ScrollBg` handler in normal mode**

After the `ScriptCmd::PlayVoice` block in normal mode (~line 587), add:

```rust
                Some(ScriptCmd::ScrollBg { file, x1, y1, x2, y2, fade, wait }) => {
                    scroll_bg_writer.write(ScrollBgMessage { file, x1, y1, x2, y2, fade, wait });
                    if wait {
                        auto_skip.auto_timer = Some(Timer::from_seconds(fade as f32 / 1000.0, TimerMode::Once));
                        break;
                    }
                }
```

- [ ] **Step 5: Add `ScrollBg` handler in skip mode**

After the `ScriptCmd::Quake` block in skip mode (~line 624), add:

```rust
                    Some(ScriptCmd::ScrollBg { file, x1, y1, x2, y2, .. }) => {
                        scroll_bg_writer.write(ScrollBgMessage { file, x1, y1, x2, y2, fade: 0, wait: false });
                    }
```

- [ ] **Step 6: Verify**

```bash
cargo check 2>&1 | head -20
```

---

### Task 5: Implement ScrollBG in rendering plugin

**Files:**
- Modify: `src/plugins/rendering.rs`

- [ ] **Step 1: Import new types**

Add to the imports at lines 6-10:

```rust
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage,
    ShowFaceMessage, HideFaceMessage,
    ShowCgMessage, HideCgMessage,
    DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage,
    ScrollBgMessage,  // ADD THIS
};
```

Also add `BgScroll` to the components import (line 2):

```rust
use crate::components::*;
```
(already imports all from components)

- [ ] **Step 2: Register `ScrollBgMessage` in `RenderingPlugin::build`**

After line 54 (`.add_message::<HideCgMessage>()`), add:

```rust
            .add_message::<ScrollBgMessage>()
```

- [ ] **Step 3: Add systems to the schedule**

In the Update system set after `update_sprite_tweens` (after line 82), add:

```rust
                handle_scroll_bg,
                update_bg_scroll,
```

- [ ] **Step 4: Add `handle_scroll_bg` system**

After `handle_set_bg` (after line 261), add:

```rust
fn handle_scroll_bg(
    mut msg: MessageReader<ScrollBgMessage>,
    mut bg_state: ResMut<BgState>,
    mut cache: ResMut<TextureCache>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Node, &mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
) {
    for msg in msg.read() {
        let file = if msg.file.contains('.') { msg.file.clone() } else { format!("{}.jpg", msg.file) };
        let path = format!("images/bg/{}", file);
        let handle = cache.cache.entry(path.clone()).or_insert_with(|| {
            asset_server.load(&path)
        }).clone();

        for &entity in &bg_state.entities {
            commands.entity(entity).remove::<BgScroll>();
        }

        let active_idx = bg_state.active_idx;
        let active_entity = bg_state.entities[active_idx];

        if let Ok((entity, mut node, mut image_node, mut vis, mut bg)) = query.get_mut(active_entity) {
            image_node.image = handle.clone();

            if let Some(image) = images.get(&handle) {
                let w = image.texture_descriptor.size.width as f32;
                let h = image.texture_descriptor.size.height as f32;
                if w > 0.0 && h > 0.0 {
                    node.width = Val::Px(w);
                    node.height = Val::Px(h);
                }
            }

            node.left = Val::Px(msg.x1);
            node.top = Val::Px(msg.y1);
            bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0);
            *vis = Visibility::Visible;

            if (msg.x1 - msg.x2).abs() > 0.5 || (msg.y1 - msg.y2).abs() > 0.5 {
                let dur = (msg.fade as f32 / 1000.0).max(0.016);
                commands.entity(entity).insert(BgScroll {
                    timer: Timer::from_seconds(dur, TimerMode::Once),
                    start_x: msg.x1,
                    end_x: msg.x2,
                    start_y: msg.y1,
                    end_y: msg.y2,
                });
            }

            if bg_state.fade.is_some() {
                if let Ok((_, mut old_vis, _)) = query.get_mut(bg_state.entities[1 - active_idx]) {
                    *old_vis = Visibility::Hidden;
                }
                bg_state.active_idx = 1 - bg_state.active_idx;
                bg_state.fade = None;
            }
        }
    }
}
```

- [ ] **Step 5: Add `update_bg_scroll` system**

After `handle_scroll_bg`, add:

```rust
fn update_bg_scroll(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Node, &mut BgScroll)>,
    mut commands: Commands,
) {
    for (entity, mut node, mut scroll) in &mut query {
        scroll.timer.tick(time.delta());
        let t = scroll.timer.fraction();
        let eased = 1.0 - (1.0 - t) * (1.0 - t);

        node.left = Val::Px(scroll.start_x + (scroll.end_x - scroll.start_x) * eased);
        node.top = Val::Px(scroll.start_y + (scroll.end_y - scroll.start_y) * eased);

        if scroll.timer.just_finished() {
            node.left = Val::Px(scroll.end_x);
            node.top = Val::Px(scroll.end_y);
            commands.entity(entity).remove::<BgScroll>();
        }
    }
}
```

- [ ] **Step 6: Modify `handle_set_bg` to reset node size and cancel scroll**

Replace the query and inner logic to also reset node size and remove BgScroll:

Change the query at line 214 from:
```rust
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor)>,
```
to:
```rust
    mut query: Query<(&mut ImageNode, &mut Visibility, &mut BackgroundColor, &mut Node)>,
    mut commands: Commands,
```

After `let inactive_idx = 1 - bg_state.active_idx;` (line 235), add:
```rust
        for &entity in &bg_state.entities {
            commands.entity(entity).remove::<BgScroll>();
        }
```

In the `if let Ok(...)` block, change the destructure and reset node size. Replace the line:
```rust
        if let Ok((mut image_node, mut vis, mut bg)) = query.get_mut(inactive_entity) {
```
with:
```rust
        if let Ok((mut image_node, mut vis, mut bg, mut node)) = query.get_mut(inactive_entity) {
```

And inside that block, after `image_node.image = handle;`, add:
```rust
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
            node.left = Val::Px(0.0);
            node.top = Val::Px(0.0);
```

- [ ] **Step 7: Verify**

```bash
cargo check 2>&1 | head -20
```

Expected: clean compile.

---

### Task 6: Add ScrollBG mapper support

**Files:**
- Modify: `tools/artemis-export/src/mapper.rs`

- [ ] **Step 1: Add `ScrollBGenq` case in `map_calllua`**

After the `set_bg` case (line 284), add:

```rust
        s if s.contains("ScrollBGenq") || s.contains("scroll_bg") => {
            let file = cmd.attrs.get("file").or_else(|| cmd.attrs.get("0")).cloned().unwrap_or_default();
            let x1 = cmd.attrs.get("1").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y1 = cmd.attrs.get("2").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let x2 = cmd.attrs.get("4").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y2 = cmd.attrs.get("5").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let fade = cmd.attrs.get("9").and_then(|s| s.parse().ok()).unwrap_or(0);
            let wait = cmd.attrs.get("10").map(|s| s == "TRUE").unwrap_or(false);
            Some(vec![ScriptCmd::ScrollBg { file, x1, y1, x2, y2, fade, wait }])
        }
```

- [ ] **Step 2: Verify**

```bash
cd tools/artemis-export && cargo check 2>&1 | head -20
```

Expected: clean compile.
