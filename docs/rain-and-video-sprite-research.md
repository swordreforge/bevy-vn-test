# Rain Effects & Sprite Video Overlay — ASB Tag Research

## Overview

Artemis engine uses two distinct video subsystems:

1. **`PlayMovie`** — full-screen cinematic video (opening, ending). Already mapped.
2. **`DrawSpriteEx`** — sprite-based video overlay, rendered as a textured sprite on the overlay layer. **Not mapped.**

Rain effects are a special case of sprite video overlay, controlled by a dedicated set of parameter tags culminating in `rain_mja`.

---

## 1. `DrawSpriteEx` — Sprite Video Overlay

### Purpose
Plays a video file as a textured sprite on the overlay/sprite layer, not full-screen. Used for scene-specific animated overlays (weather, atmospheric effects, dream sequences).

### Usage (6 scripts)
| File | attr[1] (file) | Corresponding .ogv |
|---|---|---|
| `aiy80020.asb` | `aiy80020mov.mpg` | `aiy80020mov.ogv` |
| `aiy50050.asb` | `aiy50050mov.mpg` | `aiy50050mov.ogv` |
| `aiy20230.asb` | `aiy20230mov.mpg` | `aiy20230mov.ogv` |
| `aiy30150.asb` | `aiy30150mov.mpg` | `aiy30150mov.ogv` |
| `aiy30180.asb` | `aiy30180mov.mpg` | `aiy30180mov.ogv` |
| `aiy81010.asb` | `aiy81010mov.mpg` | `aiy81010mov.ogv` |

### Raw ASB Attributes

```
DrawSpriteEx:
  attr[0]  = sprite_id     "01" / "02"
  attr[1]  = file          "aiy80020mov.mpg"  (video filename)
  attr[2]  = mode          "03" / "04"         (playback/display mode)
  attr[3]  = transition    "NULL"
  attr[4]  = x             "000"               (position)
  attr[5]  = y             "000"
  attr[6]  = z/rotation    "000"
  attr[7]  = width         "640"               (display width)
  attr[8]  = height        "360"               (display height)
  attr[9]  = ?             "00"
  attr[10] = ?             "00"
  attr[11] = ?             "00"
  attr[12] = visible       "TRUE"
  attr[13] = ?             "TRUE"
  attr[14] = display_mode  "00"/"01"/"03"      (aspect / stretch mode)
  attr[15] = ?             "000"
  attr[16] = priority      "01"/"02"           (layer order)
  attr[17] = ?             "000"
  attr[18] = wait          "TRUE"/"FALSE"      (block script until done)
```

### Attribute Analysis

- **attr[0]**: Sprite ID, same namespace as `DrawSprite`/`FadeSprite`. When `"02"`, the video replaces/overlays sprite "02".
- **attr[1]**: Video file path, same `.mpg` convention as `PlayMovie`. Extension must be remapped `.mpg` → `.ogv`.
- **attr[7]/[8]**: Display size (640×360 = half of 1280×720). Suggests the video is composited at a specific size, not necessarily full-screen.
- **attr[14]**: Display mode — `"00"` may be stretch-to-fill, `"01"` may be aspect-ratio-preserve, `"03"` may be center/original-size.
- **attr[18]**: If `"TRUE"`, script waits for video completion before proceeding. Followed by `WaitToFinishMoviePlayingOnSprite` which is already mapped as `Wait { duration: 0 }`.

### Follow-up Tags

- **`WaitToFinishMoviePlayingOnSprite`**: Already mapped to `Wait { duration: 0 }` — a no-op. Should be changed to a blocking wait that checks if the `DrawSpriteEx` video has finished. OR, use attr[18] as the blocking mechanism instead.
- **`WaitToFinishSpriteControlling`**: Present in `aiy81010.asb`. Additional synchronization for sprite transform completion.

---

## 2. Rain Effects System

### Architecture
Rain is a particle/overlay system driven by 7 tags that work together. The video files in `assets/movie/` (`aiy*_rain.ogv`) are the visual assets, rendered as rotating/scrolling transparent overlays.

### Tag Reference

#### `rain_mja` — Trigger Rain Overlay
Starts rain playback. The "mja" suffix likely refers to the original Japanese name.

```
rain_mja:
  attr[file]     = "aiy20120_rain"       # base name (no extension)
  attr[loop]     = "aiy20200_rain_2"     # secondary loop file (optional)
  attr[priority] = "10" / "2" / "0"      # rendering layer priority
  attr[time]     = "9250"                # duration in ms? (only in aiy20200)
```

- `file`: The primary rain overlay video. Renders `assets/movie/{file}.ogv`.
- `loop`: A secondary video to loop/cycle with (only used in `aiy20200.asb`).
- `priority`: Lower numbers render behind characters/CG, higher numbers render in front.
- `time`: Optional duration override. Default behaviour: runs until `SetValidityOfRainfall(FALSE)`.

#### `SetValidityOfRainfall` — Enable/Disable
```
SetValidityOfRainfall:
  attr[0] = "TRUE" / "FALSE"     # enable/disable rain rendering
  attr[1] = "NULL"               # (always NULL in observed data)
  attr[2] = "000"                # fade time? (always "000")
```

#### `SetQuantityOfRainfall` — Density
```
SetQuantityOfRainfall:
  attr[0] = "100" / "200" / "400"   # density value
```
Higher values = denser rain. Observed values: 100 (light), 200 (moderate), 400 (heavy).

#### `SetColorOfRain` — Tint Color
```
SetColorOfRain:
  attr[0] = "194"   # R
  attr[1] = "194"   # G
  attr[2] = "194"   # B
  attr[3] = "194"   # A
```
RGBA values 0-255. All observed samples use 194 (light grey). Can be used to tint the rain overlay (e.g., blue-tinted night rain).

#### `SetVectorOfSightForRainfall` — Perspective Direction
```
SetVectorOfSightForRainfall:
  attr[0] = "000"   # direction/orientation
```
Controls the falling angle of rain based on viewing perspective. Always "000" in observed data.

#### `SetCameraAngleOfRainfall` — 3D Camera Angle
```
SetCameraAngleOfRainfall:
  attr[0] = "00"    # x angle
  attr[1] = "00"    # y angle
  attr[2] = "00" / "20"  # z angle (tilt)
```
Applied to the rain overlay for pseudo-3D perspective. Usually all zeros; `"20"` for z was observed in `aiy20210.asb`.

#### `SetPriorityOfRainfallScreen` — Render Layer
```
SetPriorityOfRainfallScreen:
  attr[0] = "00" / "02"     # priority layer
```
Controls which sprite layer the rain overlay renders on. Usually "00" (behind characters/CG); "02" in some scenes.

### Rain Script Flow (Typical Pattern)

```
SetColorOfRain(R=194, G=194, B=194, A=194)    # configure tint
SetVectorOfSightForRainfall(000)               # configure angle
SetQuantityOfRainfall(200)                     # configure density
SetValidityOfRainfall(TRUE)                    # enable rain layer
rain_mja(file="aiy20120_rain", priority=10)   # start video playback
... (scene dialogue)
SetValidityOfRainfall(FALSE)                   # stop rain
```

### Rain Scripts & Asset Mapping

| Script | asset file | loop file |
|---|---|---|
| `aiy20120.asb` | `aiy20120_rain.ogv` | — |
| `aiy20200.asb` | `aiy20200_rain.ogv` | `aiy20200_rain_2.ogv` |
| `aiy20210.asb` | `aiy20210_rain.ogv`, `aiy20210_rain_2.ogv` | — |
| `aiy71410.asb` | `aiy71410_rain.ogv` | — |

All 6 `_rain.ogv` files are now accounted for.

---

## 3. `MovieInit` — Full-screen Video Initialization

### Purpose
Precedes `PlayMovie` in `aiy00150.asb` and `aiy50010.asb`. Initializes the video playback subsystem before the actual `PlayMovie` command.

```
MovieInit:
  (no attributes)
```

### Mapping Strategy
Should be mapped to a no-op or a resource init. Since our GStreamer pipeline is lazily initialized in `check_video_completion()`, no runtime action is needed — but the mapper should still emit a variant so it doesn't appear as a skipped/unknown tag.

---

## 4. Other Unmapped Tags (Minor)

Found in rain/video ASB files but not essential for the core rain/video pipeline:

| Tag | Found in | Notes |
|---|---|---|
| `DrawBustshotWithFiltering` | `aiy81010.asb` | Similar to `DrawSpriteWithFiltering` |
| `blur_set` | `aiy00150.asb` | Screen blur effect |
| `ChangeBustshot` | `aiy30180.asb` | Character bust shot change |
| `RegisterTextToHistory` | `aiy20210.asb` | Text history logging |
| `CallSavingWindow` | `aiy20230.asb` | Save prompt |
| `WaitForInput` | `aiy81010.asb` | Wait for any key |
| `WaitToFinishVoicePlaying` | `aiy20210.asb` | Wait for voice clip end |
| `WaitToFinishSEPlaying` | `aiy80020.asb` | Wait for SE clip end |
| `WaitToFinishSpriteControlling` | `aiy81010.asb` | Wait for sprite animation |
| `GetExecutionMode` | `aiy71410.asb` | Check skip/auto mode |
| `CancelSkipping` | `aiy80020.asb` | Force cancel skip mode |
| `ScrollBG` | `aiy50010.asb` | (already mapped via calllua) |
| `StartShakingOfAllObjects` | `aiy50010.asb` | Screen shake |
| `TerminateShakingOfAllObjects` | `aiy50010.asb` | Stop screen shake |
| `StartShakingOfSprite` | `aiy30180.asb` | Sprite-specific shake |
| `CheckQuickJump` | `aiy20200.asb` | Quick jump flag |
| `ChangeVolumeOfStreamingSE` | `aiy20210.asb` | SE volume control |
| `DisableGradation` | `aiy20200.asb` | Disable gradient |
| `EnableHorizontalGradation` | `aiy20200.asb` | Enable horiz gradient |
| `MoveBustshot` | `aiy20200.asb` | Move bust shot sprite |
| `size` / `Size` | many | Window resize |

---

## 5. Summary: All 12 `.ogv` Files Mapped

| File | System | Tag | Scripts |
|---|---|---|---|
| `aiy50010mov.ogv` | Full-screen video | `PlayMovie` | `aiy50010` |
| `movie.ogv` (missing) | Full-screen video | `PlayMovie` | `aiy00150` |
| `aiy20230mov.ogv` | Sprite video | `DrawSpriteEx` | `aiy20230` |
| `aiy30150mov.ogv` | Sprite video | `DrawSpriteEx` | `aiy30150` |
| `aiy30180mov.ogv` | Sprite video | `DrawSpriteEx` | `aiy30180` |
| `aiy50050mov.ogv` | Sprite video | `DrawSpriteEx` | `aiy50050` |
| `aiy80020mov.ogv` | Sprite video | `DrawSpriteEx` | `aiy80020` |
| `aiy81010mov.ogv` | Sprite video | `DrawSpriteEx` | `aiy81010` |
| `aiy20120_rain.ogv` | Rain overlay | `rain_mja` | `aiy20120` |
| `aiy20200_rain.ogv` | Rain overlay | `rain_mja` | `aiy20200` |
| `aiy20200_rain_2.ogv` | Rain overlay | `rain_mja` (loop) | `aiy20200` |
| `aiy20210_rain.ogv` | Rain overlay | `rain_mja` | `aiy20210` |
| `aiy20210_rain_2.ogv` | Rain overlay | `rain_mja` | `aiy20210` |
| `aiy71410_rain.ogv` | Rain overlay | `rain_mja` | `aiy71410` |

---

## 6. Implementation Plan (Next Steps)

### Phase A: Mapper Changes (`mapper.rs`)

1. Add `MovieInit` → `ScriptCmd::MovieInit` (no-op variant or resource signal)
2. Add `DrawSpriteEx` → `ScriptCmd::DrawSpriteEx { id, file, x, y, width, height, display_mode, priority, wait }` (map relevant attrs)
3. Add `rain_mja` → `ScriptCmd::RainMja { file, loop_file, priority, time }`
4. Add all 6 rain parameter tags (consolidate into `ScriptCmd::RainParam { ... }` or individual commands)
5. Update `WaitToFinishMoviePlayingOnSprite` → proper blocking wait (or rely on `DrawSpriteEx.wait`)

### Phase B: `script.rs` — New Variants

Add new `ScriptCmd` variants:
- `MovieInit`
- `DrawSpriteEx { id, file, width, height, display_mode, priority, wait }`
- `RainMja { file, loop_file: Option<String>, priority: i32, time: Option<u64> }`
- `SetRainValid { enabled: bool }`
- `SetRainQuantity { density: u32 }`
- `SetRainColor { r: u8, g: u8, b: u8, a: u8 }`
- `SetRainVector { direction: u32 }`
- `SetRainCameraAngle { x: u32, y: u32, z: u32 }`
- `SetRainPriority { priority: u32 }`

### Phase C: `script_runner.rs` — Runtime Handlers

Each new variant needs a handler in both skip and normal execution paths.

### Phase D: Rendering — Rain System

Implement actual rain rendering: parse the rain .ogv video (or particle system), render as an overlay with position/color/density parameters.

### Phase E: Rendering — `DrawSpriteEx` Video Overlay

Use GStreamer appsink to decode the video and update a sprite texture, similar to `PlayMovie` but as a sprite overlay instead of full-screen.
