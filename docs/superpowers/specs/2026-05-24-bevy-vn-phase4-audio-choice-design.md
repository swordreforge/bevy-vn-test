# Phase 4: Audio + Choice Branching UI — Design Spec

## Overview

Add audio playback (BGM/SE/Voice) and choice branching UI to the Bevy VN engine. Follows the same message-driven pattern established in Phase 3: ScriptRunner sends Messages, a dedicated Plugin reacts.

---

## Audio

### Messages (`src/audio_messages.rs`)

```rust
#[derive(Message)]
struct PlayBgmMessage { id: String, volume: Option<f32>, fade_in: Option<u64> }

#[derive(Message)]
struct StopBgmMessage { id: Option<String>, fade_out: Option<u64> }

#[derive(Message)]
struct PlaySeMessage { file: String, volume: Option<f32> }

#[derive(Message)]
struct PlayVoiceMessage { file: String, volume: Option<f32> }
```

### Bevy 0.18 Audio API

- **Play BGM**: `commands.spawn((AudioPlayer(handle), PlaybackSettings::LOOP))` — auto-loop, `AudioSink` auto-added for volume control
- **Play SE/Voice**: `commands.spawn((AudioPlayer(handle), PlaybackSettings::DESPAWN))` — auto-despawn on finish
- **Volume**: `sink.set_volume(Volume::Linear(vol))` via `AudioSink` query
- **Stop**: `commands.entity(entity).despawn()`

### Resource

```rust
#[derive(Resource, Default)]
struct BgmManager {
    current_id: Option<String>,
    entity: Option<Entity>,
}
```

Stores the active BGM entity so `StopBgm` can despawn it and support fade-out timing.

### Plugin (`src/plugins/audio.rs`)

```
AudioPlugin
├── handle_play_bgm  ← PlayBgmMessage
│   ├── Stop current BGM if playing (despawn existing entity)
│   ├── AssetServer::load::<AudioSource>("audio/bgm/bgm_{id}_a.ogg")
│   ├── Spawn: (AudioPlayer(handle), PlaybackSettings::LOOP)
│   ├── Set volume via AudioSink query
│   └── Store entity in BgmManager
├── handle_stop_bgm  ← StopBgmMessage
│   ├── Despawn BGM entity (instant or delayed for fade)
│   └── Clear BgmManager
├── handle_play_se   ← PlaySeMessage
│   └── Spawn: (AudioPlayer(handle), PlaybackSettings::DESPAWN)
├── handle_play_voice ← PlayVoiceMessage
│   └── Spawn: (AudioPlayer(handle), PlaybackSettings::DESPAWN)
├── register messages in app
└── registered in main.rs after RenderingPlugin
```

### Asset Path Resolution

| Script command | Asset path |
|---|---|
| `PlayBgm { id: "0101" }` | `audio/bgm/bgm_0101_a.ogg` |
| `PlaySe { file: "00010" }` | `audio/se/00010.ogg` |
| `PlayVoice { file: "aiy400201490" }` | `audio/voice/aiy400201490.ogg` |

### Volume Handling

At play time, read from `Settings` resource (which already has `bgm_volume`, `se_volume`, `voice_volume`). Full Settings sliders come in Phase 7.

### ScriptRunner Changes

Add 4 new match arms to the `process_advance` match block, each writing the corresponding Message.

### Test Assets

Copy sample files from game-source:
- `bgm/` → one `bgm_0101_a.ogg`
- `se/` → one `se/00010.ogg`
- `voice/` → one voice file

### Audio Features NOT in Scope

- Cross-fade between BGM tracks (just stop + play)
- Panning/spatial audio
- Per-scene BGM unlocking (Phase 8)
- Artemis CSV registry lookup (Phase 8)

---

## Choice Branching UI

### Messages

```rust
#[derive(Message)]
struct ChoiceSelectedMessage { index: usize }
```

### Resource

```rust
#[derive(Resource, Default)]
struct ChoiceState {
    active: bool,
    options: Vec<ChoiceOption>,
}
```

`ChoiceOption` already exists in `script.rs` with `text`, `affection_change`, and `goto` fields.

### Marker Component

```rust
#[derive(Component)]
struct ChoiceUiRoot;
```

### Choice Overlay Entity

Spawned on `ChoiceState::active = true`:

```
Node (full-screen, absolute, centered flex)
├── Node (container, dark semi-transparent bg, centered column)
│   ├── TextButton (Option 1)
│   ├── TextButton (Option 2)
│   └── TextButton (Option 3...)
```

Each button is a `(Node, Text, Interaction)` tuple. When `Interaction::Pressed`, write `ChoiceSelectedMessage { index }`.

### Flow

1. `ScriptRunner.process_advance` encounters `ScriptCmd::Choice`
2. Set `ChoiceState { active: true, options }`
3. `break` — script execution pauses (same pattern as Dialogue reveal)
4. `spawn_choice_ui` system (run in `Update` when `ChoiceState::active && no ChoiceUiRoot entity exists`) spawns the overlay
5. `handle_choice_selection` system (runs on `Interaction::Pressed` within the choice UI) writes `ChoiceSelectedMessage`
6. `process_choice` system reads `ChoiceSelectedMessage`, applies `affection_change` if any, calls `engine.jump_to_label(goto)` if any, clears `ChoiceState`, despawns `ChoiceUiRoot`
7. `cleanup_choice_ui` system (run when `ChoiceState` becomes inactive) despawns overlay if any remains
8. ScriptRunner resumes on next AdvanceEvent

### Choice Button Style

- Width: 60% of screen
- Height: 60px per option
- Background: dark (rgba(0,0,0,0.8)), rounded corners
- Text: white, centered, font_size 22
- Hover: lighter background (rgba(50,50,50,0.9))
- Spacing: 12px between buttons

### Z-Index

- `ZIndex(4)` — one layer above dialogue UI (ZIndex 3) since choices modal-overlay the scene

### Test Script Update

Add a choice after the CG scene:

```ron
Choice(options: [
    ChoiceOption(text: "I understand.", affection_change: Some(("nayuta", 1)), goto: Some("good_end")),
    ChoiceOption(text: "I don't understand.", goto: Some("bad_end")),
]),
Label(name: "good_end"),
Dialogue(speaker: Some("ナユタ"), text: "Good end."),
Jump(target: "end"),
Label(name: "bad_end"),
Dialogue(speaker: Some("ナユタ"), text: "Bad end."),
Label(name: "end"),
```

### Choice Features NOT in Scope

- Choice timeouts
- Conditional option visibility (defer to Phase 6 affection system)
- Choice sound effects

---

## Implementation Plan

### Tasks

| # | Task | Files |
|---|------|-------|
| 1 | Create `src/audio_messages.rs` with 4 Message types | New file |
| 2 | Create `src/plugins/audio.rs` with AudioPlugin (play_bgm, stop_bgm, play_se, play_voice) | New file |
| 3 | Add BgmManager resource to `src/resources.rs` | Edit |
| 4 | Create ChoiceState resource, ChoiceSelectedMessage, ChoiceUiRoot component | Edit files |
| 5 | Create choice UI systems (spawn, handle selection, cleanup) | Edit `src/plugins/audio.rs` or new file |
| 6 | Update ScriptRunner: add PlayBgm/StopBgm/PlaySe/PlayVoice/Choice match arms | Edit `src/plugins/script_runner.rs` |
| 7 | Register AudioPlugin in main.rs + mod.rs, register messages | Edit |
| 8 | Copy sample audio assets from game-source | Bash |
| 9 | Update test.bscript.ron with choice branching | Edit |
| 10 | Smoke test (cargo build + brief launch) | Bash |

### Order

Tasks 1-2 → 3-5 (parallel) → 6 → 7 → 8-9 (parallel) → 10
