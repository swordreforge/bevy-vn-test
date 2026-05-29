# AGENTS.md — bevy-vn

Bevy 0.18 visual novel engine ("Aiyoku no Eustia"). Imports Artemis engine assets.

## Build & Run

- `cargo run` — desktop build & run
- `cargo check` — fast typecheck
- `cargo build` — build only (dev profile uses opt-level 1 for crate, 3 for deps — first build is slow)
- `cargo build --release` — release with LTO thin

No test framework is configured. `cargo test` will compile but has no tests.

## Workspace

- **Root crate** (`bevy-vn`): the game engine, produces `cdylib` + `lib`
- **`tools/artemis-export`**: CLI tool for converting Artemis `.asb` files to `.bscript.ron`

## build.rs — Code generation

`build.rs` scans `assets/scripts/*.bscript.ron` and `assets/image/ev/` at compile time, generating `OUT_DIR/game_data.rs`. This provides:
- `all_scripts()` — list of script names + `include_str!` content
- `obj_index_content()` — `obj_index.ron` inline
- `all_cg_files()` / `ev_file_ext()` / `ev_file_path()` — CG image registry

**Adding/removing scripts or CG images requires a rebuild** (`cargo build`). Changing script content alone does not, since `include_str!` is embed-based, but `cargo:rerun-if-changed` references the directories.

## Bevy 0.18 API differences

This project uses Bevy 0.18 which has breaking API changes from earlier versions:

| Old (pre-0.18) | Bevy 0.18 |
|---|---|
| `Event` / `EventWriter` / `add_event` | `Message` / `MessageWriter` / `add_message` |
| `touch_input` | `Touches` (`.any_just_pressed()`) |
| `get_single_mut()` | `single_mut()` |
| `Style` | Direct `Node` fields |
| `TextBundle` | Separate `Text` + `TextFont` + `TextColor` components |

## Architecture

- **Entrypoint**: `src/main.rs` → `build_app()` in `src/lib.rs`
- **State machine**: `AppState` in `src/state.rs` — `Boot → Splash → Title → Gameplay → Menu/SaveLoad/Gallery/Settings/Backlog`
- **Scripts**: `src/script.rs` defines `ScriptCmd` enum (30+ variants). Loaded by `script_loader` plugin, executed by `script_runner` plugin.
- **Resources**: `src/resources.rs` — `AffectionMap`, `SaveData`, `SaveManager`, `Settings`, `UnlockState`, `ObjFileIndex`, `GameFont`
- **Plugins** (`src/plugins/`): audio, title, inputs, menu, script_loader, script_runner, affection, save_load, dialogue, settings, gallery, rendering, choice, screen_transition, splash, backlog, event_system
- **Message files**: `src/audio_messages.rs`, `src/rendering_messages.rs`, `src/choice_messages.rs` — Bevy 0.18 `Message` types
- **Android**: `build_app()` is shared; `android_main()` in `lib.rs` under `#[cfg(feature = "android")]`

## Conventions

- Game window: 1280×720, scale factor override 1.0, camera `ScalingMode::AutoMin` (minimum 1280×720)
- Script files are `.bscript.ron` (RON format), named like `aiyXXXXX.bscript.ron`
- `obj_index.ron` maps asset names to paths for `image/obj/` character sprites
- Saves go to `saves/` directory (JSON serialization)
- Gallery has a debug unlock key (press U to unlock all CGs)
- Asset directories: `assets/{audio,fonts,image,movie,scripts,shaders}`

## .gitignore quirk

`assets/scripts/*.bscript.ron` is gitignored (they are conversion output from `tools/artemis-export`), but are required at build time. A local checkout needs the conversion step or pre-generated files.