# Settings Interactivity Design

## Overview
Replace the current Settings placeholder (just text) with a full interactive settings screen. Wire settings to affect runtime behavior: audio volume, text speed, auto/skip mode, message window opacity.

## UI Layout
Full-screen dark overlay, centered vertical list, same visual style as the Menu:

```
┌──────────────────────────────────────┐
│  ← Back                              │
│                                      │
│           Settings                   │
│                                      │
│  BGM Volume    [■■■■■■■□□□] 70%     │
│  SE Volume     [■■■■■□□□□□] 50%     │
│  Voice Volume  [■■■■■■■□□□] 70%     │
│  Text Speed    [■■■■□□□□□□] 40%     │
│  Msg Opacity   [■■■■■■■□□□] 70%     │
│                                      │
│  Auto Mode     [●] ON   [○] OFF     │
│  Skip Mode     [○] ON   [●] OFF     │
└──────────────────────────────────────┘
```

### Slider control
- Each slider is a horizontal bar divided into 10 clickable segments
- Each segment is a separate entity with a `SliderSegment(usize)` component
- Filled segments are colored (e.g. white/blue), unfilled are dim
- Clicking segment N sets value to N*10 (for 0-100% range)
- Current numeric value displayed as text to the right

### Toggle control
- Two radio-button style text buttons: ON / OFF
- Clicking toggles the boolean in the Settings resource
- Active state is highlighted (white), inactive is dim (gray)

### Back button
- "← Back" text button at top left
- Click transitions `AppState::Settings` → `AppState::Menu`

## Component Markers
New components in `src/components.rs`:
- `SettingsScreen` — root entity (already exists, repurposed)
- `SettingsBackButton` — back button
- `SliderLabel(String)` — label for a slider row
- `SliderSegment(usize)` — one of 10 clickable segments
- `SliderValueText` — displays current numeric value
- `ToggleOption(String)` — ON or OFF text
- `ToggleGroup(String)` — groups two toggles for the same setting (e.g. "auto", "skip")

## Runtime Wiring

### Audio Volume
- `AudioPlugin`: new system `apply_audio_settings` runs in Update
- NOT gated by any state — volume changes take effect in Menu/Settings too
- Reads `Settings.bgm_volume`, `Settings.se_volume`, `Settings.voice_volume`
- Queries `BgmManager.entity` → `AudioSink.set_volume(Volume::Linear(v))`
- Queries for active SE/Voice entities with `AudioSink` component, sets volume
- Runs at low frequency (every 30 frames) to avoid churn

### Text Speed
- `ScriptRunner` reads `Settings.text_speed` on each advance
- Text reveal timer uses `text_speed` as ms per character (default 40)
- Lower value = faster text (10 = near-instant, 100 = slow)

### Auto Mode
- When `Settings.auto_mode` is true and text is fully revealed, start a timer
- After 2 seconds, auto-trigger `AdvanceEvent`
- If a Choice is active, auto mode pauses until choice is made
- Toggle auto mode OFF on any manual click

### Skip Mode
- When `Settings.skip_mode` is true, complete all text instantly
- Auto-advance after 0.5 seconds
- Skip mode turns OFF automatically when reaching a Choice

### Message Window Opacity
- `DialoguePlugin`: store dialogue BG entity in a resource
- System applies `Settings.message_window_opacity` as alpha to `BackgroundColor`
- Range 0–100 maps to alpha 0.0–1.0

## Data Flow
```
User click → SliderSegment/ToggleOption system 
  → updates Settings resource
    → apply_audio_settings reads Settings → AudioSink
    → ScriptRunner reads Settings.text_speed on advance
    → DialoguePlugin reads Settings.message_window_opacity
    → Auto mode timer system reads Settings.auto_mode
    → Skip mode system reads Settings.skip_mode
```

## Files Changed
| File | Change |
|------|--------|
| `src/components.rs` | +6 new marker components |
| `src/plugins/settings.rs` | Full rewrite — real UI + interaction systems |
| `src/plugins/audio.rs` | +`apply_audio_settings` system |
| `src/plugins/script_runner.rs` | Read text_speed, auto/skip mode |
| `src/plugins/dialogue.rs` | Store bg entity, apply opacity |
| `src/resources.rs` | No changes needed |
| `src/main.rs` | No changes needed |

## Not in Scope
- Persistent settings save (write to disk on change, load on boot) — deferred
- Volume mute button — would add later
- Advanced audio options (per-channel EQ, etc.)
