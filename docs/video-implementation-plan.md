# Video System — Scientific Phased Implementation Plan

## Principle

Each phase produces a **working, testable artifact**. No phase depends on uncertain technology. Risk (native dependency complexity) is pushed to later phases.

---

## Phase 1 — Mapper Integration (1-2h)

**Goal**: `bscript.ron` files contain `PlayMovie` commands. **0 runtime changes.**

### Steps

1. Add `PlayMovie` handler in `mapper.rs`:

```rust
"PlayMovie" => {
    let file = cmd.attrs.get("0")?;
    Some(vec![ScriptCmd::PlayMovie { file: file.to_string() }])
}
```

2. Add `WaitToFinishMoviePlayingOnSprite` handler — map to `Wait { duration: 0 }`:

```rust
"WaitToFinishMoviePlayingOnSprite" => {
    Some(vec![ScriptCmd::Wait { duration: 0 }])
}
```
Rationale: `PlayMovie` already blocks script execution (once Phase 2 implements it). The "WaitToFinish" becomes a no-op guard — by the time script reaches it, the video is already done.

3. Add 2 exporter tests: `test_play_movie_tag` + `test_wait_to_finish_movie_tag`

### Deliverable

- `cargo test -p artemis-export` passes (82/82 tests)
- `cargo check` clean
- `aiy00150.bscript.ron` and `aiy50010.bscript.ron` contain `PlayMovie(...)` commands
- Files with rain `.ogv` (aiy20120, aiy20200, aiy20210, aiy71410) — NOT handled here (see Phase 5)
- File `aiy80020.asb` (sprite-based video) — NOT handled here (uses `DrawSprite`/`DrawSpriteEx`, fundamentally different mechanism)

**Verification**: grep for `PlayMovie` in generated bscript files.

---

## Phase 2 — Runner Stub (1-2h)

**Goal**: Script pauses at `PlayMovie` points with a deterministic delay. **No video playback yet.** Verifies the flow end-to-end.

### Steps

1. Add `PendingVideo` resource in `resources.rs`:

```rust
#[derive(Resource, Default)]
pub struct PendingVideo {
    pub playing: bool,
    pub elapsed_ms: u64,
}
```

2. Add `PlayMovie` handler in `script_runner.rs`:

```rust
ScriptCmd::PlayMovie { file } => {
    info!("🎬 Video would play: {} (→ {})", file, file.replace(".mpg", ".ogv"));
    video_state.playing = true;
    video_state.elapsed_ms = 0;
}
```

3. Add timer system (runs in `Update`):

```rust
fn tick_pending_video(time: Res<Time>, mut video_state: ResMut<PendingVideo>) {
    if video_state.playing {
        video_state.elapsed_ms += time.delta().as_millis() as u64;
        if video_state.elapsed_ms >= 3000 { // 3s stub duration
            video_state.playing = false;
        }
    }
}
```

4. Gate script advancement on `video_state.playing` (alongside existing `pending_commands` check).

### Deliverable

- `cargo check` clean
- Running the game: at movie points, script pauses for 3 seconds with a log message, then continues
- Dialogue window should be hidden during this pause (we can emit `Window { show: false }` before PlayMovie in the mapper, or handle it in the runner)

**Verification**: Manual playthrough reaches a movie point → see log + 3s pause → script continues.

---

## Phase 3 — Desktop Video Playback (3-5h)

**Goal**: Movies play visually on desktop. **Android NOT covered yet.**

### Dependency Analysis

`bevy_movie_player` v0.7.1 uses ffmpeg via `ffmpeg-next` crate. On desktop (Linux/macOS/Windows):

- **Linux**: ffmpeg system library (`libavformat`, `libavcodec`, etc.) must be installed
  - Ubuntu/Debian: `apt install libavformat-dev libavcodec-dev libavutil-dev libswscale-dev`
  - Arch: `pacman -S ffmpeg`
  - Fedora: `dnf install ffmpeg-devel`
- **macOS**: `brew install ffmpeg`
- **Windows**: vcpkg or prebuilt DLLs

The crate compiles against system ffmpeg headers, which is well-established and reliable.

### Implementation

1. Check if `bevy_movie_player` 0.7 API:
   - Does it expose `MoviePlayerBundle` / `MovieSource` / `MoviePlayerState`?
   - Does it handle Ogg Theora out of the box? (ffmpeg supports Theora natively)
   - Does it work as a Bevy plugin or require manual setup?

2. Add dependency (desktop-only, behind a cfg gate if needed):
```toml
[target.'cfg(not(target_os = "android"))'.dependencies]
bevy_movie_player = { version = "0.7", default-features = false }
```

3. Implement `VideoPlugin` in `src/plugins/video.rs`:
   - Register `MoviePlayerPlugin` on desktop
   - Spawn `MoviePlayerBundle` when `PlayMovie` is encountered
   - Detect completion via `MoviePlayer` component state
   - Replace stub `PendingVideo` with real completion detection

4. Extension mapping: in the runner `PlayMovie` handler:
```rust
let actual_file = file.replace(".mpg", ".ogv");
let path = format!("movie/{}", actual_file);
// hand to bevy_movie_player
```

### Alternative: Manual ffmpeg via `ffmpeg-next`

If `bevy_movie_player` is immature, fall back to direct `ffmpeg-next` usage:
- Decode frames in a separate thread
- Send RGBA frames through a channel
- Update a Bevy `Image` each frame
- Output audio via `rodio` (already a dependency)

This is more work but gives full control.

### Deliverable

- Desktop build plays `.ogv` movies
- Opening (aiy00150) and ending (aiy50010) cutscenes render on screen
- Script resumes when video finishes
- Controls gated during video (Escape = stop + return to title)

**Verification**: Visual confirmation on desktop.

---

## Phase 4 — Android Video (5-10h)

**Goal**: Movies play on Android devices.

### Challenge

ffmpeg on Android requires NDK cross-compilation for `aarch64-linux-android`. This is doable but involves:

1. Install Android NDK (via Android Studio or standalone)
2. Install `cargo-ndk`
3. Ensure `ffmpeg-next` can find Android ffmpeg libraries
4. Either:
   a. **(Recommended)** Build ffmpeg for Android via `ffmpeg-android` or `media-ndk` and link statically
   b. Use Android's built-in `MediaPlayer` via JNI/NDK API

### Option A: Static ffmpeg via NDK

```
# Install NDK + cargo-ndk
cargo install cargo-ndk

# Build ffmpeg for Android (using ffmpeg-android-maker or similar)
./build-ffmpeg-android.sh

# Cross-compile
cargo ndk -t arm64-v8a build --features android
```

Downsides: Binary size increases by ~20-30MB (static ffmpeg). Build time increases significantly.

### Option B: Android MediaPlayer via JNI (Recommended)

Use `android-activity` crate (already a dependency!) to access Android's `NativeActivity` and call `MediaPlayer` Java API via JNI:

```
PlayMovie → JNI call → Android MediaPlayer → video on SurfaceView overlay
```

Advantages:
- No ffmpeg bundled (~20-30MB smaller APK)
- Hardware decoding (better performance, less battery)
- Supported formats: H.264, H.265, VP9 — need to convert `.ogv` → `.mp4`
- Android's MediaPlayer handles all complexity

Disadvantages:
- Need to convert `.ogv` files to `.mp4` (H.264)
- JNI bridge code required
- SurfaceView overlay means Bevy can't render UI on top of video (accept for full-screen cutscenes)

Given that cutscenes are full-screen with no interactive elements, Option B is the pragmatic choice.

### Steps for Option B

1. Add `movie-to-mp4` conversion as a build step (`build.sh` or `build.rs`)
2. Android: on `PlayMovie`, call through JNI to start `MediaPlayer` on a `SurfaceView`
3. Register a callback/JNI listener for completion
4. Desktop still uses `bevy_movie_player` (from Phase 3)

### Deliverable

- APK plays video on Android devices
- Script resumes when video finishes
- No ffmpeg in APK (Option B) or bundled ffmpeg (Option A)

**Verification**: Visual confirmation on Android device.

---

## Phase 5 — Rain System (Separate Track, 3-5h)

**Goal**: Procedural rain effects controlled by ASB tags. **Independent of Phases 1-4.**

### Rationale

Rain is NOT video. The `.ogv` rain files are pre-rendered fallbacks. Our engine should use real-time particles.

### Implementation

1. Add `ScriptCmd` variants:

```rust
SetRain {
    color: [f32; 4],      // RGBA
    direction: Vec2,       // wind (x, y)
    quantity: f32,         // 0.0–1.0 density
    enabled: bool,
}
```

2. Mapper handlers (4 tags):
- `SetColorOfRain` → set color on rain state
- `SetVectorOfSightForRainfall` → set direction
- `SetQuantityOfRainfall` → set density
- `SetValidityOfRainfall` → toggle enabled

3. `RainPlugin` with:
- `RainState` resource (color, direction, quantity, enabled)
- Particle spawning system (spawn N droplets/frame based on quantity)
- Droplet movement system (apply direction/gravity)
- Droplet render system (small lines or sprites)

4. The rain `.ogv` files in `assets/movie/` are unused by the particle system — they were pre-rendered fallbacks for the original engine.

### Deliverable

- `cargo check` clean
- Rain scenes show real-time particle rain
- Color, wind direction, density controllable
- Can be parallelized with Phases 1-4

**Verification**: Visual confirmation on rain scenes (aiy20120, aiy20200, aiy20210, aiy71410).

---

## Phase 6 — Sprite-Based Video (2-4h, Lower Priority)

**Goal**: Handle `aiy80020.asb` which uses `aiy80020mov.mpg` as a sprite texture.

This is a DIFFERENT mechanism from `PlayMovie`. The raw ASB shows:

```
DrawSpriteEx → references aiy80020mov.mpg
...
FadeSprite → manipulates the sprite
```

The `.mpg` file is loaded as a **texture for a sprite**, not played as full-screen video. This may mean:
- The original Artemis engine could play video into a sprite texture
- OR the file is actually a still image with a `.mpg` extension (unlikely)
- OR it uses hardware video decode to feed frames into a sprite

### Approach

1. Check if `aiy80020mov.ogv` is a short animation or a few frames
2. If it's a short loop: extract frames via ffmpeg offline, create a sprite sheet/animation sequence
3. If it's a real video: implement sprite-based video playback (render decoded frames to a sprite texture)

**Lower priority** — the game works without this, just the sprite area would be blank or show a placeholder.

---

## Summary Timeline

```
Week 1:  Phase 1 (Mapper)       → testable bscript output
         Phase 2 (Runner Stub)  → testable game flow
         Phase 5 (Rain)         → testable rain effects (parallel)

Week 2:  Phase 3 (Desktop Video) → testable cutscenes on desktop

Week 3:  Phase 4 (Android Video) → testable cutscenes on device

Optional: Phase 6 (Sprite Video) → after core video works
```

## Risk Register

| Risk | Impact | Mitigation |
|---|---|---|
| bevy_movie_player incompatible with Bevy 0.18.1+ | Phase 3 blocked | Check exact version; fall back to manual ffmpeg-next |
| ffmpeg Android NDK cross-compilation fails | Phase 4 blocked | Switch to Android MediaPlayer JNI approach |
| Ogg Theora unsupported on some Android devices | Phase 4 degraded | Convert to H.264 `.mp4` during build |
| APK size exceeds 4GB | Release blocked | Option B (MediaPlayer JNI) avoids ffmpeg ~30MB; already at 3.1GB |
| Rain particle system performance | Phase 5 degraded | Limit max particles; use simple line rendering; fall back to pre-rendered `.ogv` |
