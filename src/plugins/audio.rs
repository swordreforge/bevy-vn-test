use bevy::{audio::Volume, prelude::*};
use crate::audio_messages::*;
use crate::components::AudioType;
use crate::resources::{BgmManager, PendingBgm, PendingBgmLoad, Settings, VoiceManager};
use rodio::{self, Source};
use std::io::Cursor;
use std::sync::Arc;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<BgmManager>()
            .init_resource::<VoiceManager>()
            .init_resource::<PendingBgm>()
            .add_message::<PlayBgmMessage>()
            .add_message::<StopBgmMessage>()
            .add_message::<PlaySeMessage>()
            .add_message::<PlayVoiceMessage>()
            .add_systems(Update, (
                handle_play_bgm,
                process_pending_bgm,
                handle_stop_bgm,
                handle_play_se,
                handle_play_voice,
                apply_audio_settings,
            ));
    }
}

fn handle_play_bgm(
    mut reader: MessageReader<PlayBgmMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
    mut pending: ResMut<PendingBgm>,
) {
    for msg in reader.read() {
        if let Some(entity) = bgm.entity.take() {
            commands.entity(entity).despawn();
        }
        pending.0 = None;
        bgm.current_id = Some(msg.id.clone());

        let volume = msg.volume.unwrap_or(1.0);
        let path_a = format!("audio/bgm/bgm_{}_a.ogg", msg.id);
        let path_b = format!("audio/bgm/bgm_{}_b.ogg", msg.id);

        if std::path::Path::new(&format!("assets/{}", path_b)).exists() {
            let handle_a: Handle<AudioSource> = asset_server.load(&path_a);
            let handle_b: Handle<AudioSource> = asset_server.load(&path_b);
            pending.0 = Some(PendingBgmLoad { id: msg.id.clone(), handle_a, handle_b, volume });
        } else {
            let handle: Handle<AudioSource> = asset_server.load(&path_a);
            let entity = commands.spawn((
                AudioPlayer(handle),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: Volume::Linear(volume),
                    ..default()
                },
                AudioType::Bgm,
            )).id();
            bgm.entity = Some(entity);
        }
    }
}

fn process_pending_bgm(
    mut pending: ResMut<PendingBgm>,
    mut assets: ResMut<Assets<AudioSource>>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
) {
    let Some(ref p) = pending.0 else { return };
    let Some(ref source_a) = assets.get(&p.handle_a) else { return };
    let Some(ref source_b) = assets.get(&p.handle_b) else { return };

    let id = p.id.clone();
    let volume = p.volume;

    let combined = concat_ogg_bytes(&source_a.bytes, &source_b.bytes);
    let combined_source = AudioSource { bytes: Arc::from(combined) };
    let handle = assets.add(combined_source);

    let entity = commands.spawn((
        AudioPlayer(handle),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Loop,
            volume: Volume::Linear(volume),
            ..default()
        },
        AudioType::Bgm,
    )).id();
    bgm.entity = Some(entity);
    pending.0 = None;
    info!("BGM {}: concatenated _a + _b", id);
}

fn concat_ogg_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
    let a_decoder = rodio::Decoder::new(Cursor::new(a.to_vec()));
    let b_decoder = rodio::Decoder::new(Cursor::new(b.to_vec()));
    let (Ok(dec_a), Ok(dec_b)) = (a_decoder, b_decoder) else {
        warn!("Failed to decode BGM segment, falling back to raw concatenation");
        let mut buf = Vec::with_capacity(a.len() + b.len());
        buf.extend_from_slice(a);
        buf.extend_from_slice(b);
        return buf;
    };

    let channels = dec_a.channels();
    let sample_rate = dec_a.sample_rate();
    let samples_a: Vec<i16> = dec_a.collect();
    let samples_b: Vec<i16> = dec_b.collect();

    let mut pcm = Vec::with_capacity((samples_a.len() + samples_b.len()) * 2);
    for &s in samples_a.iter().chain(samples_b.iter()) {
        pcm.extend_from_slice(&s.to_le_bytes());
    }

    let data_size = pcm.len() as u32;
    let bytes_per_sec = sample_rate * channels as u32 * 2;
    let block_align = channels as u16 * 2;
    let buf_size = 44 + data_size as usize;

    let mut wav = Vec::with_capacity(buf_size);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(buf_size as u32 - 8).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&(channels as u16).to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&bytes_per_sec.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(&pcm);
    wav
}

fn handle_stop_bgm(
    mut reader: MessageReader<StopBgmMessage>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
    mut pending: ResMut<PendingBgm>,
) {
    for _ in reader.read() {
        pending.0 = None;
        if let Some(entity) = bgm.entity.take() {
            commands.entity(entity).despawn();
        }
        bgm.current_id = None;
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
            AudioType::Se,
        ));
    }
}

fn handle_play_voice(
    mut reader: MessageReader<PlayVoiceMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut voice: ResMut<VoiceManager>,
) {
    for msg in reader.read() {
        if let Some(entity) = voice.entity.take() {
            commands.entity(entity).despawn();
        }
        let path = format!("audio/voice/{}.ogg", msg.file);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        let entity = commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN,
            AudioType::Voice,
        )).id();
        voice.entity = Some(entity);
    }
}

fn apply_audio_settings(
    settings: Res<Settings>,
    mut query: Query<(&AudioType, &mut AudioSink)>,
) {
    for (audio_type, mut sink) in query.iter_mut() {
        let volume = match audio_type {
            AudioType::Bgm => settings.bgm_volume,
            AudioType::Se => settings.se_volume,
            AudioType::Voice => settings.voice_volume,
        };
        sink.set_volume(Volume::Linear(volume));
    }
}
