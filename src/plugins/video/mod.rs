// This module is heavily platform-gated. On Android, many params are intentionally unused.
#![cfg_attr(target_os = "android", allow(unused, unused_mut))]

use crate::plugins::inputs::{AdvanceEvent, AdvanceSource};
use crate::resources::{
    PendingSpriteVideoBlock, PendingVideo, PendingVideoInit,
    RainOverlayState, SpriteVideoManager,
};
#[cfg(not(target_os = "android"))]
use crate::resources::{GstVideoState, RainGstState};
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
        info!("Video not supported on Android: {}", asset_path);
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
        info!("Sprite video not supported on Android");
        Entity::PLACEHOLDER
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
}

pub fn is_sprite_video_playing(sprite_mgr: &SpriteVideoManager, sprite_id: &str) -> bool {
    #[cfg(not(target_os = "android"))]
    {
        sprite_mgr.videos.contains_key(sprite_id)
    }
    #[cfg(target_os = "android")]
    {
        false
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
