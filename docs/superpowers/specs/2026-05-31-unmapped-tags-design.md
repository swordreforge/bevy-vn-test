# Unmapped ASB/IET Tags — Mapping Design

Date: 2026-05-31
Status: Draft

## Overview

Eliminate all `_ => None` catch-all skips in the Artemis script converter by mapping every
currently-unhandled tag to a `ScriptCmd` variant. ~88 unique tag types generating ~35,000 skips
across the full script corpus.

**Goal:** Every tag in every `.asb` / `.iet` file produces a `ScriptCmd`, never falls through to
`None` / `[skip]`. Game-flow-affecting tags get real handlers; visual-only / legacy / Artemis
engine internals get no-op `ScriptCmd` variants with `info!` logging.

## Strategy

Group tags by function, map each group to either an existing `ScriptCmd` variant (reuse) or one
or two new variants (newgroup). Implementation order follows priority within the 4 groups.

## Group 1 — Game Logic (P0–P1)

Tags that affect script flow, state, or save data. Each needs a real runtime handler.

### G1A — TerminateExecutionOfScript → `Halt`

**Mapper:** add single line mapping the tag name.
**Runtime:** `Halt` handler already exists (clears script stack + index). No changes.

Only affects `.asb` files (IET already uses `[TerminateExecutionOfScript]` → `Halt`).

### G1B — SEStop → new `StopAllSe`

**Problem:** 2,530 occurrences. `StopStreamingSe` only stops a specific channel. Artemis
`SEStop` stops all SE (sound effects, including oneshot and streaming).

**New variant:**
```rust
StopAllSe,
```

**Mapper:**
```rust
"SEStop" => Some(ScriptCmd::StopAllSe),
```

**Runtime (both paths):** iterate `audio_state.streaming_se` and stop each channel.
```rust
ScriptCmd::StopAllSe => {
    audio_state.streaming_se.clear();
}
```

### G1C — RegisterTextToHistory → new `PushHistory`

**Problem:** 42 occurrences. Artemis expects the engine to push dialogue text to a history
buffer for the backlog feature. Currently the backlog plugin (`src/plugins/backlog.rs`)
exists but has no data source.

**New variant:**
```rust
PushHistory,
```

**Runtime approach:** When the script runner encounters `PushHistory`, it pushes the most
recent dialogue text to a `BacklogHistory` resource (list of `BacklogEntry`). If there is
no pending dialogue, it is a no-op.

**Mapper:**
```rust
"RegisterTextToHistory" => Some(ScriptCmd::PushHistory),
```

### G1D — WaitToFinishVoicePlaying → `WaitVoice`

**Problem:** 30 occurrences. Artemis blocks until voice playback finishes. Without a
separate audio thread, the simplest mapping is a fixed-duration `Wait`.

**Approach:** Check `audio_state.voice_playing` at runtime. If a voice file is loaded and
playing, compute remaining duration from the audio file metadata (or use a
configurable default: 2000ms). Fall back to `Wait { duration: 0 }` (immediate continue)
when no voice is active.

**New variant:**
```rust
WaitVoice,
```

**Mapper:**
```rust
"WaitToFinishVoicePlaying" => Some(ScriptCmd::WaitVoice),
```

**Runtime:**
```rust
ScriptCmd::WaitVoice => {
    if let Some(remaining) = get_voice_remaining_duration(&audio_state) {
        auto_timer = Some(Timer::from_seconds(remaining, TimerMode::Once));
        break;
    }
}
```
For the initial implementation, `Wait { duration: 2000 }` as a hardcoded fallback.

### G1E — GetExecutionMode → new `QueryMode`

**Problem:** 41 occurrences. Artemis queries whether the engine is in skip/auto mode and
stores the result (typically for conditional branches). Our skip/auto state lives in
`Settings.skip_mode` / `Settings.auto_mode`.

**New variant:**
```rust
QueryMode { mode: String },
```

**Mapper:** parse the attribute string to determine which mode to check.
```rust
"GetExecutionMode" => Some(ScriptCmd::QueryMode { mode: attr.to_string() }),
```

**Runtime:** read `settings.skip_mode` and `settings.auto_mode`, set a local flag or
`tmp` variable accordingly. Currently just sets `tmp = 0` (normal mode) as a stub.

### G1F — exif → new `Exif`

**Problem:** 79 occurrences. Artemis conditional execution tag. Format:
```
[exif exp="t.tmp == 100"]
  [calllua ...]
[exif exp="t.tmp != 0"]
```

If the expression evaluates to false, the subsequent command (typically `calllua`) is
skipped. This is inline conditional execution (not block-structured like `if/endif`).

**New variant:**
```rust
Exif { expression: String },
```

**Mapper:** parse the `exp` attribute directly.
```rust
"exif" => Some(ScriptCmd::Exif { expression: exp_attr }),
```

**Runtime:** evaluate the expression using `evaluate_script_expression()` (or a new
`evaluate_condition_expression()` that supports comparison operators). If false, skip
the next command by advancing `engine.script_index` by one additional position.

### G1G — BgmX → `PlayBgmX` / `StopBgmX`

**Problem:** 7 occurrences. Raw `BgmX` tag in ASB files (the existing converter also
handles this via `calllua bgm_play_x`). Map to existing variants.

**Approach:** Parse `BgmX` arguments (id, volume, fade) → `PlayBgmX`. Parse `BgmX stop`
→ `StopBgmX`.

```rust
"BgmX" => {
    if attrs.contains("stop") || attrs.contains("0") {
        Some(ScriptCmd::StopBgmX { id: 0, fade_out: 0 })
    } else {
        Some(ScriptCmd::PlayBgmX { id: parse_id(attrs), volume: 100, fade_in: 0 })
    }
}
```

## Group 2 — Audio (P2)

### G2A — ChangeVolumeOfBGM → `BgmVol`

Existing `BgmVol` variant already handles BGM volume. Add mapper line.

```rust
"ChangeVolumeOfBGM" | "ChangeVolumeOfBGMX" => map_bgmvol(attrs),
```

### G2B — FadeOutBGM → `StopBgm` with fade

```rust
"FadeOutBGM" => Some(ScriptCmd::StopBgm { id: parse_id(attrs), fade_out: parse_fade(attrs) }),
```

### G2C — ChangeVolumeOfStreamingSE → new `StreamingSeVol`

```rust
StreamingSeVol { id: u32, volume: u32 },
```

### G2D — FadeOutStreamingSE → `StopStreamingSe` with params

```rust
"FadeOutStreamingSE" => Some(ScriptCmd::StopStreamingSe { channel: parse_channel(attrs) }),
```

## Group 3 — Visuals (P2–P3)

### G3A — DrawBG → `SetBg`

DrawBG is the "raw" Artemis background command (as opposed to `Back`/`Fadeout` sequence).
Map directly to `SetBg`:

```rust
"DrawBG" => {
    let file = parse_str_attr(attrs, 0);
    Some(ScriptCmd::SetBg { file, transition: Transition::Instant, duration: 0 })
}
```

### G3B — ScrollBG → `ScrollBg`

Similar to existing `calllua scroll_bg` handler but as a native tag.

```rust
"ScrollBG" => parse_scroll_bg(attrs),
```

### G3C — DrawSpriteEx → `DrawSprite`

Extended sprite drawing (additional filtering/blending parameters). Map to `DrawSprite`
with default parameters for the extra fields.

```rust
"DrawSpriteEx" | "DrawSpriteWithFiltering" => map_draw_sprite(attrs),
```

### G3D — DrawBustshot / DrawBustshotWithFiltering → `ShowFg`

Character bustshot (tachi-e / standing sprite) with filtering options.

```rust
"DrawBustshot" | "DrawBustshotWithFiltering" | "ChangeBustshot" => map_show_fg(attrs),
```

### G3E — blur_set → new `Blur` (no-op)

```rust
Blur { power: u32 },
```

**Runtime:** no-op (visual effect, deferred).

### G3F — Rain effects → new `RainParam` family

Tags: `SetColorOfRain`, `SetQuantityOfRainfall`, `SetVectorOfSightForRainfall`,
`SetPriorityOfRainfallScreen`, `SetCameraAngleOfRainfall`, `SetValidityOfRainfall`.

```rust
RainParam { param: RainParamType, value: String },
```

**Runtime:** no-op (rain particle system planned as separate phase).

### G3G — Screen Shake → new `ShakeScreen` family

Tags: `StartShakingOfAllObjects`, `StartShakingOfSprite`, `TerminateShakingOfAllObjects`,
`TerminateShakingOfSprite`, `ShakeScreenSx`.

```rust
ShakeScreen { power: u32, time: u32 },
ShakeSprite { id: u32, power: u32, time: u32 },
```

**Runtime:** map `ShakeScreen` to call `ScreenShake` plugin if present; no-op otherwise.

### G3H — SetColorOfMonologue → new `MonologueColor`

```rust
MonologueColor { color: String },
```

**Runtime:** no-op (UI styling, deferred).

### G3I — Visual tween / transition stubs

Tags: `tween` (positional tweening), `FadeScene` (scene-wide fade), `MoveBustshot`,
`FadeBustshot`, `lytweendel`.

All mapped as no-op `ScriptCmd` variants:
```rust
Tween { args: String },
FadeScene { color: String, time: u32 },
```

### G3J — DrawMusicTelop → no-op

Music title overlay. Not needed in this engine (audio is non-diegetic).

## Group 4 — Engine / UI / Legacy (no-op)

All tags that are Artemis engine internals, UI features we don't use, or legacy formats:
`Size`, `flip`, `lyprop`, `lydel`, `lyc`, `lyc2`, `lyevent`, `trans`, `sys_trans`,
`CallSavingWindow`, `BgmShow`, `BgmNo`, `MovieInit`, `ResetSc`, `stop`, `exstop`,
`set_vsync`, `push_check`, `tweetmode`, `chgmsg`, `/chgmsg`, `rp2`, `ruby`, `/ruby`,
`SetJumpLabel`, `CheckQuickJump`, `CancelSkipping`, `SetValidityOfSkipping`,
`WaitToFinishSpriteControlling`, `EnableHorizontalGradation`, `DisableGradation`,
`DisableWindowEx`, `WaitForInput`, `script`, `wait`, `st`, `wt`, `btn_stop`, `btnon`,
`btn_start`, `allkeystart`, `allkeystop`, `allkeyclick`, `rain_mja`, `rpx`, `sestop`,
`SetEndOfScene`, `shake50300`, `calllua (1)`.

**Mapping pattern (mapper.rs):**
```rust
// Group 4 — Engine/UI/Legacy no-ops
"Size" | "flip" | "lyprop" | "lydel" | "lyc" | "lyc2" | "trans" | "sys_trans"
    | ... => Some(ScriptCmd::NoOp { tag: tag.to_string() }),
```

**New variant:**
```rust
NoOp { tag: String },
```

**Runtime:** `info!("Skipping no-op tag: {}", tag);`

This ensures every tag produces a `ScriptCmd` while the `info!` log makes it easy to
identify any that should be promoted to a real handler.

## New `ScriptCmd` Variants Summary

| Variant | Kind | Group |
|---------|------|-------|
| `StopAllSe` | new | G1B |
| `PushHistory` | new | G1C |
| `WaitVoice` | new | G1D |
| `QueryMode { mode }` | new | G1E |
| `Exif { expression }` | new | G1F |
| `StreamingSeVol { id, volume }` | new | G2C |
| `Blur { power }` | new | G3E |
| `RainParam { param, value }` | new | G3F |
| `ShakeScreen { power, time }` | new | G3G |
| `ShakeSprite { id, power, time }` | new | G3G |
| `MonologueColor { color }` | new | G3H |
| `Tween { args }` | new | G3I |
| `FadeScene { color, time }` | new | G3I |
| `NoOp { tag }` | new | G4 |
| `ChangeVolumeOfBGM` | use existing `BgmVol` | G2A |
| `FadeOutBGM` | use existing `StopBgm` | G2B |

~14 new variants, ~10 existing reuse.

## Implementation Order

1. **script.rs** — add all new `ScriptCmd` variants first (mapper and runner depend on them).
2. **mapper.rs** — add ALL new tag mappings. Do all tags at once to eliminate all `[skip]`.
3. **iet.rs** — add any IET-equivalent mappings (most IET tags already handled;
   check for gaps).
4. **script_runner.rs** — add runtime handlers (both skip and normal paths) for
   all new variants. Group 4 (`NoOp`) gets a single catch-all arm.
5. **`cargo check`** — verify compilation.
6. **`cargo test -p artemis-export`** — verify 83+ tests still pass.
7. **Re-run converter** — verify zero `[skip]` lines in verbose output.

## Testing

- Existing 83 mapper tests continue to pass (no existing mapping changed).
- New mapper tests for each group:
  - At least 1 test per group 1 variant (realistic attribute input).
  - 1 test per group 3/4 tag family (representative input).
- Integration: run full converter with `--verbose`, grep for `[skip]` → expect zero.

## Open Questions

1. `GetExecutionMode` stub: what value should `tmp` get? Proposal: `0` for normal,
   `1` for skip, `2` for auto.
2. `exif` expression evaluation: reuse `evaluate_script_expression()` (which handles
   `t.tmp+N`) or add a new `evaluate_condition()` that handles `==`, `!=`, `>=`, etc.?
3. `WaitToFinishVoicePlaying`: realistic default duration? Proposal: 2000ms hardcoded,
   configurable later.
