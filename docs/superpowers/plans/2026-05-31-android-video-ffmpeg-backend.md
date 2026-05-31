# Android Video Backend (ffmpeg-the-third) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace GStreamer stubs (`info!("Video not supported on Android")`) with real video playback using `ffmpeg-the-third` on Android, while keeping GStreamer on Desktop.

**Architecture:** Each public function/sytem in `video/mod.rs` gets a `#[cfg(target_os = "android")]` arm alongside the existing `#[cfg(not(target_os = "android"))]` GStreamer arm. Resource structs in `resources.rs` gain cfg-gated FFmpeg state fields. FFmpeg decodes packets from a pre-loaded in-memory buffer one-at-a-time in the Bevy system, scales to RGBA, and writes into the Bevy `Image` handle — same contract as the GStreamer appsink path.

**Key design decision for ffmpeg streaming:** Pre-read all compressed packets into a `Vec<Packet>` upfront (memory-cheap, ~few MB for a 30s video). The Bevy system then pulls packets one-at-a-time through `send_packet → receive_frame → scaler.run` to get RGBA data. This avoids lifetime issues with `ictx.packets()` while keeping a pull-based model.

**Tech Stack:** `ffmpeg-the-third = "5"` + features `build, build-mediacodec` (Android only). Desktop retains `gstreamer = "0.25"`. Cross-compilation via `cargo-ndk`.

**Prerequisites:** `cargo-ndk` installed. Android NDK (r26 or newer) configured. Test via `cargo ndk -t arm64-v8a build`.

---

## File Inventory

| File | Responsibility | Changes |
|---|---|---|
| `Cargo.toml` | Dependencies | Add `ffmpeg-the-third` behind `[target.'cfg(target_os = "android")']` |
| `src/resources.rs` | State structs | Add `FFmpegPipeline` (opaque decoder handle), `PendingVideo.ffmpeg`, `SpriteVideoFFmpegState`, `RainFFmpegState` |
| `src/plugins/video/mod.rs` | Video playback logic | Add Android FFmpeg arms in all 6 public fns + 4 systems + `create_ffmpeg_pipeline` helper |
| `src/lib.rs` | App wiring | No changes needed (already registers `VideoPlugin`) |

**No changes in:** `src/plugins/script_runner.rs` (calls generic fns), `tools/artemis-export/` (platform-independent).

---

### Task 1: Add ffmpeg-the-third dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Edit `Cargo.toml` — add the android-gated ffmpeg dep**

```toml
[target.'cfg(target_os = "android")'.dependencies]
ffmpeg-the-third = { version = "5", features = ["build", "build-mediacodec"] }
```

Leave the existing `[target.'cfg(not(target_os = "android"))'.dependencies]` GStreamer block unchanged.

- [ ] **Step 2: Verify desktop build is unaffected**

```bash
cargo check
```
Expected: success, zero ffmpeg crates compiled.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add ffmpeg-the-third dep for Android video backend"
```

---

### Task 2: Add FFmpeg state types to resources.rs

**Files:**
- Modify: `src/resources.rs` (after line 601 for sprite types, after line 616 for rain types)

**What:** Add three Android-only state structs, then add cfg-gated `ffmpeg` fields to `PendingVideo`, `SpriteVideoManager`, and `RainOverlayState`.

- [ ] **Step 1: Add `FFmpegPipeline` type** (after `#[cfg(not(target_os = "android"))] fn map_video_file`, around line 585)

This struct wraps all ffmpeg-the-third state needed to decode one video. It is stored opaquely so that the rest of resources.rs does not need to import ffmpeg types.

```rust
// ── Android FFmpeg decoder state ──

/// Wraps all ffmpeg-the-third state for single-video decoding.
/// Pre-loaded packets are decoded one-at-a-time in the Bevy system.
#[cfg(target_os = "android")]
pub struct FFmpegPipeline {
    /// Pre-read compressed packets from the video stream
    pub packets: Vec<ffmpeg_the_third::Packet>,
    pub packet_cursor: usize,
    pub stream_index: usize,
    pub decoder: ffmpeg_the_third::codec::decoder::video::Video,
    pub scaler: ffmpeg_the_third::software::scaling::Context,
    pub flushed: bool,
    pub eos: bool,
}
```

- [ ] **Step 2: Define `FFmpegVideoState` for full-screen movie** (analogous to `GstVideoState`)

```rust
#[cfg(target_os = "android")]
pub struct FFmpegVideoState {
    pub pipeline: FFmpegPipeline,
    pub image_handle: Handle<Image>,
    pub width: u32,
    pub height: u32,
}
```

- [ ] **Step 3: Define `SpriteVideoFFmpegState`** (analogous to `SpriteVideoGstState`)

```rust
#[cfg(target_os = "android")]
pub struct SpriteVideoFFmpegState {
    pub pipeline: FFmpegPipeline,
    pub image_handle: Handle<Image>,
    pub entity: Entity,
    pub eos: bool,
    pub width: u32,
    pub height: u32,
}
```

- [ ] **Step 4: Define `RainFFmpegState`** (analogous to `RainGstState`)

```rust
#[cfg(target_os = "android")]
pub struct RainFFmpegState {
    pub pipeline: FFmpegPipeline,
    pub image_handle: Handle<Image>,
    pub entity: Entity,
    pub width: u32,
    pub height: u32,
}
```

- [ ] **Step 5: Add `ffmpeg` field to `PendingVideo`**

Change from:
```rust
    #[cfg(not(target_os = "android"))]
    pub gst: Option<GstVideoState>,
```
To:
```rust
    #[cfg(not(target_os = "android"))]
    pub gst: Option<GstVideoState>,
    #[cfg(target_os = "android")]
    pub ffmpeg: Option<FFmpegVideoState>,
```

- [ ] **Step 6: Update `SpriteVideoManager`** — the `videos` field needs an Android variant:

```rust
#[derive(Resource, Default)]
pub struct SpriteVideoManager {
    #[cfg(not(target_os = "android"))]
    pub videos: HashMap<String, SpriteVideoGstState>,
    #[cfg(target_os = "android")]
    pub videos: HashMap<String, SpriteVideoFFmpegState>,
}
```

- [ ] **Step 7: Add `ffmpeg` field to `RainOverlayState`**

After `#[cfg(not(target_os = "android"))] pub gst: Option<RainGstState>,`:
```rust
    #[cfg(target_os = "android")]
    pub ffmpeg: Option<RainFFmpegState>,
```

And in the `Default` impl body add:
```rust
            #[cfg(target_os = "android")]
            ffmpeg: None,
```

- [ ] **Step 8: Verify desktop build**

```bash
cargo check
```
Expected: success.

- [ ] **Step 9: Commit**

```bash
git add src/resources.rs
git commit -m "feat: add FFmpegPipeline + per-type FFmpeg state for Android"
```

---

### Task 3: Implement `create_ffmpeg_pipeline` helper

**Files:**
- Modify: `src/plugins/video/mod.rs` (add after `create_gst_pipeline` at line 541)

- [ ] **Step 1: Add the Android-only `use` import at top of file**

```rust
#[cfg(target_os = "android")]
use ffmpeg_the_third as ffmpeg;
```

- [ ] **Step 2: Write `create_ffmpeg_pipeline`**

```rust
/// Build an FFmpegPipeline from a video file.
/// Pre-reads all compressed packets into memory.
#[cfg(target_os = "android")]
fn create_ffmpeg_pipeline(
    abs_path: &std::path::Path,
) -> (Option<crate::resources::FFmpegPipeline>, u32, u32) {
    use ffmpeg::*;

    let path_str = match abs_path.to_str() {
        Some(s) => s.to_string(),
        None => { return (None, 0, 0); }
    };

    // -- 1. Initialize ffmpeg
    ffmpeg::init().unwrap();

    // -- 2. Open input
    let mut ictx = match format::input(&path_str) {
        Ok(ctx) => ctx,
        Err(e) => { warn!("ffmpeg: open failed: {}", e); return (None, 0, 0); }
    };

    // -- 3. Find best video stream
    let video_stream = match ictx.streams().best(media::Type::Video) {
        Some(s) => s,
        None => { warn!("ffmpeg: no video stream"); return (None, 0, 0); }
    };
    let stream_index = video_stream.index();
    let params = video_stream.parameters();

    // -- 4. Create decoder
    let codec_ctx = match codec::context::Context::from_parameters(params.clone()) {
        Ok(c) => c,
        Err(e) => { warn!("ffmpeg: codec ctx: {}", e); return (None, 0, 0); }
    };
    let mut decoder = match codec_ctx.decoder().video() {
        Ok(d) => d,
        Err(e) => { warn!("ffmpeg: decoder: {}", e); return (None, 0, 0); }
    };
    let width = decoder.width();
    let height = decoder.height();

    // -- 5. Create scaler for RGBA output
    let scaler = match software::scaling::Context::get(
        decoder.format(),
        width,
        height,
        format::Pixel::RGBA,
        width,
        height,
        software::scaling::Flags::BILINEAR,
    ) {
        Ok(s) => s,
        Err(e) => { warn!("ffmpeg: scaler: {}", e); return (None, 0, 0); }
    };

    // -- 6. Read all compressed packets into memory
    let mut packets: Vec<ffmpeg::Packet> = Vec::new();
    for (_stream, packet) in ictx.packets() {
        packets.push(packet);
    }

    info!("ffmpeg: loaded {} packets from {:?} ({}x{})",
        packets.len(), abs_path, width, height);

    (
        Some(crate::resources::FFmpegPipeline {
            packets,
            packet_cursor: 0,
            stream_index,
            decoder,
            scaler,
            flushed: false,
            eos: false,
        }),
        width,
        height,
    )
}
```

- [ ] **Step 3: Write `ffmpeg_decode_next_frame` helper**

`poll_frame` for the pull-based model: send the next packet, try to receive a frame, scale to RGBA.

```rust
/// Decode the next available frame from the pipeline.
/// Returns `Some(RGBA bytes)` when a new frame is ready, `None` otherwise.
/// Returns `None` but sets `eos = true` when all frames are exhausted.
#[cfg(target_os = "android")]
fn ffmpeg_decode_next_frame(
    pipeline: &mut crate::resources::FFmpegPipeline,
) -> Option<Vec<u8>> {
    use ffmpeg::*;
    use ffmpeg::codec::decoder::Video as _;

    // Flush path: after all packets are sent, call send_eof and drain
    if pipeline.flushed {
        let mut frame = util::frame::video::Video::empty();
        let mut rgba = util::frame::video::Video::empty();
        match pipeline.decoder.receive_frame(&mut frame) {
            Ok(()) => {
                if pipeline.scaler.run(&frame, &mut rgba).is_ok() {
                    let data = rgba.data(0).to_vec();
                    return Some(data);
                }
                return None;
            }
            Err(ffmpeg::error::Error::Eof) => {
                pipeline.eos = true;
                return None;
            }
            Err(_) => return None,
        }
    }

    // Normal path: send packets until we get a video frame or run out
    let mut frame = util::frame::video::Video::empty();
    let mut rgba = util::frame::video::Video::empty();

    while pipeline.packet_cursor < pipeline.packets.len() {
        let packet = &pipeline.packets[pipeline.packet_cursor];
        pipeline.packet_cursor += 1;

        // Skip non-video packets (demuxed per-stream — but ffmpeg-the-third's
        // iteration yields all packets; we check stream_index in the iterator)
        if pipeline.decoder.send_packet(packet).is_err() {
            continue;
        }

        // Try to receive any pending frames from this packet
        if pipeline.decoder.receive_frame(&mut frame).is_ok() {
            if pipeline.scaler.run(&frame, &mut rgba).is_ok() {
                return Some(rgba.data(0).to_vec());
            }
        }
        // Continue sending packets — some frames span multiple packets
    }

    // All packets sent, flush decoder
    let _ = pipeline.decoder.send_eof();
    pipeline.flushed = true;

    // Try one more receive (avoids recursion)
    if pipeline.decoder.receive_frame(&mut frame).is_ok() {
        if pipeline.scaler.run(&frame, &mut rgba).is_ok() {
            return Some(rgba.data(0).to_vec());
        }
    }

    pipeline.eos = true;
    None
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```
Expected: success (the Android code is cfg-gated so desktop check skips it).

- [ ] **Step 5: Commit**

```bash
git add src/plugins/video/mod.rs
git commit -m "feat: add create_ffmpeg_pipeline + ffmpeg_decode_next_frame helpers"
```

---

### Task 4: Implement full-screen movie (PlayMovie) for Android

**Files:**
- Modify: `src/plugins/video/mod.rs`

**What:** Add `#[cfg(target_os = "android")]` arms inside `spawn_video()`, `check_video_completion()`, and `cleanup_video()`.

- [ ] **Step 1: Android arm of `spawn_video`** (lines 34-45)

Replace the existing stub:
```rust
pub fn spawn_video(commands: &mut Commands, asset_path: String) -> Entity {
    info!("Preparing video: {}", asset_path);
    #[cfg(not(target_os = "android"))]
    {
        commands.insert_resource(PendingVideoInit { asset_path });
    }
    #[cfg(target_os = "android")]
    {
        info!("Video not supported on Android: {}", asset_path);
    }
    Entity::PLACEHOLDER
}
```

Change the Android arm to:
```rust
    #[cfg(target_os = "android")]
    {
        commands.insert_resource(PendingVideoInit { asset_path });
    }
```

(Full-screen movie uses the same lazy-init pattern as GStreamer: `PendingVideoInit` is consumed by `check_video_completion`.)

- [ ] **Step 2: Android arm in `check_video_completion`** (inside the `VideoPlugin` system)

After line 168 (after the `if let Some(ref mut timer)` block), add an Android arm. The main logic (after `pending_video.playing && ...`) needs to init the ffmpeg pipeline:

Inside `check_video_completion`, after the existing `#[cfg(not(target_os = "android"))]` block (line 55-156), add:

```rust
    #[cfg(target_os = "android")]
    {
        if pending_video.playing && pending_video.ffmpeg.is_none() {
            if let Some(init) = video_init.as_mut() {
                let abs_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("assets")
                    .join(&init.asset_path);
                commands.remove_resource::<PendingVideoInit>();

                let (pipeline, width, height) = if abs_path.exists() {
                    create_ffmpeg_pipeline(&abs_path)
                } else {
                    warn!("Video file not found: {:?}", abs_path);
                    (None, 0, 0)
                };

                if let Some(pipeline) = pipeline {
                    let blank = Image::new(
                        wgpu_types::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
                        wgpu_types::TextureDimension::D2,
                        // One RGBA black frame
                        vec![0u8; (width.max(1) * height.max(1) * 4) as usize],
                        wgpu_types::TextureFormat::Rgba8UnormSrgb,
                        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
                    );
                    let image_handle = images.add(blank);
                    let entity = commands.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        ImageNode::new(image_handle.clone()),
                        ZIndex(10),
                    )).id();

                    pending_video.ffmpeg = Some(crate::resources::FFmpegVideoState {
                        pipeline,
                        image_handle,
                        width,
                        height,
                    });
                    pending_video.entity = Some(entity);
                }
            }
        }

        // Poll frames
        let eos = pending_video.ffmpeg.as_mut().map(|ffstate| {
            if let Some(rgba_data) = ffmpeg_decode_next_frame(&mut ffstate.pipeline) {
                if let Some(img) = images.get_mut(&ffstate.image_handle) {
                    img.data = Some(rgba_data);
                }
            }
            ffstate.pipeline.eos
        }).unwrap_or(false);

        if eos {
            info!("FFmpeg video EOS");
            if let Some(entity) = pending_video.entity.take() {
                commands.entity(entity).despawn();
            }
            pending_video.playing = false;
            pending_video.ffmpeg = None;
            pending_video.timer = None;
            advance_ev.write(AdvanceEvent {
                source: AdvanceSource::Auto,
            });
            return;
        }
    }
```

- [ ] **Step 3: Android arm of `cleanup_video`** (lines 171-187)

After the existing `#[cfg(not(target_os = "android"))]` block:
```rust
    #[cfg(target_os = "android")]
    {
        pending_video.ffmpeg = None;
    }
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```
Expected: success (Android paths are cfg-gated).

- [ ] **Step 5: Commit**

```bash
git add src/plugins/video/mod.rs
git commit -m "feat: full-screen movie PlayMovie via ffmpeg-the-third on Android"
```

---

### Task 5: Implement sprite video (DrawSpriteEx) for Android

**Files:**
- Modify: `src/plugins/video/mod.rs`

**What:** Add `#[cfg(target_os = "android")]` arms inside `spawn_sprite_video()`, `check_sprite_videos()`, `cleanup_sprite_videos()`, `is_sprite_video_playing()`, and `stop_sprite_video()`.

- [ ] **Step 1: Android arm of `spawn_sprite_video`** (replace the stub at lines 262-266)

```rust
    #[cfg(target_os = "android")]
    {
        if !abs_path.exists() {
            warn!("Sprite video file not found: {:?}", abs_path);
            return Entity::PLACEHOLDER;
        }

        let (pipeline, vid_width, vid_height) = create_ffmpeg_pipeline(abs_path);
        let pipeline = match pipeline {
            Some(p) => p,
            None => return Entity::PLACEHOLDER,
        };

        let blank = Image::new(
            wgpu_types::Extent3d { width: vid_width.max(1), height: vid_height.max(1), depth_or_array_layers: 1 },
            wgpu_types::TextureDimension::D2,
            vec![0u8; (vid_width.max(1) * vid_height.max(1) * 4) as usize],
            wgpu_types::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        let image_handle = images.add(blank);

        let entity = commands.spawn((
            Node {
                width: Val::Px(w),
                height: Val::Px(h),
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                ..default()
            },
            ImageNode::new(image_handle.clone()),
            Visibility::Visible,
            ZIndex(priority.max(0) as i32 + 1),
        )).id();

        sprite_mgr.videos.insert(
            sprite_id.clone(),
            crate::resources::SpriteVideoFFmpegState {
                pipeline,
                image_handle,
                entity,
                eos: false,
                width: vid_width,
                height: vid_height,
            },
        );

        info!("Sprite video entity {} created for sprite {}", entity, sprite_id);
        entity
    }
```

- [ ] **Step 2: Android arm of `check_sprite_videos`** (after the `#[cfg(not(target_os = "android"))]` block at lines 276-311)

```rust
    #[cfg(target_os = "android")]
    {
        let mut finished: Vec<String> = Vec::new();
        for (sprite_id, state) in sprite_mgr.videos.iter_mut() {
            if let Some(rgba_data) = ffmpeg_decode_next_frame(&mut state.pipeline) {
                if let Some(img) = images.get_mut(&state.image_handle) {
                    img.data = Some(rgba_data);
                }
            }
            if state.pipeline.eos {
                state.eos = true;
                finished.push(sprite_id.clone());
            }
        }

        for id in finished {
            info!("Sprite video EOS: {}", id);
            if let Some(state) = sprite_mgr.videos.remove(&id) {
                commands.entity(state.entity).despawn();
            }
            if blocked.0.as_deref() == Some(&id) {
                blocked.0 = None;
                advance_ev.write(AdvanceEvent {
                    source: AdvanceSource::Auto,
                });
            }
        }
    }
```

- [ ] **Step 3: Android arm of `cleanup_sprite_videos`** (after line 322)

```rust
    #[cfg(target_os = "android")]
    {
        for (_id, state) in sprite_mgr.videos.drain() {
            commands.entity(state.entity).despawn();
        }
    }
```

- [ ] **Step 4: Android arm of `is_sprite_video_playing`** (replace the stub at lines 331-335)

```rust
    #[cfg(target_os = "android")]
    {
        sprite_mgr.videos.contains_key(sprite_id)
    }
```

- [ ] **Step 5: Android arm of `stop_sprite_video`** (after line 350)

```rust
    #[cfg(target_os = "android")]
    {
        if let Some(state) = sprite_mgr.videos.remove(sprite_id) {
            commands.entity(state.entity).despawn();
            info!("Stopped sprite video: {}", sprite_id);
        }
    }
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check
```
Expected: success.

- [ ] **Step 7: Commit**

```bash
git add src/plugins/video/mod.rs
git commit -m "feat: DrawSpriteEx sprite video via ffmpeg-the-third on Android"
```

---

### Task 6: Implement rain overlay for Android

**Files:**
- Modify: `src/plugins/video/mod.rs`

**What:** Add `#[cfg(target_os = "android")]` arms in `start_rain_video()`, `update_rain_video()`, and `stop_rain_video()`.

- [ ] **Step 1: Android arm in `start_rain_video`** (after line 424)

```rust
    #[cfg(target_os = "android")]
    {
        if !abs_path.exists() {
            warn!("Rain video file not found: {:?}", abs_path);
            return;
        }

        let (pipeline, vid_width, vid_height) = create_ffmpeg_pipeline(abs_path);
        let pipeline = match pipeline {
            Some(p) => p,
            None => return,
        };

        let blank = Image::new(
            wgpu_types::Extent3d { width: vid_width.max(1), height: vid_height.max(1), depth_or_array_layers: 1 },
            wgpu_types::TextureDimension::D2,
            vec![0u8; (vid_width.max(1) * vid_height.max(1) * 4) as usize],
            wgpu_types::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        let image_handle = images.add(blank);

        let entity = commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            ImageNode {
                image: image_handle.clone(),
                color: rain.color,
                ..default()
            },
            Visibility::Visible,
            ZIndex(priority.max(0) as i32 + 1),
        )).id();

        rain.entity = Some(entity);
        rain.ffmpeg = Some(crate::resources::RainFFmpegState {
            pipeline,
            image_handle,
            entity,
            width: vid_width,
            height: vid_height,
        });
        rain.enabled = true;
        info!("Rain overlay started (ffmpeg)");
    }
```

- [ ] **Step 2: Android arm in `update_rain_video`** (after line 459)

```rust
    #[cfg(target_os = "android")]
    {
        let Some(ref mut ffstate) = rain.ffmpeg else { return };

        if let Some(rgba_data) = ffmpeg_decode_next_frame(&mut ffstate.pipeline) {
            if let Some(img) = images.get_mut(&ffstate.image_handle) {
                img.data = Some(rgba_data);
            }
        }

        if ffstate.pipeline.eos {
            if let Some(entity) = rain.entity.take() {
                commands.entity(entity).despawn();
            }
            rain.ffmpeg = None;
            rain.enabled = false;
            info!("Rain video EOS — restart via rain_mja");
        }
    }
```

- [ ] **Step 3: Android arm in `stop_rain_video`** (after line 478)

```rust
    #[cfg(target_os = "android")]
    {
        rain.ffmpeg = None;
    }
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add src/plugins/video/mod.rs
git commit -m "feat: rain overlay via ffmpeg-the-third on Android"
```

---

### Task 7: Build and verify with Android NDK

- [ ] **Step 1: Check that cargo-ndk is available**

```bash
which cargo-ndk
```
If missing, install: `cargo install cargo-ndk`

- [ ] **Step 2: Build for arm64-v8a**

```bash
cargo ndk -t arm64-v8a build 2>&1
```
Expected: build succeeds (first build will compile ffmpeg from source via NDK — allow 10-30 min).

- [ ] **Step 3: Check for warnings**

```bash
cargo ndk -t arm64-v8a build 2>&1 | grep -i warning
```
Expected: no ffmpeg-related warnings.

- [ ] **Step 4: Verify binary size**

```bash
ls -lh target/aarch64-linux-android/debug/libbevy_vn.so
```
Expected: reasonable file size (the .so will include ffmpeg statically linked).

- [ ] **Step 5: Commit final build tweaks (if any)**

```bash
git add -A && git commit -m "fix: android build after ffmpeg backend integration"
```

---

## Post-Merge Verification Checklist

- [ ] Desktop `cargo run` — videos still play via GStreamer
- [ ] Android `cargo ndk -t arm64-v8a run` — opening video plays via ffmpeg-the-third
- [ ] Android sprite video (DrawSpriteEx) renders and EOS unblocks script
- [ ] Android rain overlay renders, SetRainColor updates entity tint
- [ ] Memory: no leak on video stop/cleanup/EOS
- [ ] Gallery unlock: no regression (movie playback is separate from CG gallery)

## Edge Cases

- **File not found**: GStreamer path handles this; ffmpeg path must also log a warning and return a blank frame (not crash)
- **Zero-frame video**: `ffmpeg_decode_next_frame` returns `None` immediately, `eos` becomes true, pipeline cleans up — same as GStreamer path
- **Rapid rain_mja / rain_mja cycling**: `start_rain_video` calls `stop_rain_video` first, so old ffmpeg state is dropped before new one starts
- **Frame size mismatch**: ffmpeg decodes at native resolution. The Bevy `Node` size is set from ASB attributes (`w`, `h`); the image may be up/down-sampled by Bevy's UI renderer
