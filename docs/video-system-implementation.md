# Video System Implementation

## Current State

`ScriptCmd::PlayMovie { file: String }` exists at `src/script.rs:159` but is **never emitted** by the mapper or IET parser. The runtime has no video playback capability.

### Unmapped Assets

| Asset | Scripts | Current Status |
|---|---|---|
| 12 `.ogv` files in `assets/movie/` | Referenced in 8 ASB files | Sitting unused |
| Rain system tags (5 tags) | `aiy20120.asb` | Not mapped |

### ASB Tags to Map

| Tag | Occurrences | Scripts |
|---|---|---|
| `PlayMovie` | 2 | `aiy00150` (opening), `aiy50010` (ending) |
| `WaitToFinishMoviePlayingOnSprite` | 6 | `aiy20230`, `aiy30150`, `aiy30180`, `aiy40170`, `aiy50050`, `aiy81010` |
| `SetColorOfRain` | 1+ | `aiy20120` |
| `SetVectorOfSightForRainfall` | 1+ | `aiy20120` |
| `SetQuantityOfRainfall` | 1+ | `aiy20120` |
| `SetValidityOfRainfall` | 1+ | `aiy20120` |

### Movie File Inventory (assets/movie/)

**Cutscene movies** (full-screen, likely intended for `PlayMovie`):
- `aiy20230mov.ogv` → from `aiy20230.asb`
- `aiy30150mov.ogv` → from `aiy30150.asb` (also reused in `aiy40170.asb`)
- `aiy30180mov.ogv` → from `aiy30180.asb`
- `aiy50050mov.ogv` → from `aiy50050.asb`
- `aiy80020mov.ogv` → from `aiy80020.asb` (used as sprite texture, not full-screen)
- `aiy81010mov.ogv` → from `aiy81010.asb`

**Opening/Ending**:
- `aiy50010mov.ogv` → from `aiy50010.asb` (ending)
- `movie.mpg` → from `aiy00150.asb` (opening — generic name)

**Rain overlays**:
- `aiy20120_rain.ogv` → rainfall in `aiy20120`
- `aiy20200_rain.ogv` / `aiy20200_rain_2.ogv` → rainfall in `aiy20200`
- `aiy20210_rain.ogv` / `aiy20210_rain_2.ogv` → rainfall in `aiy20210`
- `aiy71410_rain.ogv` → rainfall in `aiy71410`

Note: ASB references `.mpg` extension, but actual files are `.ogv` (Ogg Theora). Extension must be remapped at load time.

---

## Phase 1: Mapper + ScriptCmd

### 1a. Mapper — `PlayMovie` tag

In `tools/artemis-export/src/mapper.rs`, add:

```rust
"PlayMovie" => {
    let file = cmd.attrs.get("0")?;
    Some(vec![ScriptCmd::PlayMovie { file: file.to_string() }])
}
```

The `file` attr value is e.g. `"movie.mpg"` or `"aiy50010mov.mpg"`. Store as-is; the `.mpg` → `.ogv` mapping happens at load time.

### 1b. Mapper — `WaitToFinishMoviePlayingOnSprite` tag

Add a new `ScriptCmd::WaitForMovie` variant (or reuse via blocking flag — see Phase 1c):

```rust
"WaitToFinishMoviePlayingOnSprite" => {
    Some(vec![ScriptCmd::Wait { duration: 0 }])
}
```

Simplest approach: emit `Wait { duration: 0 }` (immediate advance). This works because `PlayMovie` is the blocking step — when `PlayMovie` finishes, the video is done. The `WaitToFinishMoviePlayingOnSprite` is a no-op guard.

**Alternative**: Add `ScriptCmd::WaitForMovie` and make the runner synchronize with the video player entity's lifecycle.

### 1c. ScriptCmd changes

`PlayMovie { file: String }` already exists at `src/script.rs:159`.

If choosing the non-trivial `WaitToFinishMoviePlayingOnSprite` mapping, add:

```rust
/// Block script execution until the current video finishes playing.
/// Used after PlayMovie when the original engine needed an explicit
/// synchronization point.
WaitForMovie,
```

---

## Phase 2: Video Backend

### 2a. Dependency: `bevy_movie_player`

Add to `Cargo.toml`:

```toml
bevy_movie_player = { version = "0.7", default-features = false }
```

Note: v0.7.1 supports Bevy 0.18. Uses ffmpeg under the hood, supporting Ogg Theora (`.ogv`) and MPEG-1 (`.mpg`).

### 2b. Plugin Registration

In `src/lib.rs`, add the plugin:

```rust
use bevy_movie_player::MoviePlayerPlugin;

build_app()
    .add_plugins(MoviePlayerPlugin)
    // ...
```

### 2c. Android Cross-Compilation

`bevy_movie_player` depends on ffmpeg, which requires NDK cross-compilation for Android:

1. Install `cargo-ndk` and Android NDK
2. Add ffmpeg bindings for Android target (see `ffmpeg-next` crate docs)
3. Build with `cargo ndk -t arm64-v8a -o ../android/app/src/main/jniLibs build --features android`

The build script (`android/build.sh`) needs updating to include ffmpeg libraries.

If Android ffmpeg cross-compilation is too complex, an alternative OS-level approach: use `MediaPlayer` via `AndroidApp::native_activity()` JNI calls to delegate video playback to Android's native `MediaPlayer`. This avoids bundling ffmpeg but requires JNI bridge code.

---

## Phase 3: Script Runner

### 3a. `PlayMovie` Handler

In `src/plugins/script_runner.rs`, add a handler that:

```rust
ScriptCmd::PlayMovie { file } => {
    // 1. Map extension: .mpg → .ogv
    let actual_file = file.replace(".mpg", ".ogv");

    // 2. Hide dialogue (optional — during video we show no UI)
    //    Already handled if PlayMovie is preceded by Window{off}

    // 3. Spawn video player entity
    //    The bevy_movie_player crate provides a component API:
    commands.spawn(MoviePlayerBundle {
        source: MovieSource::File(format!("movie/{}", actual_file)),
        ..default()
    });

    // 4. Block script execution via a resource
    video_state.playing = true;
}
```

The `MovieSource::File` path must match how `bevy_movie_player` loads assets — relative to the `assets/` directory. Since our files are in `assets/movie/`, the path would be `movie/aiy50010mov.ogv`.

### 3b. Blocking/Pending Video Resource

In `src/resources.rs`:

```rust
/// Tracks whether a video is currently playing.
/// Script execution is paused while this is true.
#[derive(Resource, Default)]
pub struct PendingVideo {
    pub playing: bool,
    pub entity: Option<Entity>,
}
```

### 3c. State Machine Integration

The `pending_commands` machinery (used for `Wait`, fade overlays, etc.) already handles blocking commands. Extend it to also check `PendingVideo`:

In the main update loop (around line 210), after checking `pending_commands`:

```rust
if video_state.playing {
    // Check if video entity still exists or if MoviePlayer reports done
    continue; // skip advance
}
```

`bevy_movie_player` likely despawns the video entity or signals completion through a component/event. Register a system that detects video end:

```rust
fn check_video_ended(
    video_state: ResMut<PendingVideo>,
    player_query: Query<&MoviePlayer>,
) {
    if let Some(entity) = video_state.entity {
        if let Ok(player) = player_query.get(entity) {
            if player.state == MoviePlayerState::Finished {
                video_state.playing = false;
                // Optionally despawn entity
            }
        }
    }
}
```

### 3d. Extension Mapping

The `.mpg` → `.ogv` mapping can be handled centrally, similar to CG extension mapping in `ev_file_ext()`:

In `src/resources.rs`:

```rust
/// Map the video filename from ASB reference to actual file.
/// ASB scripts reference .mpg, but actual files are .ogv (Ogg Theora).
pub fn map_video_file(asb_path: &str) -> String {
    if asb_path.ends_with(".mpg") {
        asb_path.replacen(".mpg", ".ogv", 1)
    } else {
        format!("{}.ogv", asb_path)
    }
}
```

---

## Phase 4: UI Integration

### 4a. During Video Playback

During video:
- Dialogue window should be hidden (`Window { show: false }` is typically issued before `PlayMovie`)
- No advance/vote/skip interaction — video plays to completion
- **Skip mode** should skip the entire video (just advance script without spawning player)
- **Auto mode** should not auto-advance during video

### 4b. Controls During Video

- **Escape** → Stop video, return to title (same as current Escape behavior)
- **Left click / tap** → [Optional] Skip current video (depends on design preference)
- **Advance event** should be gated on `!video_state.playing`

In `inputs.rs`:

```rust
fn handle_advance(
    // ...
    video_state: Res<PendingVideo>,
    mut advance_ev: EventWriter<AdvanceEvent>,
) {
    if video_state.playing { return; }
    // ...
}
```

---

## Phase 5: Build System

### 5a. Asset Embedding (build.rs)

The `.ogv` files in `assets/movie/` need to be registered so they're embedded in the APK for Android:

Update `build.rs` to scan `assets/movie/` and generate a `movie_files()` function (similar to `all_cg_files()`).

```rust
fn scan_movie_files() -> Vec<String> {
    let movie_dir = project_root.join("assets").join("movie");
    let mut files = Vec::new();
    if movie_dir.exists() {
        for entry in fs::read_dir(movie_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |e| e == "ogv") {
                files.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    files
}
```

### 5b. Asset Loading

`bevy_movie_player` uses Bevy's `AssetServer` internally. For Android, ensure the `.ogv` files are loadable via `AndroidAssetReader`. This may require:

- Adding the files to the APK's assets directory (handled by `build.rs` embedding or `assets/` directory inclusion)
- If using embedded `include_bytes!` + custom asset loader, match the pattern used by scripts/CGs

---

## Implementation Order

1. **Mapper**: `PlayMovie` and `WaitToFinishMoviePlayingOnSprite` tag handlers → immediately testable (bscript output includes `PlayMovie` commands)
2. **ScriptCmd**: `WaitForMovie` variant (if chosen)
3. **Runner**: `PlayMovie` handler with stub (just advance, no video) — verify bscript flow works end-to-end
4. **Dependency**: Add `bevy_movie_player`, verify desktop build
5. **Plugin**: `PendingVideo` resource, state machine gating, video start/end detection
6. **Rain system** (separate): `SetColorOfRain` etc. → particle system, independent of video

---

## Rain System (Separate Track)

The rain tags are **not video** — they control a procedural particle system:

| Tag | Effect |
|---|---|
| `SetColorOfRain` | Rain droplet color |
| `SetVectorOfSightForRainfall` | Rain wind direction/angle |
| `SetQuantityOfRainfall` | Rain density/intensity |
| `SetValidityOfRainfall` | Rain on/off toggle |

The `.ogv` rain files (`*_rain.ogv`) are pre-rendered fallbacks for platforms without real-time particle capabilities. In our engine, implement as a Bevy particle system (or simple sprite spawner) controlled by these parameters.

Implementation path:
1. Add `ScriptCmd::SetRain { color, direction, quantity, enabled }` variant
2. Map the 4 rain tags in mapper.rs
3. Create a `RainPlugin` with a particle system
4. The `ValidityOfRainfall` tag toggles the system on/off
5. No dependency on video playback libraries

---

## References

- `bevy_movie_player` v0.7.1: https://crates.io/crates/bevy_movie_player
- ASB files with PlayMovie: `aiy00150`, `aiy50010`
- ASB files with WaitToFinishMoviePlayingOnSprite: `aiy20230`, `aiy30150`, `aiy30180`, `aiy40170`, `aiy50050`, `aiy81010`
- ASB file with rain tags: `aiy20120`
- Existing CG extension mapping pattern: `ev_file_ext()` / `ev_file_path()` in `build.rs`
- Existing blocking command pattern: `pending_commands` in `script_runner.rs`
