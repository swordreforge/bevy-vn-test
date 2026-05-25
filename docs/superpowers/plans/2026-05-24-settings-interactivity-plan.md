# Settings Interactivity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Settings placeholder with interactive volume sliders, auto/skip toggles, message opacity, and wire everything to runtime behavior (audio volume, text speed, auto-advance, skip mode).

**Architecture:** Full-screen overlay settings (same style as Menu) with 4 sliders + 2 toggles. Interaction via marker components + systems. Runtime systems read `Settings` resource each frame/on-change.

**Tech Stack:** Bevy 0.18 UI (marker components, Interaction-based click detection), `AudioSink` for runtime volume, `Timer` for auto-advance.

---

### Task 1: Add components for Settings + Audio type markers

**Files:**
- Modify: `src/components.rs` (after line 59)

- [ ] **Add 7 new marker components** at the end of `src/components.rs`:

```rust
// === Settings UI Components ===
#[derive(Component)]
pub struct SettingsBackButton;

#[derive(Component)]
pub struct SliderSegment(pub usize);

#[derive(Component, Clone, Copy, PartialEq)]
pub enum SliderSetting {
    BgmVolume,
    SeVolume,
    VoiceVolume,
    TextSpeed,
    MsgOpacity,
}

#[derive(Component)]
pub struct SliderValueText;

#[derive(Component)]
pub struct ToggleOption {
    pub group: String,
    pub value: bool,
}

// === Audio type markers ===
#[derive(Component, Clone, Copy, PartialEq)]
pub enum AudioType {
    Bgm,
    Se,
    Voice,
}
```

- [ ] **Verify compilation:**

```bash
cargo check
```

Expected: compiles with only pre-existing warnings. If there's an import error, check that `SliderSetting` and `ToggleOption` are simple enough to not need extra imports.

- [ ] **Commit:**

```bash
git add src/components.rs
git commit -m "feat: add settings UI components and AudioType marker"
```

---

### Task 2: Rewrite Settings plugin with full UI + interaction

**Files:**
- Modify: `src/plugins/settings.rs` (full rewrite, 47→~250 lines)
- Reference: `src/plugins/menu.rs` (for dark overlay style)

- [ ] **Rewrite `src/plugins/settings.rs`** with:
  - `setup_settings_ui`: spawns full-screen overlay, back button, title, 4 sliders (10 clickable segments each), 2 toggle pairs, value text
  - `handle_slider_clicks`: detect `Interaction::Pressed` on `SliderSegment` + `SliderSetting`, compute value = idx × 10 (0–90), update `Settings` resource field
  - `handle_toggle_clicks`: detect `Interaction::Pressed` on `ToggleOption`, update `Settings` field based on `group` ("auto" or "skip") and `value` (true/false)
  - `handle_back_click`: detect `Interaction::Pressed` on `SettingsBackButton`, transition `AppState::Settings` → `AppState::Menu`
  - `update_slider_visuals`: query all `(SliderSegment, SliderSetting, &BackgroundColor, &Interaction)`, set filled (white) if idx × 10 ≤ current setting value, else dim (gray)
  - `update_toggle_visuals`: query all `ToggleOption`, set ON text white/dim and OFF text dim/white based on `Settings` value for that group

```rust
use bevy::prelude::*;
use crate::components::*;
use crate::resources::Settings;
use crate::state::AppState;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Settings), setup_settings_ui)
            .add_systems(OnExit(AppState::Settings), cleanup_settings)
            .add_systems(Update, (
                handle_slider_clicks,
                handle_toggle_clicks,
                handle_back_click,
                update_slider_visuals,
                update_toggle_visuals,
            ).run_if(in_state(AppState::Settings)));
    }
}

#[derive(Component)]
struct SettingsScreen;

// Slider config: (label, SliderSetting, initial getter fn, field setter fn)
// We use closures to map between SliderSetting enum and Settings fields
fn get_setting_value(setting: &Settings, s: SliderSetting) -> f32 {
    match s {
        SliderSetting::BgmVolume => setting.bgm_volume * 100.0,
        SliderSetting::SeVolume => setting.se_volume * 100.0,
        SliderSetting::VoiceVolume => setting.voice_volume * 100.0,
        SliderSetting::TextSpeed => setting.text_speed as f32,
        SliderSetting::MsgOpacity => setting.message_window_opacity as f32,
    }
}

fn set_setting_value(settings: &mut Settings, s: SliderSetting, val: f32) {
    match s {
        SliderSetting::BgmVolume => settings.bgm_volume = val / 100.0,
        SliderSetting::SeVolume => settings.se_volume = val / 100.0,
        SliderSetting::VoiceVolume => settings.voice_volume = val / 100.0,
        SliderSetting::TextSpeed => settings.text_speed = val as u32,
        SliderSetting::MsgOpacity => settings.message_window_opacity = val as u8,
    }
}

fn color_for_filled(filled: bool) -> Color {
    if filled {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::srgb(0.25, 0.25, 0.3)
    }
}

fn setup_settings_ui(mut commands: Commands, settings: Res<Settings>) {
    let slider_defs: [(&str, SliderSetting, f32); 5] = [
        ("BGM Volume", SliderSetting::BgmVolume, settings.bgm_volume * 100.0),
        ("SE Volume", SliderSetting::SeVolume, settings.se_volume * 100.0),
        ("Voice Volume", SliderSetting::VoiceVolume, settings.voice_volume * 100.0),
        ("Text Speed", SliderSetting::TextSpeed, settings.text_speed as f32),
        ("Msg Opacity", SliderSetting::MsgOpacity, settings.message_window_opacity as f32),
    ];

    commands.spawn((
        SettingsScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.15, 0.95)),
        ZIndex(5),
    )).with_children(|parent| {
        // Back button
        parent.spawn((
            SettingsBackButton,
            Text::new("← Back"),
            TextFont { font_size: 20.0, ..default() },
            TextColor(Color::srgb(0.6, 0.6, 0.8)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                left: Val::Px(20.0),
                ..default()
            },
        ));

        // Title
        parent.spawn((
            Text::new("Settings"),
            TextFont { font_size: 36.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(30.0)), ..default() },
        ));

        // Sliders
        for (label, setting, initial) in &slider_defs {
            let initial_val = *initial;
            let setting_copy = *setting;
            parent.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
            )).with_children(|row| {
                // Label
                row.spawn((
                    Text::new(*label),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                    Node { width: Val::Px(150.0), ..default() },
                ));

                // Track with 10 segments
                for i in 0..10 {
                    let seg_val = (i as f32) * 10.0;
                    let filled = seg_val <= initial_val;
                    row.spawn((
                        SliderSegment(i),
                        setting_copy,
                        Node {
                            width: Val::Px(20.0),
                            height: Val::Px(22.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(color_for_filled(filled)),
                    ));
                }

                // Value text
                row.spawn((
                    SliderValueText,
                    Text::new(format!("{:>3.0}", initial_val)),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                    Node {
                        width: Val::Px(50.0),
                        ..default()
                    },
                ));
            });
        }

        // Spacing before toggles
        parent.spawn((Node { height: Val::Px(20.0), ..default() },));

        // Toggles
        let toggle_defs: [(&str, &str, bool); 2] = [
            ("Auto Mode", "auto", settings.auto_mode),
            ("Skip Mode", "skip", settings.skip_mode),
        ];

        for (label, group, initial_val) in &toggle_defs {
            parent.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
            )).with_children(|row| {
                row.spawn((
                    Text::new(*label),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                    Node { width: Val::Px(150.0), ..default() },
                ));

                // ON button
                let on_active = *initial_val;
                row.spawn((
                    ToggleOption { group: group.to_string(), value: true },
                    Text::new("ON"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(if on_active { Color::WHITE } else { Color::srgb(0.4, 0.4, 0.5) }),
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if on_active { Color::srgb(0.15, 0.3, 0.6) } else { Color::srgb(0.12, 0.12, 0.18) }),
                ));

                // OFF button
                let off_active = !*initial_val;
                row.spawn((
                    ToggleOption { group: group.to_string(), value: false },
                    Text::new("OFF"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(if off_active { Color::WHITE } else { Color::srgb(0.4, 0.4, 0.5) }),
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if off_active { Color::srgb(0.3, 0.12, 0.12) } else { Color::srgb(0.12, 0.12, 0.18) }),
                ));
            });
        }
    });
}

fn handle_slider_clicks(
    mut settings: ResMut<Settings>,
    query: Query<(&SliderSegment, &SliderSetting, &Interaction), Changed<Interaction>>,
) {
    for (segment, slider_setting, interaction) in query.iter() {
        if *interaction == Interaction::Pressed {
            let val = (segment.0 as f32) * 10.0;
            set_setting_value(&mut settings, *slider_setting, val);
        }
    }
}

fn handle_toggle_clicks(
    mut settings: ResMut<Settings>,
    query: Query<(&ToggleOption, &Interaction), Changed<Interaction>>,
) {
    for (option, interaction) in query.iter() {
        if *interaction == Interaction::Pressed {
            match option.group.as_str() {
                "auto" => settings.auto_mode = option.value,
                "skip" => settings.skip_mode = option.value,
                _ => warn!("Unknown toggle group: {}", option.group),
            }
        }
    }
}

fn handle_back_click(
    mut next_state: ResMut<NextState<AppState>>,
    query: Query<&Interaction, (With<SettingsBackButton>, Changed<Interaction>)>,
) {
    for interaction in query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
        }
    }
}

fn update_slider_visuals(
    settings: Res<Settings>,
    mut query: Query<(&SliderSegment, &SliderSetting, &mut BackgroundColor), With<SliderSegment>>,
) {
    for (segment, slider_setting, mut bg) in query.iter_mut() {
        let current = get_setting_value(&settings, *slider_setting);
        let seg_val = (segment.0 as f32) * 10.0;
        *bg = BackgroundColor(color_for_filled(seg_val <= current));
    }
}

fn update_toggle_visuals(
    settings: Res<Settings>,
    mut query: Query<(&ToggleOption, &mut TextColor, &mut BackgroundColor), With<ToggleOption>>,
    // Need to handle text queries separately — TextColor is a separate component from Text
    // Actually in Bevy 0.18, TextColor IS a component you can query. But Text itself also carries color info.
    // The correct approach: query TextColor (not Text) since we set colors via TextColor component.
) {
    for (option, mut text_color, mut bg) in query.iter_mut() {
        let active = match option.group.as_str() {
            "auto" => settings.auto_mode == option.value,
            "skip" => settings.skip_mode == option.value,
            _ => false,
        };
        if active {
            *text_color = TextColor(Color::WHITE);
            *bg = BackgroundColor(if option.value { Color::srgb(0.15, 0.3, 0.6) } else { Color::srgb(0.3, 0.12, 0.12) });
        } else {
            *text_color = TextColor(Color::srgb(0.4, 0.4, 0.5));
            *bg = BackgroundColor(Color::srgb(0.12, 0.12, 0.18));
        }
    }
}

fn cleanup_settings(
    mut commands: Commands,
    query: Query<Entity, With<SettingsScreen>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
```

- [ ] **Verify compilation:**

```bash
cargo check
```

Expected: compiles. If there are import errors (e.g., `NextState`, `Color` methods), check Bevy 0.18 API and fix.

- [ ] **Commit:**

```bash
git add src/plugins/settings.rs
git commit -m "feat: rewrite Settings plugin with interactive sliders and toggles"
```

---

### Task 3: Wire audio volume settings to runtime

**Files:**
- Modify: `src/plugins/audio.rs`

- [ ] **Update `handle_play_bgm`** to add `AudioType::Bgm` marker component to BGM entity.
- [ ] **Update `handle_play_se`** to add `AudioType::Se` marker component to SE entity.
- [ ] **Update `handle_play_voice`** to add `AudioType::Voice` marker component to voice entity.
- [ ] **Add `apply_audio_settings` system** that reads `Settings` and applies volume to all active `AudioSink` entities.

```rust
use bevy::{audio::Volume, prelude::*};
use crate::audio_messages::*;
use crate::components::AudioType;
use crate::resources::{BgmManager, Settings};

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
                apply_audio_settings,
            ));
    }
}

fn handle_play_bgm(
    mut reader: MessageReader<PlayBgmMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
) {
    for msg in reader.read() {
        if let Some(entity) = bgm.entity.take() {
            commands.entity(entity).despawn();
        }

        let path = format!("audio/bgm/bgm_{}_a.ogg", msg.id);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        let volume = msg.volume.unwrap_or(1.0);
        let entity = commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::Linear(volume),
                ..default()
            },
            AudioType::Bgm,
        )).id();

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
        if let Some(entity) = bgm.entity.take() {
            commands.entity(entity).despawn();
        }
        bgm.current_id = None;
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
            AudioType::Se,
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
            AudioType::Voice,
        ));
    }
}

fn apply_audio_settings(
    settings: Res<Settings>,
    query: Query<(&AudioType, &AudioSink)>,
) {
    for (audio_type, sink) in query.iter() {
        let volume = match audio_type {
            AudioType::Bgm => settings.bgm_volume,
            AudioType::Se => settings.se_volume,
            AudioType::Voice => settings.voice_volume,
        };
        sink.set_volume(Volume::Linear(volume));
    }
}
```

- [ ] **Verify compilation:**

```bash
cargo check
```

Expected: compiles.

- [ ] **Commit:**

```bash
git add src/plugins/audio.rs
git commit -m "feat: wire audio volume settings to runtime via AudioSink"
```

---

### Task 4: Wire text speed + auto/skip mode to ScriptRunner

**Files:**
- Modify: `src/plugins/script_runner.rs`

- [ ] **Replace hardcoded `chars_per_sec`** with `Settings.text_speed` in `update_text_reveal`.
- [ ] **Add `AutoSkipTimer` resource** for auto-advance and skip timers.
- [ ] **Add `handle_auto_skip` system** that runs before `process_advance`: when text is fully revealed and auto/skip mode is on, start a timer; on timer expiry, write `AdvanceEvent`.
- [ ] **Update plugin registration** to include the new resource and system with correct ordering.

```rust
use bevy::prelude::*;
use crate::resources::{AffectionMap, ChoiceState, DialogueState, Settings, UnlockState};
use crate::audio_messages::{
    PlayBgmMessage, StopBgmMessage, PlaySeMessage, PlayVoiceMessage,
};
use crate::choice_messages::ChoiceSelectedMessage;
use crate::script::{ConditionOp, ScriptCmd, ScriptEngine};
use crate::state::AppState;
use crate::plugins::inputs::AdvanceEvent;
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowCgMessage, HideCgMessage,
};

pub struct ScriptRunnerPlugin;

#[derive(Resource)]
pub struct AutoSkipTimer {
    pub auto_timer: Option<Timer>,
    pub skip_timer: Option<Timer>,
}

impl Default for AutoSkipTimer {
    fn default() -> Self {
        Self { auto_timer: None, skip_timer: None }
    }
}

impl Plugin for ScriptRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoSkipTimer>()
            .add_systems(OnEnter(AppState::Gameplay), start_script_execution)
            .add_systems(
                Update,
                (
                    handle_auto_skip,
                    process_advance,
                    update_text_reveal,
                )
                    .chain()
                    .run_if(in_state(AppState::Gameplay)),
            );
    }
}

fn start_script_execution(mut dialogue: ResMut<DialogueState>) {
    dialogue.current_text.clear();
    dialogue.current_speaker = None;
    dialogue.text_progress = 0;
    dialogue.is_displaying = false;
}

fn process_advance(
    mut advance_ev: MessageReader<AdvanceEvent>,
    mut engine: ResMut<ScriptEngine>,
    mut dialogue: ResMut<DialogueState>,
    mut affection: ResMut<AffectionMap>,
    mut unlock_state: ResMut<UnlockState>,
    mut set_bg_writer: MessageWriter<SetBgMessage>,
    mut show_fg_writer: MessageWriter<ShowFgMessage>,
    mut hide_fg_writer: MessageWriter<HideFgMessage>,
    mut show_cg_writer: MessageWriter<ShowCgMessage>,
    mut hide_cg_writer: MessageWriter<HideCgMessage>,
    mut choice_ev: MessageReader<ChoiceSelectedMessage>,
    mut choice_state: ResMut<ChoiceState>,
    mut play_bgm_writer: MessageWriter<PlayBgmMessage>,
    mut stop_bgm_writer: MessageWriter<StopBgmMessage>,
    mut play_se_writer: MessageWriter<PlaySeMessage>,
    mut play_voice_writer: MessageWriter<PlayVoiceMessage>,
    settings: Res<Settings>,
    mut auto_skip: ResMut<AutoSkipTimer>,
) {
    for _ in advance_ev.read() {
        // Reset auto/skip timers on manual advance
        auto_skip.auto_timer = None;
        auto_skip.skip_timer = None;

        // If choice is active, check if user made a selection
        if choice_state.active {
            for ev in choice_ev.read() {
                if ev.index < choice_state.options.len() {
                    let option = &choice_state.options[ev.index];
                    if let Some((ref char_id, delta)) = option.affection_change {
                        *affection.0.entry(char_id.clone()).or_insert(0) += delta;
                    }
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

        if dialogue.is_displaying && dialogue.text_progress < dialogue.current_text.len() {
            dialogue.text_progress = dialogue.current_text.len();
            continue;
        }

        if dialogue.is_displaying && dialogue.text_progress >= dialogue.current_text.len() {
            dialogue.is_displaying = false;
            continue;
        }

        // Skip mode: when not in a choice and skip is enabled, skip through everything
        if settings.skip_mode {
            while engine.has_more() {
                let cmd = engine.advance().cloned();
                match cmd {
                    Some(ScriptCmd::Dialogue { speaker, text }) => {
                        dialogue.current_speaker = speaker;
                        dialogue.current_text = text;
                        dialogue.text_progress = text.len();
                        dialogue.is_displaying = false;
                        // Don't break — continue processing in skip mode
                    }
                    Some(ScriptCmd::Choice { options }) => {
                        choice_state.active = true;
                        choice_state.options = options;
                        break;
                    }
                    Some(ScriptCmd::ClearText) => {
                        dialogue.current_text.clear();
                        dialogue.current_speaker = None;
                        dialogue.text_progress = 0;
                        dialogue.is_displaying = false;
                    }
                    Some(ScriptCmd::Jump { target }) => {
                        if !engine.jump_to_label(&target) {
                            warn!("Jump target not found: {}", target);
                        }
                    }
                    Some(ScriptCmd::Call { target }) => {
                        engine.call_label(&target);
                    }
                    Some(ScriptCmd::Return) => {
                        engine.return_from_call();
                    }
                    Some(ScriptCmd::Condition { var, value, operator, goto }) => {
                        let flag_val = engine.flags.get(&var).copied().unwrap_or(0);
                        let met = match operator {
                            ConditionOp::Greater => flag_val > value,
                            ConditionOp::Less => flag_val < value,
                            ConditionOp::Equal => flag_val == value,
                            ConditionOp::GreaterEqual => flag_val >= value,
                            ConditionOp::LessEqual => flag_val <= value,
                        };
                        if met && !engine.jump_to_label(&goto) {
                            warn!("Condition jump target not found: {}", goto);
                        }
                    }
                    Some(ScriptCmd::AffectionChange { char_id, delta }) => {
                        *affection.0.entry(char_id).or_insert(0) += delta;
                    }
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
                    Some(ScriptCmd::SavePoint) => {}
                    Some(ScriptCmd::UnlockCg { file }) => {
                        unlock_state.cg_unlocked.insert(file);
                    }
                    Some(ScriptCmd::SetBg { file, transition: _, duration: _ }) => {
                        set_bg_writer.write(SetBgMessage { file });
                    }
                    Some(ScriptCmd::ShowFg { char_id, expression, position, transition: _ }) => {
                        show_fg_writer.write(ShowFgMessage { char_id, expression, position });
                    }
                    Some(ScriptCmd::HideFg { char_id, transition: _ }) => {
                        hide_fg_writer.write(HideFgMessage { char_id });
                    }
                    Some(ScriptCmd::ShowCg { file, transition: _ }) => {
                        show_cg_writer.write(ShowCgMessage { file: file.clone() });
                        unlock_state.cg_unlocked.insert(file);
                    }
                    Some(ScriptCmd::HideCg { transition: _ }) => {
                        hide_cg_writer.write(HideCgMessage);
                    }
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
                    Some(cmd) => {
                        info!("Script cmd (no-op): {:?}", cmd);
                    }
                    None => break,
                }
            }
            if !engine.has_more() && !dialogue.is_displaying {
                info!("Script finished: {}", engine.current_script);
            }
            continue;
        }

        // Normal mode
        while engine.has_more() {
            let cmd = engine.advance().cloned();
            match cmd {
                Some(ScriptCmd::Dialogue { speaker, text }) => {
                    dialogue.current_speaker = speaker;
                    dialogue.current_text = text;
                    dialogue.text_progress = 0;
                    dialogue.is_displaying = true;
                    break;
                }
                Some(ScriptCmd::ClearText) => {
                    dialogue.current_text.clear();
                    dialogue.current_speaker = None;
                    dialogue.text_progress = 0;
                    dialogue.is_displaying = false;
                }
                Some(ScriptCmd::Jump { target }) => {
                    if !engine.jump_to_label(&target) {
                        warn!("Jump target not found: {}", target);
                    }
                }
                Some(ScriptCmd::Call { target }) => {
                    engine.call_label(&target);
                }
                Some(ScriptCmd::Return) => {
                    engine.return_from_call();
                }
                Some(ScriptCmd::Condition { var, value, operator, goto }) => {
                    let flag_val = engine.flags.get(&var).copied().unwrap_or(0);
                    let met = match operator {
                        ConditionOp::Greater => flag_val > value,
                        ConditionOp::Less => flag_val < value,
                        ConditionOp::Equal => flag_val == value,
                        ConditionOp::GreaterEqual => flag_val >= value,
                        ConditionOp::LessEqual => flag_val <= value,
                    };
                    if met && !engine.jump_to_label(&goto) {
                        warn!("Condition jump target not found: {}", goto);
                    }
                }
                Some(ScriptCmd::AffectionChange { char_id, delta }) => {
                    *affection.0.entry(char_id).or_insert(0) += delta;
                }
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
                Some(ScriptCmd::SavePoint) => {}
                Some(ScriptCmd::UnlockCg { file }) => {
                    unlock_state.cg_unlocked.insert(file);
                }
                Some(ScriptCmd::SetBg { file, transition: _, duration: _ }) => {
                    set_bg_writer.write(SetBgMessage { file });
                }
                Some(ScriptCmd::ShowFg { char_id, expression, position, transition: _ }) => {
                    show_fg_writer.write(ShowFgMessage { char_id, expression, position });
                }
                Some(ScriptCmd::HideFg { char_id, transition: _ }) => {
                    hide_fg_writer.write(HideFgMessage { char_id });
                }
                Some(ScriptCmd::ShowCg { file, transition: _ }) => {
                    show_cg_writer.write(ShowCgMessage { file: file.clone() });
                    unlock_state.cg_unlocked.insert(file);
                }
                Some(ScriptCmd::HideCg { transition: _ }) => {
                    hide_cg_writer.write(HideCgMessage);
                }
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
                Some(ScriptCmd::Choice { options }) => {
                    choice_state.active = true;
                    choice_state.options = options;
                    if settings.skip_mode {
                        // Disable skip mode when hitting a choice
                    }
                    break;
                }
                Some(cmd) => {
                    info!("Script cmd (no-op): {:?}", cmd);
                }
                None => break,
            }
        }

        if !engine.has_more() && !dialogue.is_displaying {
            info!("Script finished: {}", engine.current_script);
        }
    }
}

fn update_text_reveal(
    time: Res<Time>,
    mut dialogue: ResMut<DialogueState>,
    settings: Res<Settings>,
) {
    if dialogue.is_displaying && dialogue.text_progress < dialogue.current_text.len() {
        let chars_per_sec = (settings.text_speed as f64).max(1.0);
        let increment = (time.delta_secs_f64() * chars_per_sec) as usize;
        dialogue.text_progress = (dialogue.text_progress + increment).min(dialogue.current_text.len());
    }
}

fn handle_auto_skip(
    time: Res<Time>,
    mut advance_ev: MessageWriter<AdvanceEvent>,
    mut auto_skip: ResMut<AutoSkipTimer>,
    dialogue: Res<DialogueState>,
    choice_state: Res<ChoiceState>,
    settings: Res<Settings>,
) {
    // If choice is active, don't auto-advance
    if choice_state.active {
        auto_skip.auto_timer = None;
        auto_skip.skip_timer = None;
        return;
    }

    // Check if text is fully revealed
    let text_fully_displayed = !dialogue.is_displaying
        || dialogue.text_progress >= dialogue.current_text.len();

    if !text_fully_displayed || dialogue.current_text.is_empty() {
        auto_skip.auto_timer = None;
        auto_skip.skip_timer = None;
        return;
    }

    // Auto mode: start 2-second timer
    if settings.auto_mode && !settings.skip_mode {
        let timer = auto_skip.auto_timer.get_or_insert_with(|| {
            Timer::from_seconds(2.0, TimerMode::Once)
        });
        timer.tick(time.delta());
        if timer.just_finished() {
            advance_ev.write(AdvanceEvent);
            auto_skip.auto_timer = None;
        }
    }

    // Skip mode: start 0.5-second timer (only after full text reveal)
    if settings.skip_mode {
        let timer = auto_skip.skip_timer.get_or_insert_with(|| {
            Timer::from_seconds(0.5, TimerMode::Once)
        });
        timer.tick(time.delta());
        if timer.just_finished() {
            auto_skip.skip_timer = None;
            advance_ev.write(AdvanceEvent);
        }
    }
}
```

- [ ] **Verify compilation:**

```bash
cargo check
```

Expected: compiles. Note: `time.delta_secs()` was changed to `time.delta_secs_f64()` for use with `f64` multiplication with `text_speed` (which is an `f64` after cast).

- [ ] **Commit:**

```bash
git add src/plugins/script_runner.rs
git commit -m "feat: wire text speed, auto mode, and skip mode to ScriptRunner"
```

---

### Task 5: Wire message window opacity to dialogue background

**Files:**
- Modify: `src/plugins/dialogue.rs`

- [ ] **Add `apply_message_opacity` system** that reads `Settings.message_window_opacity` and updates the dialogue box `BackgroundColor` alpha each frame.

```rust
use bevy::prelude::*;
use crate::components::*;
use crate::resources::{DialogueState, Settings};
use crate::state::AppState;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueState>()
            .add_systems(OnEnter(AppState::Gameplay), setup_dialogue_ui)
            .add_systems(Update, (
                update_dialogue,
                apply_message_opacity,
            ).run_if(in_state(AppState::Gameplay)))
            .add_systems(OnExit(AppState::Gameplay), cleanup_dialogue);
    }
}

fn setup_dialogue_ui(mut commands: Commands) {
    commands.spawn((
        DialogueUiRoot,
        DialogueBox,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(200.0),
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexStart,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ZIndex(3),
    ));

    commands.spawn((
        DialogueUiRoot,
        SpeakerNameDisplay,
        Text::new(""),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.8, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(200.0),
            left: Val::Px(40.0),
            ..default()
        },
        ZIndex(3),
    ));

    commands.spawn((
        DialogueUiRoot,
        DialogueTextDisplay,
        Text::new(""),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            width: Val::Percent(95.0),
            ..default()
        },
        ZIndex(3),
    ));
}

fn cleanup_dialogue(mut commands: Commands, query: Query<Entity, With<DialogueUiRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn update_dialogue(
    state: Res<DialogueState>,
    mut text_query: Query<&mut Text, (With<DialogueTextDisplay>, Without<SpeakerNameDisplay>)>,
    mut speaker_query: Query<&mut Text, (With<SpeakerNameDisplay>, Without<DialogueTextDisplay>)>,
) {
    if let Ok(mut text) = text_query.single_mut() {
        let end = state.text_progress.min(state.current_text.len());
        text.0 = state.current_text[..end].to_string();
    }
    if let Ok(mut speaker) = speaker_query.single_mut() {
        speaker.0 = state.current_speaker.clone().unwrap_or_default();
    }
}

fn apply_message_opacity(
    settings: Res<Settings>,
    mut query: Query<&mut BackgroundColor, (With<DialogueBox>, Without<DialogueUiRoot>)>,
) {
    let alpha = (settings.message_window_opacity as f32) / 100.0;
    for mut bg in query.iter_mut() {
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, alpha));
    }
}
```

- [ ] **Verify compilation:**

```bash
cargo check
```

Expected: compiles.

- [ ] **Commit:**

```bash
git add src/plugins/dialogue.rs
git commit -m "feat: wire message window opacity to dialogue background"
```

---

### Task 6: Build and smoke test

**Files:** None (full project)

- [ ] **Full build:**

```bash
cargo build 2>&1
```

Expected: no errors. Pre-existing warnings are OK.

- [ ] **Smoke test (launch + 5 sec):**

```bash
timeout 6 ./target/debug/bevy-vn 2>&1 || true
```

Expected: game window appears, no panic. Warnings about missing assets/Entity despawned are expected.

- [ ] **Update PROGRESS.md** to mark Phase 7 Settings complete.

```markdown
### Phase 7: 设置 + 打磨
- [x] Settings interactivity (sliders + toggles wired to runtime)
- [ ] 过渡动画系统 (deferred to sub-phase)
- [ ] Android 适配 (deferred to sub-phase)
```

- [ ] **Commit:**

```bash
git add PROGRESS.md
git commit -m "feat: complete Phase 7 settings interactivity"
```
