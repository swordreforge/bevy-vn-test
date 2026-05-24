use bevy::{audio::Volume, prelude::*};
use crate::audio_messages::*;
use crate::resources::BgmManager;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<BgmManager>()
            .add_message::<PlayBgmMessage>()
            .add_message::<StopBgmMessage>()
            .add_message::<PlaySeMessage>()
            .add_message::<PlayVoiceMessage>()
            .add_systems(Update, (
                handle_play_bgm,
                handle_stop_bgm,
                handle_play_se,
                handle_play_voice,
            ));
    }
}

fn handle_play_bgm(
    mut reader: MessageReader<PlayBgmMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
) {
    for msg in reader.read() {
        if let Some(entity) = bgm.entity {
            commands.entity(entity).despawn();
        }

        let path = format!("audio/bgm/bgm_{}_a.ogg", msg.id);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        let volume = msg.volume.unwrap_or(1.0);
        let entity = commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::Linear(volume),
                ..default()
            },
        )).id();

        bgm.current_id = Some(msg.id.clone());
        bgm.entity = Some(entity);
    }
}

fn handle_stop_bgm(
    mut reader: MessageReader<StopBgmMessage>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
) {
    for _ in reader.read() {
        if let Some(entity) = bgm.entity {
            commands.entity(entity).despawn();
        }
        bgm.current_id = None;
        bgm.entity = None;
    }
}

fn handle_play_se(
    mut reader: MessageReader<PlaySeMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let path = format!("audio/se/{}.ogg", msg.file);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN,
        ));
    }
}

fn handle_play_voice(
    mut reader: MessageReader<PlayVoiceMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let path = format!("audio/voice/{}.ogg", msg.file);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN,
        ));
    }
}
