use crate::plugins::inputs::{AdvanceEvent, AdvanceSource};
use crate::resources::PendingVideo;
use bevy::prelude::*;

/// Desktop video playback plugin.
///
/// Currently uses a timer-based stub (same as Android) because
/// bevy_movie_player / ffmpeg-sys-next is incompatible with
/// ffmpeg >= 8.0 on this system.
///
/// When ffmpeg-sys-next gains ffmpeg 8.x support, replace the
/// stub with a real ffmpeg-next decoder:
///   1. Spawn a decoder thread that pipes RGBA frames via channel
///   2. Update a Bevy Image handle each frame
///   3. Set pending_video.entity for completion tracking
///   4. Play audio track via the existing AudioPlugin
///
/// See docs/video-system-implementation.md for the full design.
pub struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            check_video_completion.run_if(in_state(crate::state::AppState::Gameplay)),
        );
    }
}

/// Spawn a video player.
///
/// On desktop with ffmpeg: spawns a decoder entity.
/// On Android: no-op (returns placeholder entity).
/// Currently both paths use a timer-based stub (Phase 2 behavior).
pub fn spawn_video(_commands: &mut Commands, asset_path: String) -> Entity {
    info!("Playing video (stub): {}", asset_path);
    Entity::PLACEHOLDER
}

/// Check if the current video has finished.
///
/// Desktop stub: timer-based (3s fallback).
/// Android: timer-based (until Phase 4 MediaPlayer JNI).
fn check_video_completion(
    time: Res<Time>,
    mut pending_video: ResMut<PendingVideo>,
    mut advance_ev: MessageWriter<AdvanceEvent>,
) {
    if !pending_video.playing {
        return;
    }
    if let Some(timer) = &mut pending_video.timer {
        timer.tick(time.delta());
        if timer.just_finished() {
            info!("Video stub finished, resuming script");
            pending_video.playing = false;
            pending_video.timer = None;
            advance_ev.write(AdvanceEvent {
                source: AdvanceSource::Auto,
            });
        }
    } else {
        pending_video.playing = false;
    }
}
