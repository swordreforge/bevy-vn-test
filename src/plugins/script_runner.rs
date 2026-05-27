use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use crate::resources::{AffectionMap, Backlog, BacklogEntry, ChoiceState, DialogueState, IntroPhase, QuakeState, Settings, SpriteOverlayManager, UnlockState};
use crate::audio_messages::{
    PlayBgmMessage, StopBgmMessage, PlaySeMessage, LoopSeMessage, StopStreamingSeMessage, PlayVoiceMessage,
};
use crate::choice_messages::ChoiceSelectedMessage;
use crate::components::{DialogueUiRoot, OverlayTween, ScreenOverlayRoot};
use crate::resources::WindowOverride;
use crate::script::{ConditionOp, OverlayColor, ScriptCmd, ScriptEngine};
use crate::state::AppState;
use crate::plugins::inputs::AdvanceEvent;
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage,
    ShowFaceMessage, HideFaceMessage,
    ShowCgMessage, HideCgMessage,
    DrawSpriteMessage, FadeSpriteMessage, MoveSpriteMessage,
};

pub struct ScriptRunnerPlugin;

#[derive(Resource)]
pub struct AutoSkipTimer {
    pub auto_timer: Option<Timer>,
    pub skip_timer: Option<Timer>,
}

impl Default for AutoSkipTimer {
    fn default() -> Self {
        Self { auto_timer: None, skip_timer: None }
    }
}

#[derive(SystemParam)]
pub struct ProcessAdvanceParams<'w, 's> {
    advance_ev: MessageReader<'w, 's, AdvanceEvent>,
    engine: ResMut<'w, ScriptEngine>,
    dialogue: ResMut<'w, DialogueState>,
    affection: ResMut<'w, AffectionMap>,
    backlog: ResMut<'w, Backlog>,
    unlock_state: ResMut<'w, UnlockState>,
    set_bg_writer: MessageWriter<'w, SetBgMessage>,
    show_fg_writer: MessageWriter<'w, ShowFgMessage>,
    hide_fg_writer: MessageWriter<'w, HideFgMessage>,
    show_face_writer: MessageWriter<'w, ShowFaceMessage>,
    hide_face_writer: MessageWriter<'w, HideFaceMessage>,
    show_cg_writer: MessageWriter<'w, ShowCgMessage>,
    hide_cg_writer: MessageWriter<'w, HideCgMessage>,
    draw_sprite_writer: MessageWriter<'w, DrawSpriteMessage>,
    fade_sprite_writer: MessageWriter<'w, FadeSpriteMessage>,
    move_sprite_writer: MessageWriter<'w, MoveSpriteMessage>,
    choice_ev: MessageReader<'w, 's, ChoiceSelectedMessage>,
    choice_state: ResMut<'w, ChoiceState>,
    play_bgm_writer: MessageWriter<'w, PlayBgmMessage>,
    stop_bgm_writer: MessageWriter<'w, StopBgmMessage>,
    play_se_writer: MessageWriter<'w, PlaySeMessage>,
    loop_se_writer: MessageWriter<'w, LoopSeMessage>,
    stop_streaming_se_writer: MessageWriter<'w, StopStreamingSeMessage>,
    play_voice_writer: MessageWriter<'w, PlayVoiceMessage>,
    settings: Res<'w, Settings>,
    auto_skip: ResMut<'w, AutoSkipTimer>,
    intro: ResMut<'w, IntroPhase>,
    overlay_mgr: ResMut<'w, SpriteOverlayManager>,
}

impl Plugin for ScriptRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoSkipTimer>()
            .init_resource::<IntroPhase>()
            .init_resource::<WindowOverride>()
            .add_systems(OnEnter(AppState::Gameplay), (start_script_execution, start_intro_bgm))
            .add_systems(OnEnter(AppState::Title), reset_engine_on_title)
            .add_systems(
                Update,
                (
                    handle_auto_skip,
                    process_advance,
                    update_text_reveal,
                )
                    .chain()
                    .run_if(in_state(AppState::Gameplay)),
            );
    }
}

fn start_script_execution(
    mut dialogue: ResMut<DialogueState>,
) {
    dialogue.current_text.clear();
    dialogue.current_speaker = None;
    dialogue.text_progress = 0;
    dialogue.is_displaying = false;
}

fn start_intro_bgm(
    engine: Res<ScriptEngine>,
    mut play_bgm: MessageWriter<PlayBgmMessage>,
    mut intro: ResMut<IntroPhase>,
) {
    if engine.current_line != 0 {
        return;
    }
    let is_start = engine.current_script == "main"
        || engine.current_script == "aiy00010";
    if is_start {
        play_bgm.write(PlayBgmMessage {
            id: "0304".to_string(),
            volume: None,
            fade_in: None,
        });
        intro.0 = true;
    }
}

fn reset_engine_on_title(mut engine: ResMut<ScriptEngine>) {
    engine.current_line = 0;
    engine.call_stack.clear();
    engine.flags.clear();
    engine.dialogue_idx = 0;
    if engine.scripts.contains_key("main") {
        engine.current_script = "main".to_string();
    } else if engine.scripts.contains_key("aiy00010") {
        engine.current_script = "aiy00010".to_string();
    }
}

fn process_advance(
    mut params: ProcessAdvanceParams<'_, '_>,
    mut commands: Commands,
    mut overlay_query: Query<(Entity, &mut BackgroundColor, &mut Visibility), With<ScreenOverlayRoot>>,
    mut window_query: Query<&mut Visibility, (With<DialogueUiRoot>, Without<ScreenOverlayRoot>)>,
    mut window_override: ResMut<WindowOverride>,
) {
    let ProcessAdvanceParams {
        ref mut advance_ev,
        ref mut engine,
        ref mut dialogue,
        ref mut affection,
        ref mut backlog,
        ref mut unlock_state,
        ref mut set_bg_writer,
        ref mut show_fg_writer,
        ref mut hide_fg_writer,
        ref mut show_face_writer,
        ref mut hide_face_writer,
        ref mut show_cg_writer,
        ref mut hide_cg_writer,
        ref mut draw_sprite_writer,
        ref mut fade_sprite_writer,
        ref mut move_sprite_writer,
        ref mut choice_ev,
        ref mut choice_state,
        ref mut play_bgm_writer,
        ref mut stop_bgm_writer,
        ref mut play_se_writer,
        ref mut loop_se_writer,
        ref mut stop_streaming_se_writer,
        ref mut play_voice_writer,
        ref mut settings,
        ref mut auto_skip,
        ref mut intro,
        ref mut overlay_mgr,
    } = &mut params;

    for _ in advance_ev.read() {
        // Reset auto/skip timers on manual advance
        auto_skip.auto_timer = None;
        auto_skip.skip_timer = None;

        // If choice is active, check if user made a selection
        if choice_state.active {
            for ev in choice_ev.read() {
                if ev.index < choice_state.options.len() {
                    let option = &choice_state.options[ev.index];
                    if let Some((ref char_id, delta)) = option.affection_change {
                        *affection.0.entry(char_id.clone()).or_insert(0) += delta;
                    }
                    if let Some(ref target) = option.goto {
                        if !engine.jump_to_label(target) {
                            warn!("Choice jump target not found: {}", target);
                        }
                    }
                }
                choice_state.active = false;
                choice_state.options.clear();
            }
            continue;
        }

        if dialogue.is_displaying && dialogue.text_progress < dialogue.current_text.len() {
            dialogue.text_progress = dialogue.current_text.len();
            continue;
        }

        if dialogue.is_displaying && dialogue.text_progress >= dialogue.current_text.len() {
            dialogue.is_displaying = false;
            continue;
        }

        // Skip mode: when not in a choice and skip is enabled, skip through everything
        if settings.skip_mode {
            let mut pending_voice = None;
            while engine.has_more() {
                let cmd = engine.advance().cloned();
                match cmd {
                    Some(ScriptCmd::Dialogue { speaker, text }) => {
                        for (_, entity) in overlay_mgr.sprites.drain() {
                            commands.entity(entity).despawn();
                        }
                        engine.dialogue_idx += 1;
                        if intro.0 && speaker.is_some() {
                            intro.0 = false;
                        }
                        let text_clone = text.clone();
                        backlog.entries.push(BacklogEntry {
                            speaker: speaker.clone(),
                            text: text_clone,
                            voice_file: pending_voice.take(),
                        });
                        if backlog.entries.len() > 200 {
                            backlog.entries.remove(0);
                        }
                        dialogue.current_speaker = speaker;
                        let text_len = text.len();
                        dialogue.current_text = text;
                        dialogue.text_progress = text_len;
                        dialogue.is_displaying = false;
                        window_override.0 = false;
                    }
                    Some(ScriptCmd::ClearText) => {
                        dialogue.current_text.clear();
                        dialogue.current_speaker = None;
                        dialogue.text_progress = 0;
                        dialogue.is_displaying = false;
                    }
                    Some(ScriptCmd::Jump { target }) => {
                        if !engine.jump_to_label(&target) {
                            warn!("Jump target not found: {}", target);
                        }
                    }
                    Some(ScriptCmd::Call { target }) => {
                        engine.call_label(&target);
                    }
                    Some(ScriptCmd::CallScript { script, label }) => {
                        engine.call_script(&script, label.as_deref());
                    }
                    Some(ScriptCmd::Return) => {
                        engine.return_from_call();
                    }
                    Some(ScriptCmd::Condition { var, value, operator, goto }) => {
                        let flag_val = engine.flags.get(&var).copied().unwrap_or(0);
                        let met = match operator {
                            ConditionOp::Greater => flag_val > value,
                            ConditionOp::Less => flag_val < value,
                            ConditionOp::Equal => flag_val == value,
                            ConditionOp::GreaterEqual => flag_val >= value,
                            ConditionOp::LessEqual => flag_val <= value,
                        };
                        if met && !engine.jump_to_label(&goto) {
                            warn!("Condition jump target not found: {}", goto);
                        }
                    }
                    Some(ScriptCmd::SetFlag { name, value }) => {
                        engine.flags.insert(name, value);
                    }
                    Some(ScriptCmd::Halt) => {
                        engine.call_stack.clear();
                        engine.current_script.clear();
                        engine.current_line = 0;
                    }
                    Some(ScriptCmd::AffectionChange { char_id, delta }) => {
                        *affection.0.entry(char_id).or_insert(0) += delta;
                    }
                    Some(ScriptCmd::AffectionCondition { char_id, value, operator, goto }) => {
                        let affection_val = affection.0.get(&char_id).copied().unwrap_or(0);
                        let met = match operator {
                            ConditionOp::Greater => affection_val > value,
                            ConditionOp::Less => affection_val < value,
                            ConditionOp::Equal => affection_val == value,
                            ConditionOp::GreaterEqual => affection_val >= value,
                            ConditionOp::LessEqual => affection_val <= value,
                        };
                        if met && !engine.jump_to_label(&goto) {
                            warn!("AffectionCondition jump target not found: {}", goto);
                        }
                    }
                    Some(ScriptCmd::SavePoint) => {}
                    Some(ScriptCmd::UnlockCg { file }) => {
                        unlock_state.cg_unlocked.insert(file);
                    }
                    Some(ScriptCmd::SetBg { file, .. }) => {
                        set_bg_writer.write(SetBgMessage { file, transition: None, duration: None });
                    }
                    Some(ScriptCmd::ShowFg { char_id, expression, position, .. }) => {
                        show_fg_writer.write(ShowFgMessage { char_id, expression, position, transition: None, duration: None });
                    }
                    Some(ScriptCmd::HideFg { char_id, .. }) => {
                        hide_fg_writer.write(HideFgMessage { char_id, transition: None, duration: None });
                    }
                    Some(ScriptCmd::ShowFace { char_id, .. }) => {
                        show_face_writer.write(ShowFaceMessage { char_id });
                    }
                    Some(ScriptCmd::HideFace { .. }) => {
                        hide_face_writer.write(HideFaceMessage);
                    }
                    Some(ScriptCmd::ShowCg { file, .. }) => {
                        show_cg_writer.write(ShowCgMessage { file: file.clone(), transition: None, duration: None });
                        unlock_state.cg_unlocked.insert(file);
                    }
                    Some(ScriptCmd::HideCg { .. }) => {
                        hide_cg_writer.write(HideCgMessage { transition: None, duration: None });
                    }
                    Some(ScriptCmd::DrawSprite { id, file, x, y, z, alpha, priority, time, rotation, anchor_x, anchor_y, blend_mode }) => {
                        draw_sprite_writer.write(DrawSpriteMessage { id, file, x, y, z, alpha, priority, time, rotation, anchor_x, anchor_y, blend_mode });
                    }
                    Some(ScriptCmd::FadeSprite { id, time }) => {
                        fade_sprite_writer.write(FadeSpriteMessage { id, time });
                    }
                    Some(ScriptCmd::MoveSprite { id, x, y, z, alpha, time, wait }) => {
                        move_sprite_writer.write(MoveSpriteMessage { id, x, y, z, alpha, time, wait });
                    }
                    Some(ScriptCmd::PlayBgm { id, volume, fade_in }) => {
                        if !intro.0 {
                            play_bgm_writer.write(PlayBgmMessage { id, volume, fade_in });
                        }
                    }
                    Some(ScriptCmd::StopBgm { id, fade_out }) => {
                        if !intro.0 {
                            stop_bgm_writer.write(StopBgmMessage { id, fade_out });
                        }
                    }
                    Some(ScriptCmd::BgmVol { channel: _, volume }) => {
                        let vol = match volume.as_str() {
                            "MIN" => 0.0,
                            "LOW" => 30.0 / 128.0,
                            "NORM" => 80.0 / 128.0,
                            _ => 1.0,
                        };
                        commands.queue(move |world: &mut World| {
                            let mut settings = world.resource_mut::<Settings>();
                            settings.bgm_volume = vol;
                        });
                    }
                    Some(ScriptCmd::Flash { color, time, alpha }) => {
                        for (entity, mut bg, mut vis) in overlay_query.iter_mut() {
                            let base = match color {
                                OverlayColor::Black => Color::srgba(0.0, 0.0, 0.0, 1.0),
                                OverlayColor::White => Color::srgba(1.0, 1.0, 1.0, 1.0),
                            };
                            *bg = BackgroundColor(base);
                            *vis = Visibility::Visible;
                            let start = alpha as f32 / 255.0;
                            commands.entity(entity).insert(OverlayTween {
                                timer: Timer::from_seconds(time as f32 / 1000.0, TimerMode::Once),
                                start_alpha: start,
                                end_alpha: 0.0,
                            });
                        }
                    }
                    Some(ScriptCmd::PlaySe { file, volume }) => {
                        play_se_writer.write(PlaySeMessage { file, volume });
                    }
                    Some(ScriptCmd::LoopSe { file, volume, channel }) => {
                        loop_se_writer.write(LoopSeMessage { file, volume, channel });
                    }
                    Some(ScriptCmd::StopStreamingSe { channel }) => {
                        stop_streaming_se_writer.write(StopStreamingSeMessage { channel });
                    }
                    Some(ScriptCmd::PlayVoice { file }) => {
                        pending_voice = Some(file.clone());
                        play_voice_writer.write(PlayVoiceMessage { file, volume: None });
                    }
                    Some(ScriptCmd::Wait { .. }) => {}
                    Some(ScriptCmd::ScreenOverlay { color, .. }) => {
                        for (_, mut bg, mut vis) in overlay_query.iter_mut() {
                            let base = match color {
                                OverlayColor::Black => Color::srgba(0.0, 0.0, 0.0, 1.0),
                                OverlayColor::White => Color::srgba(1.0, 1.0, 1.0, 1.0),
                            };
                            *bg = BackgroundColor(base);
                            *vis = Visibility::Visible;
                        }
                    }
                    Some(ScriptCmd::ClearOverlay { .. }) => {
                        for (entity, _, mut vis) in overlay_query.iter_mut() {
                            *vis = Visibility::Hidden;
                            commands.entity(entity).remove::<OverlayTween>();
                        }
                    }
                    Some(ScriptCmd::Window { show, .. }) => {
                        for mut vis in window_query.iter_mut() {
                            *vis = if show { Visibility::Visible } else { Visibility::Hidden };
                        }
                        window_override.0 = !show;
                    }
                    Some(ScriptCmd::ChangeWindowColor { color_idx }) => {
                        commands.queue(move |world: &mut World| {
                            let mut settings = world.resource_mut::<Settings>();
                            settings.window_color_idx = color_idx;
                        });
                    }
                    Some(ScriptCmd::ChangeWindowDesign { design }) => {
                        commands.queue(move |world: &mut World| {
                            let mut settings = world.resource_mut::<Settings>();
                            settings.window_design = design;
                        });
                    }
                    Some(cmd) => {
                        info!("Script cmd (no-op): {:?}", cmd);
                    }
                    None => break,
                }
            }
            if !engine.has_more() && !dialogue.is_displaying {
                info!("Script finished: {}", engine.current_script);
            }
            continue;
        }

        // Normal mode
        let mut pending_voice = None;
        while engine.has_more() {
            let cmd = engine.advance().cloned();
            match cmd {
                Some(ScriptCmd::Dialogue { speaker, text }) => {
                    for (_, entity) in overlay_mgr.sprites.drain() {
                        commands.entity(entity).despawn();
                    }
                    engine.dialogue_idx += 1;
                    if intro.0 && speaker.is_some() {
                        intro.0 = false;
                    }
                    let text_clone = text.clone();
                    backlog.entries.push(BacklogEntry {
                        speaker: speaker.clone(),
                        text: text_clone,
                        voice_file: pending_voice.take(),
                    });
                    if backlog.entries.len() > 200 {
                        backlog.entries.remove(0);
                    }
                    window_override.0 = false;
                    dialogue.current_speaker = speaker;
                    dialogue.current_text = text;
                    dialogue.text_progress = 0;
                    dialogue.is_displaying = true;
                    break;
                }
                Some(ScriptCmd::ClearText) => {
                    dialogue.current_text.clear();
                    dialogue.current_speaker = None;
                    dialogue.text_progress = 0;
                    dialogue.is_displaying = false;
                }
                Some(ScriptCmd::Jump { target }) => {
                    if !engine.jump_to_label(&target) {
                        warn!("Jump target not found: {}", target);
                    }
                }
                Some(ScriptCmd::Call { target }) => {
                    engine.call_label(&target);
                }
                Some(ScriptCmd::CallScript { script, label }) => {
                    engine.call_script(&script, label.as_deref());
                }
                Some(ScriptCmd::Return) => {
                    engine.return_from_call();
                }
                Some(ScriptCmd::Condition { var, value, operator, goto }) => {
                    let flag_val = engine.flags.get(&var).copied().unwrap_or(0);
                    let met = match operator {
                        ConditionOp::Greater => flag_val > value,
                        ConditionOp::Less => flag_val < value,
                        ConditionOp::Equal => flag_val == value,
                        ConditionOp::GreaterEqual => flag_val >= value,
                        ConditionOp::LessEqual => flag_val <= value,
                    };
                    if met && !engine.jump_to_label(&goto) {
                        warn!("Condition jump target not found: {}", goto);
                    }
                }
                Some(ScriptCmd::SetFlag { name, value }) => {
                    engine.flags.insert(name, value);
                }
                Some(ScriptCmd::Halt) => {
                    engine.call_stack.clear();
                    engine.current_script.clear();
                    engine.current_line = 0;
                }
                Some(ScriptCmd::AffectionChange { char_id, delta }) => {
                    *affection.0.entry(char_id).or_insert(0) += delta;
                }
                Some(ScriptCmd::AffectionCondition { char_id, value, operator, goto }) => {
                    let affection_val = affection.0.get(&char_id).copied().unwrap_or(0);
                    let met = match operator {
                        ConditionOp::Greater => affection_val > value,
                        ConditionOp::Less => affection_val < value,
                        ConditionOp::Equal => affection_val == value,
                        ConditionOp::GreaterEqual => affection_val >= value,
                        ConditionOp::LessEqual => affection_val <= value,
                    };
                    if met && !engine.jump_to_label(&goto) {
                        warn!("AffectionCondition jump target not found: {}", goto);
                    }
                }
                Some(ScriptCmd::SavePoint) => {}
                Some(ScriptCmd::UnlockCg { file }) => {
                    unlock_state.cg_unlocked.insert(file);
                }
                Some(ScriptCmd::SetBg { file, transition, duration }) => {
                    set_bg_writer.write(SetBgMessage { file, transition, duration: duration.map(|d| d as f64) });
                }
                Some(ScriptCmd::ShowFg { char_id, expression, position, transition }) => {
                    show_fg_writer.write(ShowFgMessage { char_id, expression, position, transition, duration: None });
                }
                Some(ScriptCmd::HideFg { char_id, transition }) => {
                    hide_fg_writer.write(HideFgMessage { char_id, transition, duration: None });
                }
                Some(ScriptCmd::ShowFace { char_id, .. }) => {
                    show_face_writer.write(ShowFaceMessage { char_id });
                }
                Some(ScriptCmd::HideFace { .. }) => {
                    hide_face_writer.write(HideFaceMessage);
                }
                Some(ScriptCmd::ShowCg { file, transition }) => {
                    show_cg_writer.write(ShowCgMessage { file: file.clone(), transition, duration: None });
                    unlock_state.cg_unlocked.insert(file);
                }
                Some(ScriptCmd::HideCg { transition }) => {
                    hide_cg_writer.write(HideCgMessage { transition, duration: None });
                }
                Some(ScriptCmd::DrawSprite { id, file, x, y, z, alpha, priority, time, rotation, anchor_x, anchor_y, blend_mode }) => {
                    if file.contains("_tx") {
                    }
                    draw_sprite_writer.write(DrawSpriteMessage { id, file, x, y, z, alpha, priority, time, rotation, anchor_x, anchor_y, blend_mode });
                }
                Some(ScriptCmd::FadeSprite { id, time }) => {
                    fade_sprite_writer.write(FadeSpriteMessage { id, time });
                }
                Some(ScriptCmd::MoveSprite { id, x, y, z, alpha, time, wait }) => {
                    move_sprite_writer.write(MoveSpriteMessage { id, x, y, z, alpha, time, wait });
                }
                Some(ScriptCmd::PlayBgm { id, volume, fade_in }) => {
                    if !intro.0 {
                        play_bgm_writer.write(PlayBgmMessage { id, volume, fade_in });
                    }
                }
                Some(ScriptCmd::StopBgm { id, fade_out }) => {
                    if !intro.0 {
                        stop_bgm_writer.write(StopBgmMessage { id, fade_out });
                    }
                }
                Some(ScriptCmd::PlaySe { file, volume }) => {
                    play_se_writer.write(PlaySeMessage { file, volume });
                }
                Some(ScriptCmd::LoopSe { file, volume, channel }) => {
                    loop_se_writer.write(LoopSeMessage { file, volume, channel });
                }
                Some(ScriptCmd::StopStreamingSe { channel }) => {
                    stop_streaming_se_writer.write(StopStreamingSeMessage { channel });
                }
                Some(ScriptCmd::PlayVoice { file }) => {
                    pending_voice = Some(file.clone());
                    play_voice_writer.write(PlayVoiceMessage { file, volume: None });
                }
                Some(ScriptCmd::BgmVol { channel: _, volume }) => {
                    let vol = match volume.as_str() {
                        "MIN" => 0.0,
                        "LOW" => 30.0 / 128.0,
                        "NORM" => 80.0 / 128.0,
                        _ => 1.0,
                    };
                    commands.queue(move |world: &mut World| {
                        let mut settings = world.resource_mut::<Settings>();
                        settings.bgm_volume = vol;
                    });
                }
                Some(ScriptCmd::Flash { color, time, alpha }) => {
                    for (entity, mut bg, mut vis) in overlay_query.iter_mut() {
                        let base = match color {
                            OverlayColor::Black => Color::srgba(0.0, 0.0, 0.0, 1.0),
                            OverlayColor::White => Color::srgba(1.0, 1.0, 1.0, 1.0),
                        };
                        *bg = BackgroundColor(base);
                        *vis = Visibility::Visible;
                        let start = alpha as f32 / 255.0;
                        commands.entity(entity).insert(OverlayTween {
                            timer: Timer::from_seconds(time as f32 / 1000.0, TimerMode::Once),
                            start_alpha: start,
                            end_alpha: 0.0,
                        });
                    }
                }
                Some(ScriptCmd::Quake { power, time }) => {
                    commands.insert_resource(QuakeState {
                        timer: Some(Timer::from_seconds(time as f32 / 1000.0, TimerMode::Once)),
                        intensity: power,
                    });
                }
                Some(ScriptCmd::Choice { options }) => {
                    choice_state.active = true;
                    choice_state.options = options;
                    break;
                }
                Some(ScriptCmd::Wait { duration }) => {
                    if settings.skip_mode {
                        // skip mode: continue without waiting
                    } else {
                        auto_skip.auto_timer = Some(Timer::from_seconds(duration as f32 / 1000.0, TimerMode::Once));
                        break;
                    }
                }
                Some(ScriptCmd::ScreenOverlay { color, time }) => {
                    for (entity, mut bg, mut vis) in overlay_query.iter_mut() {
                        let base = match color {
                            OverlayColor::Black => Color::srgba(0.0, 0.0, 0.0, 0.0),
                            OverlayColor::White => Color::srgba(1.0, 1.0, 1.0, 0.0),
                        };
                        *bg = BackgroundColor(base);
                        *vis = Visibility::Visible;
                        commands.entity(entity).insert(OverlayTween {
                            timer: Timer::from_seconds(time as f32 / 1000.0, TimerMode::Once),
                            start_alpha: 0.0,
                            end_alpha: 1.0,
                        });
                    }
                }
                Some(ScriptCmd::ClearOverlay { time }) => {
                    for (entity, bg, mut vis) in overlay_query.iter_mut() {
                        if time > 0 {
                            let current_alpha = bg.0.alpha();
                            commands.entity(entity).insert(OverlayTween {
                                timer: Timer::from_seconds(time as f32 / 1000.0, TimerMode::Once),
                                start_alpha: current_alpha,
                                end_alpha: 0.0,
                            });
                        } else {
                            *vis = Visibility::Hidden;
                            commands.entity(entity).remove::<OverlayTween>();
                        }
                    }
                }
                Some(ScriptCmd::Window { show, .. }) => {
                    for mut vis in window_query.iter_mut() {
                        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
                    }
                    window_override.0 = !show;
                }
                Some(ScriptCmd::ChangeWindowColor { color_idx }) => {
                    commands.queue(move |world: &mut World| {
                        let mut settings = world.resource_mut::<Settings>();
                        settings.window_color_idx = color_idx;
                    });
                }
                Some(ScriptCmd::ChangeWindowDesign { design }) => {
                    commands.queue(move |world: &mut World| {
                        let mut settings = world.resource_mut::<Settings>();
                        settings.window_design = design;
                    });
                }
                Some(cmd) => {
                    info!("Script cmd (no-op): {:?}", cmd);
                }
                None => break,
            }
        }

        if !engine.has_more() && !dialogue.is_displaying {
            info!("Script finished: {}", engine.current_script);
        }
    }
}

fn update_text_reveal(
    time: Res<Time>,
    mut dialogue: ResMut<DialogueState>,
    settings: Res<Settings>,
) {
    if dialogue.is_displaying && dialogue.text_progress < dialogue.current_text.len() {
        let chars_per_sec = (settings.text_speed as f64).max(1.0);
        let increment = (time.delta_secs_f64() * chars_per_sec) as usize;
        dialogue.text_progress = (dialogue.text_progress + increment).min(dialogue.current_text.len());
    }
}

fn handle_auto_skip(
    time: Res<Time>,
    mut advance_ev: MessageWriter<AdvanceEvent>,
    mut auto_skip: ResMut<AutoSkipTimer>,
    dialogue: Res<DialogueState>,
    choice_state: Res<ChoiceState>,
    settings: Res<Settings>,
) {
    if choice_state.active {
        auto_skip.auto_timer = None;
        auto_skip.skip_timer = None;
        return;
    }

    let text_fully_displayed = !dialogue.is_displaying
        || dialogue.text_progress >= dialogue.current_text.len();

    if settings.auto_mode && !settings.skip_mode {
        if !text_fully_displayed || dialogue.current_text.is_empty() {
            auto_skip.auto_timer = None;
            auto_skip.skip_timer = None;
            return;
        }
        let timer = auto_skip.auto_timer.get_or_insert_with(|| {
            Timer::from_seconds(2.0, TimerMode::Once)
        });
        timer.tick(time.delta());
        if timer.just_finished() {
            advance_ev.write(AdvanceEvent);
            auto_skip.auto_timer = None;
        }
    } else if !settings.skip_mode {
        if let Some(timer) = &mut auto_skip.auto_timer {
            timer.tick(time.delta());
            if timer.just_finished() {
                advance_ev.write(AdvanceEvent);
                auto_skip.auto_timer = None;
            }
        }
    }

    if settings.skip_mode {
        if !text_fully_displayed || dialogue.current_text.is_empty() {
            auto_skip.auto_timer = None;
            auto_skip.skip_timer = None;
            return;
        }
        let timer = auto_skip.skip_timer.get_or_insert_with(|| {
            Timer::from_seconds(0.5, TimerMode::Once)
        });
        timer.tick(time.delta());
        if timer.just_finished() {
            auto_skip.skip_timer = None;
            advance_ev.write(AdvanceEvent);
        }
    }
}
