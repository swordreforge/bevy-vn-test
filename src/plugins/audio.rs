use bevy::{audio::Volume, prelude::*};
use crate::audio_messages::*;
use crate::components::{AudioType, BgmFade, BgmFadeLayer};
use crate::resources::{BgmManager, BgmXManager, PendingBgm, PendingBgmLoad, PendingSe, PendingSeLoad, SeKind, SeManager, Settings, VoiceManager};
use rodio::{self, Source};
use std::io::Cursor;
use std::sync::Arc;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<BgmManager>()
            .init_resource::<BgmXManager>()
            .init_resource::<VoiceManager>()
            .init_resource::<SeManager>()
            .init_resource::<PendingSe>()
            .init_resource::<PendingBgm>()
            .add_message::<PlayBgmMessage>()
            .add_message::<StopBgmMessage>()
            .add_message::<PlayBgmXMessage>()
            .add_message::<StopBgmXMessage>()
            .add_message::<PlaySeMessage>()
            .add_message::<LoopSeMessage>()
            .add_message::<StopStreamingSeMessage>()
            .add_message::<PlayVoiceMessage>()
            .add_systems(Update, (
                handle_stop_bgm,
                handle_play_bgm,
                process_pending_bgm,
                handle_stop_bgmx,
                handle_play_bgmx,
                handle_play_se,
                handle_loop_se,
                handle_stop_streaming_se,
                process_pending_se,
                handle_play_voice,
                apply_audio_settings,
                update_bgm_fade,
            ).chain());
    }
}

fn asset_path_exists(relative: &str) -> bool {
    let path = std::path::Path::new("assets").join(relative);
    if path.exists() {
        return true;
    }
    // Android: assets are inside APK, not on filesystem.
    // Fall back to _a.ogg naming convention; if the file is truly missing,
    // process_pending_bgm's 60-frame timeout will handle it.
    #[cfg(target_os = "android")]
    {
        relative.contains("_a.ogg")
    }
    #[cfg(not(target_os = "android"))]
    {
        false
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
        let fade_in_ms = msg.fade_in.unwrap_or(0);
        let has_fade = fade_in_ms > 0;
        let fade_in_sec = fade_in_ms as f32 / 1000.0;

        pending.0 = None;
        bgm.current_id = Some(msg.id.clone());

        if let Some(old_entity) = bgm.entity.take() {
            if has_fade {
                if let Ok(mut cmd) = commands.get_entity(old_entity) {
                    cmd.insert(BgmFade {
                        timer: Timer::from_seconds(fade_in_sec, TimerMode::Once),
                        start_mult: 1.0,
                        end_mult: 0.0,
                        layer: BgmFadeLayer::Bgm,
                    });
                }
            } else {
                if let Ok(mut cmd) = commands.get_entity(old_entity) {
                    cmd.despawn();
                }
            }
        }

        let volume = msg.volume.unwrap_or(1.0);
        let path_a = format!("audio/bgm/bgm_{}_a.ogg", msg.id);
        let path_b = format!("audio/bgm/bgm_{}_b.ogg", msg.id);

        let (handle_a, handle_b) = if asset_path_exists(&path_a) {
            (asset_server.load(&path_a), asset_server.load(&path_b))
        } else {
            let single = asset_server.load(format!("audio/bgm/bgm_{}.ogg", msg.id));
            (single.clone(), single)
        };
        pending.0 = Some(PendingBgmLoad {
            id: msg.id.clone(),
            handle_a,
            handle_b,
            volume,
            has_fade,
            fade_in_sec,
            frames_waited: 0,
        });
    }
}

fn handle_stop_bgm(
    mut reader: MessageReader<StopBgmMessage>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
    mut pending: ResMut<PendingBgm>,
) {
    for msg in reader.read() {
        pending.0 = None;
        if let Some(entity) = bgm.entity.take() {
            let fade_out_ms = msg.fade_out.unwrap_or(0);
            if fade_out_ms > 0 {
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.insert(BgmFade {
                        timer: Timer::from_seconds(fade_out_ms as f32 / 1000.0, TimerMode::Once),
                        start_mult: 1.0,
                        end_mult: 0.0,
                        layer: BgmFadeLayer::Bgm,
                    });
                }
            } else {
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.despawn();
                }
            }
        }
        bgm.current_id = None;
    }
}

fn process_pending_bgm(
    mut pending: ResMut<PendingBgm>,
    mut assets: ResMut<Assets<AudioSource>>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
) {
    let Some(ref mut p) = pending.0 else { return };
    let Some(ref source_a) = assets.get(&p.handle_a) else { p.frames_waited = 0; return };

    let id = p.id.clone();
    let volume = p.volume;
    let spawn_bgm = |cmds: &mut Commands, handle: Handle<AudioSource>, fade: bool, fade_sec: f32| {
        let mut entity = cmds.spawn((
            AudioPlayer(handle),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::Linear(if fade { 0.0 } else { volume }),
                ..default()
            },
            AudioType::Bgm,
        ));
        if fade {
            entity.insert(BgmFade {
                timer: Timer::from_seconds(fade_sec, TimerMode::Once),
                start_mult: 0.0,
                end_mult: 1.0,
                layer: BgmFadeLayer::Bgm,
            });
        }
        entity.id()
    };

    if let Some(ref source_b) = assets.get(&p.handle_b) {
        let combined = concat_ogg_bytes(&source_a.bytes, &source_b.bytes);
        let combined_source = AudioSource { bytes: Arc::from(combined) };
        let handle = assets.add(combined_source);
        bgm.entity = Some(spawn_bgm(&mut commands, handle, p.has_fade, p.fade_in_sec));
        pending.0 = None;
        info!("BGM concat: {} complete", id);
    } else if p.frames_waited >= 60 {
        bgm.entity = Some(spawn_bgm(&mut commands, p.handle_a.clone(), p.has_fade, p.fade_in_sec));
        pending.0 = None;
        info!("BGM single (no B layer): {} complete", id);
    } else {
        p.frames_waited += 1;
    }
}

fn handle_play_bgmx(
    mut reader: MessageReader<PlayBgmXMessage>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut bgmx: ResMut<BgmXManager>,
) {
    for msg in reader.read() {
        let fade_in_ms = msg.fade_in.unwrap_or(0);
        let has_fade = fade_in_ms > 0;
        let fade_in_sec = fade_in_ms as f32 / 1000.0;

        bgmx.current_id = Some(msg.id.clone());

        if let Some(old_entity) = bgmx.entity.take() {
            if has_fade {
                if let Ok(mut cmd) = commands.get_entity(old_entity) {
                    cmd.insert(BgmFade {
                        timer: Timer::from_seconds(fade_in_sec, TimerMode::Once),
                        start_mult: 1.0,
                        end_mult: 0.0,
                        layer: BgmFadeLayer::BgmX,
                    });
                }
            } else {
                if let Ok(mut cmd) = commands.get_entity(old_entity) {
                    cmd.despawn();
                }
            }
        }

        let volume = msg.volume.unwrap_or(1.0);
        let path = format!("audio/bgm/bgmx_{}.ogg", msg.id);
        let handle: Handle<AudioSource> = asset_server.load(&path);
        let mut spawn_cmd = commands.spawn((
            AudioPlayer(handle),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::Linear(if has_fade { 0.0 } else { volume }),
                ..default()
            },
            AudioType::BgmX,
        ));

        if has_fade {
            spawn_cmd.insert(BgmFade {
                timer: Timer::from_seconds(fade_in_sec, TimerMode::Once),
                start_mult: 0.0,
                end_mult: 1.0,
                layer: BgmFadeLayer::BgmX,
            });
        }

        bgmx.entity = Some(spawn_cmd.id());
    }
}

fn handle_stop_bgmx(
    mut reader: MessageReader<StopBgmXMessage>,
    mut commands: Commands,
    mut bgmx: ResMut<BgmXManager>,
) {
    for msg in reader.read() {
        if let Some(entity) = bgmx.entity.take() {
            let fade_out_ms = msg.fade_out.unwrap_or(0);
            if fade_out_ms > 0 {
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.insert(BgmFade {
                        timer: Timer::from_seconds(fade_out_ms as f32 / 1000.0, TimerMode::Once),
                        start_mult: 1.0,
                        end_mult: 0.0,
                        layer: BgmFadeLayer::BgmX,
                    });
                }
            } else {
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.despawn();
                }
            }
        }
        bgmx.current_id = None;
    }
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

fn handle_play_se(
    mut reader: MessageReader<PlaySeMessage>,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingSe>,
) {
    for msg in reader.read() {
        let path_a = format!("audio/se/{}_a.ogg", msg.file);
        let path_single = format!("audio/se/{}.ogg", msg.file);
        if asset_path_exists(&path_a) {
            let handle_a = asset_server.load(&path_a);
            let handle_b = asset_server.load(format!("audio/se/{}_b.ogg", msg.file));
            pending.0.push(PendingSeLoad {
                file: msg.file.clone(),
                handle_a: handle_a.clone(),
                handle_b: Some(handle_b),
                handle_single: Some(handle_a),
                kind: SeKind::OneShot,
                frames_waited: 0,
            });
        } else {
            let handle = asset_server.load(&path_single);
            pending.0.push(PendingSeLoad {
                file: msg.file.clone(),
                handle_a: handle.clone(),
                handle_b: None,
                handle_single: Some(handle),
                kind: SeKind::OneShot,
                frames_waited: 0,
            });
        }
    }
}

fn handle_loop_se(
    mut reader: MessageReader<LoopSeMessage>,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingSe>,
    mut se: ResMut<SeManager>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        if let Some(old) = se.entities.remove(&msg.channel) {
            if let Ok(mut cmd) = commands.get_entity(old) {
                cmd.despawn();
            }
        }
        let vol = msg.volume.unwrap_or(1.0);
        let path_a = format!("audio/se/{}_a.ogg", msg.file);
        let path_single = format!("audio/se/{}.ogg", msg.file);
        if asset_path_exists(&path_a) {
            let handle_a = asset_server.load(&path_a);
            let handle_b = asset_server.load(format!("audio/se/{}_b.ogg", msg.file));
            pending.0.push(PendingSeLoad {
                file: msg.file.clone(),
                handle_a: handle_a.clone(),
                handle_b: Some(handle_b),
                handle_single: Some(handle_a),
                kind: SeKind::Loop {
                    channel: msg.channel,
                    volume: vol,
                },
                frames_waited: 0,
            });
        } else {
            let handle = asset_server.load(&path_single);
            pending.0.push(PendingSeLoad {
                file: msg.file.clone(),
                handle_a: handle.clone(),
                handle_b: None,
                handle_single: Some(handle),
                kind: SeKind::Loop {
                    channel: msg.channel,
                    volume: vol,
                },
                frames_waited: 0,
            });
        }
    }
}

fn handle_stop_streaming_se(
    mut reader: MessageReader<StopStreamingSeMessage>,
    mut commands: Commands,
    mut se: ResMut<SeManager>,
) {
    for msg in reader.read() {
        if let Some(entity) = se.entities.remove(&msg.channel) {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
        }
    }
}

fn process_pending_se(
    mut pending: ResMut<PendingSe>,
    mut assets: ResMut<Assets<AudioSource>>,
    mut commands: Commands,
    mut se: ResMut<SeManager>,
) {
    let mut i = 0;
    while i < pending.0.len() {
        let p = &mut pending.0[i];
        let has_b = p.handle_b.as_ref().and_then(|h| assets.get(h));
        match assets.get(&p.handle_a) {
            Some(a) => {
                if let Some(b) = has_b {
                    let combined = concat_ogg_bytes(&a.bytes, &b.bytes);
                    let combined_source = AudioSource { bytes: std::sync::Arc::from(combined) };
                    let handle = assets.add(combined_source);
                    spawn_se(&mut commands, &mut se, &p.kind, handle);
                    info!("SE concat: {} complete", p.file);
                    pending.0.swap_remove(i);
                } else if p.frames_waited >= 2 {
                    let handle = p.handle_a.clone();
                    spawn_se(&mut commands, &mut se, &p.kind, handle);
                    info!("SE single (no B layer): {}", p.file);
                    pending.0.swap_remove(i);
                } else {
                    p.frames_waited += 1;
                    i += 1;
                }
            }
            None => {
                let loaded_single = p.handle_single.as_ref().and_then(|h| assets.get(h));
                if let Some(single) = loaded_single {
                    let cloned_bytes = single.bytes.clone();
                    let handle = assets.add(AudioSource { bytes: cloned_bytes });
                    spawn_se(&mut commands, &mut se, &p.kind, handle);
                    info!("SE single (no _a/_b): {}", p.file);
                    pending.0.swap_remove(i);
                } else if p.frames_waited >= 30 {
                    if let Some(h) = p.handle_single.clone() {
                        spawn_se(&mut commands, &mut se, &p.kind, h);
                        warn!("SE fallback (not yet loaded): {}", p.file);
                    } else {
                        warn!("SE files not found, skipping: {}", p.file);
                    }
                    pending.0.swap_remove(i);
                } else {
                    p.frames_waited += 1;
                    i += 1;
                }
            }
        }
    }
}

fn spawn_se(
    commands: &mut Commands,
    se: &mut ResMut<SeManager>,
    kind: &SeKind,
    handle: Handle<AudioSource>,
) {
    match kind {
        SeKind::OneShot => {
            commands.spawn((
                AudioPlayer(handle),
                PlaybackSettings::DESPAWN,
                AudioType::Se,
            ));
        }
        SeKind::Loop { channel, volume } => {
            let ch = *channel;
            let vol = *volume;
            if let Some(old) = se.entities.remove(&ch) {
                if let Ok(mut cmd) = commands.get_entity(old) {
                    cmd.despawn();
                }
            }
            let entity = commands.spawn((
                AudioPlayer(handle),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: Volume::Linear(vol),
                    ..default()
                },
                AudioType::Se,
            )).id();
            se.entities.insert(ch, entity);
        }
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
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
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
    mut query: Query<(&AudioType, &mut AudioSink), Without<BgmFade>>,
) {
    for (audio_type, mut sink) in query.iter_mut() {
        let volume = match audio_type {
            AudioType::Bgm | AudioType::BgmX => settings.bgm_volume,
            AudioType::Se => settings.se_volume,
            AudioType::Voice => settings.voice_volume,
        };
        sink.set_volume(Volume::Linear(volume));
    }
}

fn update_bgm_fade(
    time: Res<Time>,
    settings: Res<Settings>,
    mut commands: Commands,
    mut bgm: ResMut<BgmManager>,
    mut bgmx: ResMut<BgmXManager>,
    mut query: Query<(Entity, &AudioType, &mut AudioSink, &mut BgmFade)>,
) {
    for (entity, audio_type, mut sink, mut fade) in query.iter_mut() {
        fade.timer.tick(time.delta());
        let t = fade.timer.fraction().min(1.0);
        let mult = fade.start_mult + (fade.end_mult - fade.start_mult) * t;

        let base_volume = match audio_type {
            AudioType::Bgm | AudioType::BgmX => settings.bgm_volume,
            _ => 1.0,
        };
        sink.set_volume(Volume::Linear(base_volume * mult));

        if fade.timer.just_finished() {
            sink.set_volume(Volume::Linear(base_volume * fade.end_mult));
            if fade.end_mult <= 0.0 {
                match fade.layer {
                    BgmFadeLayer::Bgm => bgm.entity = None,
                    BgmFadeLayer::BgmX => bgmx.entity = None,
                }
                commands.entity(entity).despawn();
            } else {
                commands.entity(entity).remove::<BgmFade>();
            }
        }
    }
}
