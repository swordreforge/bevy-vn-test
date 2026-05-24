# Phase 4: Audio + Choice Branching UI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add audio playback (BGM/SE/Voice) and choice branching UI to the Bevy VN engine.

**Architecture:** Message-driven (same pattern as Phase 3 Rendering). ScriptRunner sends Messages (PlayBgmMessage, StopBgmMessage, PlaySeMessage, PlayVoiceMessage, ChoiceSelectedMessage). AudioPlugin reacts to audio messages. Choice UI systems manage a center-overlay choice modal with ZIndex(4).

**Tech Stack:** Bevy 0.18 (bevy_audio, bevy_asset), OGG Vorbis

---

## File Structure

### New Files
| File | Purpose |
|------|---------|
| `src/audio_messages.rs` | PlayBgmMessage, StopBgmMessage, PlaySeMessage, PlayVoiceMessage |
| `src/plugins/audio.rs` | AudioPlugin — handle_play_bgm, handle_stop_bgm, handle_play_se, handle_play_voice |
| `src/plugins/choice.rs` | ChoicePlugin — choice_ui_spawn, handle_choice_selection, choice_ui_cleanup |

### Modified Files
| File | Changes |
|------|---------|
| `src/resources.rs` | Add BgmManager resource |
| `src/components.rs` | Add ChoiceUiRoot marker component, ChoiceButtonIndex component |
| `src/plugins/script_runner.rs` | Add match arms for PlayBgm/StopBgm/PlaySe/PlayVoice/Choice |
| `src/plugins/mod.rs` | Add `pub mod audio; pub mod choice;` |
| `src/main.rs` | Register AudioPlugin and ChoicePlugin |
| `assets/scripts/test.bscript.ron` | Add choice branching |

### Asset Files
| File | Source |
|------|--------|
| `assets/audio/bgm/bgm_0101_a.ogg` | Copy from game-source |
| `assets/audio/se/00010.ogg` | Copy from game-source |
| `assets/audio/voice/aiy400201490.ogg` | Copy from game-source |

---

### Task 1: Create audio_messages.rs

**Files:**
- Create: `src/audio_messages.rs`

- [ ] **Step 1: Write the file**

Write `src/audio_messages.rs`:

```rust
use bevy::prelude::*;

#[derive(Message)]
pub struct PlayBgmMessage {
    pub id: String,
    pub volume: Option<f32>,
    pub fade_in: Option<u64>,
}

#[derive(Message)]
pub struct StopBgmMessage {
    pub id: Option<String>,
    pub fade_out: Option<u64>,
}

#[derive(Message)]
pub struct PlaySeMessage {
    pub file: String,
    pub volume: Option<f32>,
}

#[derive(Message)]
pub struct PlayVoiceMessage {
    pub file: String,
    pub volume: Option<f32>,
}
```

---

### Task 2: Add BgmManager resource

**Files:**
- Modify: `src/resources.rs`

- [ ] **Step 1: Add BgmManager to resources.rs**

Add after `TextureCache`:

```rust
#[derive(Resource, Default)]
pub struct BgmManager {
    pub current_id: Option<String>,
    pub entity: Option<Entity>,
}
```

---

### Task 3: Create AudioPlugin

**Files:**
- Create: `src/plugins/audio.rs`

- [ ] **Step 1: Write the audio plugin file**

Write `src/plugins/audio.rs`:

```rust
use bevy::{audio::Volume, prelude::*};
use crate::audio_messages::*;
use crate::resources::BgmManager;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<BgmManager>()
            .add_message::<PlayBgmMessage>()
            .add_message::<StopBgmMessage>()
            .add_message::<PlaySeMessage>()
            .add_message::<PlayVoiceMessage>()
            .add_systems(Update, (
                handle_play_bgm,
                handle_stop_bgm,
                handle_play_se,
                handle_play_voice,
            ));
    }
}

fn handle_play_bgm(
    mut reader: MessageReader<PlayBgmMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
    audio_sink: Query<&AudioSink>,
) {
    for msg in reader.read() {
        // Stop existing BGM
        if let Some(entity) = bgm.entity {
            commands.entity(entity).despawn();
        }

        let path = format!("audio/bgm/bgm_{}_a.ogg", msg.id);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        let entity = commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings::LOOP,
        )).id();

        // Set volume if specified
        if let Some(vol) = msg.volume {
            if let Ok(sink) = audio_sink.get(entity) {
                sink.set_volume(Volume::Linear(vol));
            }
        }

        bgm.current_id = Some(msg.id.clone());
        bgm.entity = Some(entity);
    }
}

fn handle_stop_bgm(
    mut reader: MessageReader<StopBgmMessage>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
) {
    for _ in reader.read() {
        if let Some(entity) = bgm.entity {
            commands.entity(entity).despawn();
        }
        bgm.current_id = None;
        bgm.entity = None;
    }
}

fn handle_play_se(
    mut reader: MessageReader<PlaySeMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let path = format!("audio/se/{}.ogg", msg.file);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN,
        ));
    }
}

fn handle_play_voice(
    mut reader: MessageReader<PlayVoiceMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let path = format!("audio/voice/{}.ogg", msg.file);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN,
        ));
    }
}
```

---

### Task 4: Create choice_messages.rs and choice components

**Files:**
- Create: `src/choice_messages.rs`
- Modify: `src/components.rs`

- [ ] **Step 1: Create choice_messages.rs**

Write `src/choice_messages.rs`:

```rust
use bevy::prelude::*;

#[derive(Message)]
pub struct ChoiceSelectedMessage {
    pub index: usize,
}
```

- [ ] **Step 2: Add markers to components.rs**

Add after `CgRoot`:

```rust
#[derive(Component)]
pub struct ChoiceUiRoot;

#[derive(Component)]
pub struct ChoiceButtonIndex(pub usize);
```

---

### Task 5: Add ChoiceState resource

**Files:**
- Modify: `src/resources.rs`

- [ ] **Step 1: Add ChoiceState**

Add after ChoiceUiRoot:

Actually add after `BgmManager`:

```rust
#[derive(Resource, Default)]
pub struct ChoiceState {
    pub active: bool,
    pub options: Vec<crate::script::ChoiceOption>,
}
```

---

### Task 6: Create ChoicePlugin

**Files:**
- Create: `src/plugins/choice.rs`

- [ ] **Step 1: Write the choice plugin**

Write `src/plugins/choice.rs`:

```rust
use bevy::prelude::*;
use crate::choice_messages::ChoiceSelectedMessage;
use crate::components::{ChoiceUiRoot, ChoiceButtonIndex};
use crate::resources::ChoiceState;
use crate::script::ChoiceOption;

pub struct ChoicePlugin;

impl Plugin for ChoicePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ChoiceState>()
            .add_message::<ChoiceSelectedMessage>()
            .add_systems(Update, (
                choice_ui_spawn.run_if(|state: Res<ChoiceState>| state.active),
                handle_choice_selection,
                choice_ui_cleanup.run_if(|state: Res<ChoiceState>| !state.active),
            ));
    }
}

const CHOICE_BG_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
const CHOICE_BUTTON_COLOR: Color = Color::srgba(0.15, 0.15, 0.15, 0.95);
const CHOICE_BUTTON_HOVER: Color = Color::srgba(0.35, 0.35, 0.35, 0.95);
const CHOICE_BUTTON_PRESSED: Color = Color::srgba(0.25, 0.25, 0.25, 0.95);

fn choice_ui_spawn(
    mut commands: Commands,
    state: Res<ChoiceState>,
    existing: Query<Entity, With<ChoiceUiRoot>>,
    asset_server: Res<AssetServer>,
) {
    if !existing.is_empty() {
        return;
    }

    let mut parent = commands.spawn((
        ChoiceUiRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            ..default()
        },
        BackgroundColor(CHOICE_BG_COLOR),
        ZIndex(4),
    ));

    for (i, option) in state.options.iter().enumerate() {
        parent.with_child((
            ChoiceButtonIndex(i),
            Button,
            Node {
                width: Val::Px(768.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(CHOICE_BUTTON_COLOR),
            Text::new(&option.text),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    }
}

fn handle_choice_selection(
    mut query: Query<(&Interaction, &ChoiceButtonIndex, &mut BackgroundColor), Changed<Interaction>>,
    mut writer: MessageWriter<ChoiceSelectedMessage>,
    state: Res<ChoiceState>,
) {
    if !state.active {
        return;
    }

    for (interaction, index, mut bg) in query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(CHOICE_BUTTON_PRESSED);
                writer.write(ChoiceSelectedMessage { index: index.0 });
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(CHOICE_BUTTON_HOVER);
            }
            Interaction::None => {
                *bg = BackgroundColor(CHOICE_BUTTON_COLOR);
            }
        }
    }
}

fn choice_ui_cleanup(
    mut commands: Commands,
    existing: Query<Entity, With<ChoiceUiRoot>>,
) {
    for entity in existing.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
```

Note: The hover color handling above is simplified. In practice, each button's BackgroundColor can be updated individually when its Interaction changes. Let me fix the hover system:

```rust
fn handle_choice_selection(
    mut query: Query<(&Interaction, &ChoiceButtonIndex, &mut BackgroundColor), Changed<Interaction>>,
    mut writer: MessageWriter<ChoiceSelectedMessage>,
    state: Res<ChoiceState>,
) {
    if !state.active {
        return;
    }

    for (interaction, index, mut bg) in query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(CHOICE_BUTTON_PRESSED);
                writer.write(ChoiceSelectedMessage { index: index.0 });
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(CHOICE_BUTTON_HOVER);
            }
            Interaction::None => {
                *bg = BackgroundColor(CHOICE_BUTTON_COLOR);
            }
        }
    }
}
```

---

### Task 7: Update ScriptRunner for audio + choice

**Files:**
- Modify: `src/plugins/script_runner.rs`

- [ ] **Step 1: Add audio and choice message imports**

Replace the existing imports:

```rust
use bevy::prelude::*;
use crate::resources::{AffectionMap, DialogueState, ChoiceState};
use crate::script::{ConditionOp, ScriptCmd, ScriptEngine};
use crate::state::AppState;
use crate::plugins::inputs::AdvanceEvent;
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowCgMessage, HideCgMessage,
};
use crate::audio_messages::{
    PlayBgmMessage, StopBgmMessage, PlaySeMessage, PlayVoiceMessage,
};
use crate::choice_messages::ChoiceSelectedMessage;
```

- [ ] **Step 2: Add new parameters to process_advance**

Replace the existing function signature to add:

```rust
fn process_advance(
    mut advance_ev: MessageReader<AdvanceEvent>,
    mut choice_ev: MessageReader<ChoiceSelectedMessage>,
    mut engine: ResMut<ScriptEngine>,
    mut dialogue: ResMut<DialogueState>,
    mut affection: ResMut<AffectionMap>,
    mut choice_state: ResMut<ChoiceState>,
    // rendering writers
    mut set_bg_writer: MessageWriter<SetBgMessage>,
    mut show_fg_writer: MessageWriter<ShowFgMessage>,
    mut hide_fg_writer: MessageWriter<HideFgMessage>,
    mut show_cg_writer: MessageWriter<ShowCgMessage>,
    mut hide_cg_writer: MessageWriter<HideCgMessage>,
    // audio writers
    mut play_bgm_writer: MessageWriter<PlayBgmMessage>,
    mut stop_bgm_writer: MessageWriter<StopBgmMessage>,
    mut play_se_writer: MessageWriter<PlaySeMessage>,
    mut play_voice_writer: MessageWriter<PlayVoiceMessage>,
) {
```

- [ ] **Step 3: Add choice handling at the top of the loop**

After `for _ in advance_ev.read() {`, add before the dialogue check:

```rust
        // If choice is active, check if user made a selection
        if choice_state.active {
            for ev in choice_ev.read() {
                if ev.index < choice_state.options.len() {
                    let option = &choice_state.options[ev.index];
                    // Apply affection change
                    if let Some((ref char_id, delta)) = option.affection_change {
                        *affection.0.entry(char_id.clone()).or_insert(0) += delta;
                    }
                    // Jump to label
                    if let Some(ref target) = option.goto {
                        if !engine.jump_to_label(target) {
                            warn!("Choice jump target not found: {}", target);
                        }
                    }
                }
                choice_state.active = false;
                choice_state.options.clear();
            }
            continue;
        }
```

- [ ] **Step 4: Add Choice handler inside the match block**

Add before `// Keep no-op log`:

```rust
                Some(ScriptCmd::Choice { options }) => {
                    choice_state.active = true;
                    choice_state.options = options;
                    break;
                }
```

- [ ] **Step 5: Add audio command handlers**

Add after `HideCg` handler and before `Choice`:

```rust
                Some(ScriptCmd::PlayBgm { id, volume, fade_in }) => {
                    play_bgm_writer.write(PlayBgmMessage { id, volume, fade_in });
                }
                Some(ScriptCmd::StopBgm { id, fade_out }) => {
                    stop_bgm_writer.write(StopBgmMessage { id, fade_out });
                }
                Some(ScriptCmd::PlaySe { file, volume }) => {
                    play_se_writer.write(PlaySeMessage { file, volume });
                }
                Some(ScriptCmd::PlayVoice { file }) => {
                    play_voice_writer.write(PlayVoiceMessage { file, volume: None });
                }
```

---

### Task 8: Register plugins and modules

**Files:**
- Modify: `src/plugins/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add module declarations to mod.rs**

```rust
pub mod audio;
pub mod choice;
```

- [ ] **Step 2: Add plugins to main.rs**

After `RenderingPlugin` registration:

```rust
            .add_plugins(plugins::audio::AudioPlugin)
            .add_plugins(plugins::choice::ChoicePlugin)
```

---

### Task 9: Copy sample audio assets

**Files:**
- Copy from game-source to assets/

- [ ] **Step 1: Copy BGM, SE, and voice files**

```bash
mkdir -p assets/audio/bgm assets/audio/se assets/audio/voice
cp /home/swordreforge/Downloads/game-source/bgm/bgm_0101_a.ogg assets/audio/bgm/
cp /home/swordreforge/Downloads/game-source/se/00010.ogg assets/audio/se/
# Pick a voice file — use aits or aiy prefixed
cp /home/swordreforge/Downloads/game-source/voice/001_ユースティア/ai* assets/audio/voice/ 2>/dev/null || true
ls assets/audio/bgm/ assets/audio/se/ assets/audio/voice/
```

Note: If the voice directory `001_ユースティア` has no files or different naming, pick any existing voice file:

```bash
# Find any voice file
find /home/swordreforge/Downloads/game-source/voice -name "*.ogg" | head -1 | xargs -I{} cp {} assets/audio/voice/
```

---

### Task 10: Update test.bscript.ron

**Files:**
- Modify: `assets/scripts/test.bscript.ron`

- [ ] **Step 1: Add PlayBgm, PlaySe, PlayVoice, and choice branching**

Read the current file first, then overwrite:

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
    PlayBgm(id: "0101", volume: Some(0.5), fade_in: Some(2000)),
    Dialogue(speaker: None, text: "— BGM start —"),
    ShowCg(file: "eve_010101.png", transition: None),
    Dialogue(speaker: None, text: "— CG表示 —"),
    PlaySe(file: "00010", volume: Some(0.8)),
    Dialogue(speaker: None, text: "— SE再生 —"),
    HideCg(transition: None),
    Choice(options: [
        ChoiceOption(text: "信じる", affection_change: Some(("nayuta", 1)), goto: Some("good_end")),
        ChoiceOption(text: "まだわからない", goto: Some("bad_end")),
    ]),
    Label(name: "good_end"),
    Dialogue(speaker: Some("ナユタ"), text: "そうか…ありがとう。"),
    PlayVoice(file: "aiy400201490", volume: None),
    Dialogue(speaker: None, text: "— voice test —"),
    StopBgm(id: None, fade_out: None),
    Dialogue(speaker: None, text: "— Fin —"),
    Jump(target: "end"),
    Label(name: "bad_end"),
    Dialogue(speaker: Some("ナユタ"), text: "そうか…いつかわかるといいな。"),
    StopBgm(id: None, fade_out: None),
    Dialogue(speaker: None, text: "— Fin —"),
    Label(name: "end"),
]
```

---

### Task 11: Smoke test

**Files:**
- (no file changes)

- [ ] **Step 1: cargo check**

```bash
cargo check 2>&1
```
Expected: succeeds with only pre-existing warnings (dead code for Settings, UnlockState, unused imports).

- [ ] **Step 2: cargo build**

```bash
cargo build 2>&1
```
Expected: build succeeds.

- [ ] **Step 3: Quick launch test**

```bash
timeout 4 ./target/debug/bevy-vn 2>&1 || true
```
Expected: game window opens at 1280x720, info logs show script loading, no panics or crashes.
