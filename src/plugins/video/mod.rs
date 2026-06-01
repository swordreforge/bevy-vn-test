// This module is heavily platform-gated. On Android, many params are intentionally unused.
#![cfg_attr(target_os = "android", allow(unused, unused_mut))]

use crate::plugins::inputs::{AdvanceEvent, AdvanceSource};
use crate::resources::{
    PendingSpriteVideoBlock, PendingVideo, PendingVideoInit,
    RainOverlayState, SpriteVideoManager,
};
#[cfg(not(target_os = "android"))]
use crate::resources::{GstVideoState, RainGstState};
#[cfg(target_os = "android")]
use ffmpeg_the_third as ffmpeg;
use bevy::prelude::*;

pub struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_os = "android"))]
        {
            let _ = gstreamer::init();
        }
        app.add_systems(
            Update,
            (
                check_video_completion,
                check_sprite_videos,
                update_rain_video,
            )
                .run_if(in_state(crate::state::AppState::Gameplay)),
        )
        .add_systems(
            OnExit(crate::state::AppState::Gameplay),
            (cleanup_video, cleanup_sprite_videos, stop_rain_video_system),
        );
    }
}

// ── Full-screen movie (PlayMovie) ──

pub fn spawn_video(commands: &mut Commands, asset_path: String) -> Entity {
    info!("Preparing video: {}", asset_path);
    #[cfg(not(target_os = "android"))]
    {
        commands.insert_resource(PendingVideoInit { asset_path });
    }
    #[cfg(target_os = "android")]
    {
        commands.insert_resource(PendingVideoInit { asset_path });
    }
    Entity::PLACEHOLDER
}

fn check_video_completion(
    mut commands: Commands,
    time: Res<Time>,
    mut pending_video: ResMut<PendingVideo>,
    mut advance_ev: MessageWriter<AdvanceEvent>,
    mut images: ResMut<Assets<Image>>,
    mut video_init: Option<ResMut<PendingVideoInit>>,
) {
    #[cfg(not(target_os = "android"))]
    {
        use gst::prelude::*;
        use gstreamer as gst;

        if pending_video.playing && pending_video.gst.is_none() {
            if let Some(init) = video_init.take() {
                let abs_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("assets")
                    .join(&init.asset_path);
                commands.remove_resource::<PendingVideoInit>();
                drop(init);

                let (pipeline, appsink, width, height) = if abs_path.exists() {
                    create_gst_pipeline(&abs_path)
                } else {
                    warn!("Video file not found: {:?}", abs_path);
                    (None, None, 1u32, 1u32)
                };

                if let (Some(pipeline), Some(appsink)) = (pipeline, appsink) {
                    let blank = Image::new(
                        wgpu_types::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        wgpu_types::TextureDimension::D2,
                        vec![0u8; 4],
                        wgpu_types::TextureFormat::Rgba8UnormSrgb,
                        bevy::asset::RenderAssetUsages::MAIN_WORLD
                            | bevy::asset::RenderAssetUsages::RENDER_WORLD,
                    );
                    let image_handle = images.add(blank);
                    let entity = commands
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            ImageNode::new(image_handle.clone()),
                            ZIndex(10),
                        ))
                        .id();

                    pending_video.gst = Some(GstVideoState {
                        pipeline,
                        appsink,
                        image_handle,
                        width,
                        height,
                    });
                    pending_video.entity = Some(entity);
                    info!("GStreamer pipeline started for: {:?}", abs_path);
                }
            }
        }

        let eos = pending_video
            .gst
            .as_ref()
            .map(|gst_state| {
                while let Some(sample) =
                    gst_state.appsink.try_pull_sample(gst::ClockTime::ZERO)
                {
                    let buffer = match sample.buffer() {
                        Some(b) => b,
                        None => continue,
                    };
                    let map = match buffer.map_readable() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    if let Some(img) = images.get_mut(&gst_state.image_handle) {
                        img.data = Some(map.as_slice().to_vec());
                    }
                }
                gst_state.appsink.is_eos()
            })
            .unwrap_or(false);

        if eos {
            info!("Video playback finished (EOS)");
            if let Some(ref gst_state) = pending_video.gst {
                let _ = gst_state.pipeline.set_state(gst::State::Null);
            }
            if let Some(entity) = pending_video.entity.take() {
                commands.entity(entity).despawn();
            }
            pending_video.playing = false;
            pending_video.gst = None;
            pending_video.timer = None;

            advance_ev.write(AdvanceEvent {
                source: AdvanceSource::Auto,
            });
            return;
        }
    }

    #[cfg(target_os = "android")]
    {
        if pending_video.playing && pending_video.ffmpeg.is_none() {
            if let Some(init) = video_init.take() {
                let abs_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("assets")
                    .join(&init.asset_path);
                commands.remove_resource::<PendingVideoInit>();
                drop(init);

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

    if let Some(ref mut timer) = pending_video.timer {
        timer.tick(time.delta());
        if timer.just_finished() {
            info!("Video timer fallback finished, resuming script");
            pending_video.playing = false;
            pending_video.timer = None;
            advance_ev.write(AdvanceEvent {
                source: AdvanceSource::Auto,
            });
        }
    }
}

fn cleanup_video(mut commands: Commands, mut pending_video: ResMut<PendingVideo>) {
    if let Some(entity) = pending_video.entity.take() {
        commands.entity(entity).despawn();
    }
    #[cfg(not(target_os = "android"))]
    {
        use gst::prelude::*;
        use gstreamer as gst;
        if let Some(ref gst_state) = pending_video.gst {
            let _ = gst_state.pipeline.set_state(gst::State::Null);
        }
        pending_video.gst = None;
    }
    #[cfg(target_os = "android")]
    {
        pending_video.ffmpeg = None;
    }
    pending_video.playing = false;
    pending_video.timer = None;
    commands.remove_resource::<PendingVideoInit>();
}

// ── Sprite video overlay (DrawSpriteEx) ──

pub fn spawn_sprite_video(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    sprite_mgr: &mut SpriteVideoManager,
    sprite_id: String,
    abs_path: &std::path::Path,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    priority: i32,
) -> Entity {
    #[cfg(not(target_os = "android"))]
    {
        let (pipeline, appsink, _width, _height) = if abs_path.exists() {
            create_gst_pipeline(abs_path)
        } else {
            warn!("Sprite video file not found: {:?}", abs_path);
            (None, None, 0, 0)
        };

        let (pipeline, appsink) = match (pipeline, appsink) {
            (Some(p), Some(a)) => (p, a),
            _ => return Entity::PLACEHOLDER,
        };

        let blank = Image::new(
            wgpu_types::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            wgpu_types::TextureDimension::D2,
            vec![0u8; 4],
            wgpu_types::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::MAIN_WORLD
                | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        let image_handle = images.add(blank);

        let entity = commands
            .spawn((
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
            ))
            .id();

        sprite_mgr.videos.insert(
            sprite_id.clone(),
            crate::resources::SpriteVideoGstState {
                pipeline,
                appsink,
                image_handle,
                entity,
                eos: false,
            },
        );

        info!("Sprite video entity {} created for sprite {}", entity, sprite_id);
        entity
    }

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
}

fn check_sprite_videos(
    mut commands: Commands,
    mut sprite_mgr: ResMut<SpriteVideoManager>,
    mut images: ResMut<Assets<Image>>,
    mut blocked: ResMut<PendingSpriteVideoBlock>,
    mut advance_ev: MessageWriter<AdvanceEvent>,
) {
    #[cfg(not(target_os = "android"))]
    {
        use gst::prelude::*;
        use gstreamer as gst;

        let mut finished: Vec<String> = Vec::new();
        for (sprite_id, state) in sprite_mgr.videos.iter() {
            while let Some(sample) = state.appsink.try_pull_sample(gst::ClockTime::ZERO) {
                if let Some(buffer) = sample.buffer() {
                    if let Ok(map) = buffer.map_readable() {
                        if let Some(img) = images.get_mut(&state.image_handle) {
                            img.data = Some(map.as_slice().to_vec());
                        }
                    }
                }
            }
            if state.appsink.is_eos() {
                finished.push(sprite_id.clone());
            }
        }

        for id in finished {
            info!("Sprite video EOS: sprite {}", id);
            if let Some(state) = sprite_mgr.videos.remove(&id) {
                let _ = state.pipeline.set_state(gst::State::Null);
                commands.entity(state.entity).despawn();
            }
            // If script runner was blocked on this sprite, wake it
            if blocked.0.as_deref() == Some(&id) {
                blocked.0 = None;
                advance_ev.write(AdvanceEvent {
                    source: AdvanceSource::Auto,
                });
            }
        }
    }

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
}

fn cleanup_sprite_videos(mut commands: Commands, mut sprite_mgr: ResMut<SpriteVideoManager>) {
    #[cfg(not(target_os = "android"))]
    {
        use gst::prelude::*;
        use gstreamer as gst;
        for (_id, state) in sprite_mgr.videos.drain() {
            let _ = state.pipeline.set_state(gst::State::Null);
            commands.entity(state.entity).despawn();
        }
    }
    #[cfg(target_os = "android")]
    {
        for (_id, state) in sprite_mgr.videos.drain() {
            commands.entity(state.entity).despawn();
        }
    }
}

pub fn is_sprite_video_playing(sprite_mgr: &SpriteVideoManager, sprite_id: &str) -> bool {
    #[cfg(not(target_os = "android"))]
    {
        sprite_mgr.videos.contains_key(sprite_id)
    }
    #[cfg(target_os = "android")]
    {
        sprite_mgr.videos.contains_key(sprite_id)
    }
}

pub fn stop_sprite_video(
    commands: &mut Commands,
    sprite_mgr: &mut SpriteVideoManager,
    sprite_id: &str,
) {
    #[cfg(not(target_os = "android"))]
    {
        if let Some(state) = sprite_mgr.videos.remove(sprite_id) {
            use gst::prelude::*;
            use gstreamer as gst;
            let _ = state.pipeline.set_state(gst::State::Null);
            commands.entity(state.entity).despawn();
            info!("Stopped sprite video: {}", sprite_id);
        }
    }
    #[cfg(target_os = "android")]
    {
        if let Some(state) = sprite_mgr.videos.remove(sprite_id) {
            commands.entity(state.entity).despawn();
            info!("Stopped sprite video: {}", sprite_id);
        }
    }
}

// ── Rain overlay ──

pub fn start_rain_video(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    rain: &mut RainOverlayState,
    abs_path: &std::path::Path,
    priority: i32,
) {
    // Stop existing rain first
    stop_rain_video(commands, rain);

    #[cfg(not(target_os = "android"))]
    {
        if !abs_path.exists() {
            warn!("Rain video file not found: {:?}", abs_path);
            return;
        }

        let (pipeline, appsink, _width, _height) = create_gst_pipeline(abs_path);

        let (pipeline, appsink) = match (pipeline, appsink) {
            (Some(p), Some(a)) => (p, a),
            _ => {
                warn!("Failed to create rain GStreamer pipeline");
                return;
            }
        };

        let blank = Image::new(
            wgpu_types::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            wgpu_types::TextureDimension::D2,
            vec![0u8; 4],
            wgpu_types::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::MAIN_WORLD
                | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        let image_handle = images.add(blank);

        let entity = commands
            .spawn((
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
            ))
            .id();

        rain.entity = Some(entity);
        rain.gst = Some(RainGstState {
            pipeline,
            appsink,
            image_handle,
            entity,
        });
        rain.enabled = true;
        info!("Rain overlay started");
    }

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
}

fn update_rain_video(
    mut commands: Commands,
    mut rain: ResMut<RainOverlayState>,
    mut images: ResMut<Assets<Image>>,
) {
    #[cfg(not(target_os = "android"))]
    {
        let Some(ref gst_state) = rain.gst else { return };
        use gstreamer as gst;

        while let Some(sample) = gst_state.appsink.try_pull_sample(gst::ClockTime::ZERO) {
            if let Some(buffer) = sample.buffer() {
                if let Ok(map) = buffer.map_readable() {
                    if let Some(img) = images.get_mut(&gst_state.image_handle) {
                        img.data = Some(map.as_slice().to_vec());
                    }
                }
            }
        }

        // EOS → clean up; next rain_mja will restart
        if gst_state.appsink.is_eos() {
            if let Some(ref gst_state) = rain.gst {
                use gst::prelude::*;
                let _ = gst_state.pipeline.set_state(gst::State::Null);
            }
            if let Some(entity) = rain.entity.take() {
                commands.entity(entity).despawn();
            }
            rain.gst = None;
            rain.enabled = false;
            info!("Rain video EOS — restart via rain_mja");
        }
    }

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
}

fn stop_rain_video_system(mut commands: Commands, mut rain: ResMut<RainOverlayState>) {
    stop_rain_video(&mut commands, &mut rain);
}

pub fn stop_rain_video(commands: &mut Commands, rain: &mut RainOverlayState) {
    if let Some(entity) = rain.entity.take() {
        commands.entity(entity).despawn();
    }
    #[cfg(not(target_os = "android"))]
    {
        use gstreamer as gst;
        if let Some(ref gst_state) = rain.gst {
            use gst::prelude::*;
            let _ = gst_state.pipeline.set_state(gst::State::Null);
        }
        rain.gst = None;
    }
    #[cfg(target_os = "android")]
    {
        rain.ffmpeg = None;
    }
    rain.enabled = false;
}

// ── Shared GStreamer pipeline builder ──

#[cfg(not(target_os = "android"))]
fn create_gst_pipeline(
    abs_path: &std::path::Path,
) -> (
    Option<gstreamer::Pipeline>,
    Option<gstreamer_app::AppSink>,
    u32,
    u32,
) {
    use gst::prelude::*;
    use gstreamer as gst;
    use gstreamer_app as gst_app;
    use gstreamer_video as gst_video;

    let uri = format!("file://{}", abs_path.display());

    let playbin = match gst::ElementFactory::make("playbin").build() {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to create playbin: {}", e);
            return (None, None, 0, 0);
        }
    };

    playbin.set_property("uri", &uri);

    if let Ok(fakesink) = gst::ElementFactory::make("fakesink").build() {
        playbin.set_property("audio-sink", &fakesink);
    }

    let appsink = gst::ElementFactory::make("appsink")
        .build()
        .unwrap_or_else(|_| panic!("Failed to create appsink"));
    let appsink = appsink
        .dynamic_cast::<gst_app::AppSink>()
        .unwrap_or_else(|_| panic!("Failed to cast to AppSink"));
    let caps = gst_video::VideoCapsBuilder::new()
        .format(gst_video::VideoFormat::Rgba)
        .build();
    appsink.set_caps(Some(&caps));
    appsink.set_property("sync", &false);
    appsink.set_property("max-buffers", &1u32);
    appsink.set_property("emit-signals", &false);

    playbin.set_property("video-sink", &appsink.upcast_ref::<gst::Element>());

    let pipeline = gst::Pipeline::new();
    if pipeline.add(&playbin).is_err() {
        warn!("Failed to add playbin to pipeline");
        return (None, None, 0, 0);
    }
    if pipeline.set_state(gst::State::Playing).is_err() {
        warn!("Failed to start GStreamer pipeline");
        return (None, None, 0, 0);
    }

    (Some(pipeline), Some(appsink), 1280, 720)
}

// ── Android FFmpeg pipeline builder ──

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

    ffmpeg::init().unwrap();

    let mut ictx = match format::input(&path_str) {
        Ok(ctx) => ctx,
        Err(e) => { warn!("ffmpeg: open failed: {}", e); return (None, 0, 0); }
    };

    let video_stream = match ictx.streams().best(media::Type::Video) {
        Some(s) => s,
        None => { warn!("ffmpeg: no video stream"); return (None, 0, 0); }
    };
    let stream_index = video_stream.index();
    let params = video_stream.parameters();

    let codec_ctx = match codec::context::Context::from_parameters(params) {
        Ok(c) => c,
        Err(e) => { warn!("ffmpeg: codec ctx: {}", e); return (None, 0, 0); }
    };
    let mut decoder = match codec_ctx.decoder().video() {
        Ok(d) => d,
        Err(e) => { warn!("ffmpeg: decoder: {}", e); return (None, 0, 0); }
    };
    let width = decoder.width();
    let height = decoder.height();

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

    let mut packets: Vec<ffmpeg::Packet> = Vec::new();
    for result in ictx.packets() {
        match result {
            Ok((_stream, packet)) => packets.push(packet),
            Err(e) => warn!("ffmpeg: packet read error: {}", e),
        }
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

/// Decode the next available frame from the pipeline.
/// Returns `Some(RGBA bytes)` when a new frame is ready.
#[cfg(target_os = "android")]
fn ffmpeg_decode_next_frame(
    pipeline: &mut crate::resources::FFmpegPipeline,
) -> Option<Vec<u8>> {
    use ffmpeg::*;
    use ffmpeg::codec::decoder::Video as _;

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

    let mut frame = util::frame::video::Video::empty();
    let mut rgba = util::frame::video::Video::empty();

    while pipeline.packet_cursor < pipeline.packets.len() {
        let packet = &pipeline.packets[pipeline.packet_cursor];
        pipeline.packet_cursor += 1;

        if pipeline.decoder.send_packet(packet).is_err() {
            continue;
        }

        if pipeline.decoder.receive_frame(&mut frame).is_ok() {
            if pipeline.scaler.run(&frame, &mut rgba).is_ok() {
                return Some(rgba.data(0).to_vec());
            }
        }
    }

    let _ = pipeline.decoder.send_eof();
    pipeline.flushed = true;

    if pipeline.decoder.receive_frame(&mut frame).is_ok() {
        if pipeline.scaler.run(&frame, &mut rgba).is_ok() {
            return Some(rgba.data(0).to_vec());
        }
    }

    pipeline.eos = true;
    None
}
