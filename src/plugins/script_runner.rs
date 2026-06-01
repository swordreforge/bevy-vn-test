use crate::audio_messages::{
    LoopSeMessage, PlayBgmMessage, PlayBgmXMessage, PlaySeMessage, PlayVoiceMessage,
    StopBgmMessage, StopBgmXMessage, StopStreamingSeMessage,
};
use crate::choice_messages::ChoiceSelectedMessage;
use crate::components::{DialogueUiRoot, OverlayTween, ScreenOverlayRoot};
use crate::plugins::event_system::view_data;
use crate::plugins::event_system::{ViewPhase, ViewState};
use crate::plugins::inputs::{AdvanceEvent, AdvanceSource};
use crate::rendering_messages::{
    AnimateSpriteMessage, DrawSpriteMessage, FadeSpriteMessage, HideCgMessage, HideFaceMessage,
    HideFgMessage, MoveSpriteMessage, ScrollBgMessage, SetBgMessage, ShowCgMessage,
    ShowFaceMessage, ShowFgMessage,
};
use crate::plugins::video::{spawn_sprite_video, spawn_video, start_rain_video};
use crate::resources::{
    save_unlock_state, sync_affection_from_work, map_video_file, AffectionMap, Backlog,
    BacklogEntry, ChoiceState, CompletedRoute, DialogueState, GameRestrictions, IntroPhase,
    PendingSpriteVideoBlock, PendingVideo, QuakeState, RainOverlayState, RouteConfig, Settings,
    SpriteOverlayManager, SpriteVideoManager, UnlockState,
};
use crate::resources::{SelectedRoute, ViewBlocking, WindowOverride};
use crate::script::{
    evaluate_condition_expression, evaluate_script_expression, ConditionOp, OverlayColor, ScriptCmd,
    ScriptEngine,
};
use crate::state::AppState;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub struct ScriptRunnerPlugin;

#[derive(Resource)]
pub struct AutoSkipTimer {
    pub auto_timer: Option<Timer>,
    pub skip_timer: Option<Timer>,
}

impl Default for AutoSkipTimer {
    fn default() -> Self {
        Self {
            auto_timer: None,
            skip_timer: None,
        }
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
    config: Res<'w, RouteConfig>,
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
    play_bgmx_writer: MessageWriter<'w, PlayBgmXMessage>,
    stop_bgmx_writer: MessageWriter<'w, StopBgmXMessage>,
    play_se_writer: MessageWriter<'w, PlaySeMessage>,
    loop_se_writer: MessageWriter<'w, LoopSeMessage>,
    stop_streaming_se_writer: MessageWriter<'w, StopStreamingSeMessage>,
    play_voice_writer: MessageWriter<'w, PlayVoiceMessage>,
    scroll_bg_writer: MessageWriter<'w, ScrollBgMessage>,
    animate_sprite_writer: MessageWriter<'w, AnimateSpriteMessage>,
    settings: Res<'w, Settings>,
    auto_skip: ResMut<'w, AutoSkipTimer>,
    intro: ResMut<'w, IntroPhase>,
    overlay_mgr: ResMut<'w, SpriteOverlayManager>,
    restrictions: ResMut<'w, GameRestrictions>,
    pending_video: ResMut<'w, PendingVideo>,
    sprite_video_mgr: ResMut<'w, SpriteVideoManager>,
    rain_state: ResMut<'w, RainOverlayState>,
    blocked_sprite: ResMut<'w, PendingSpriteVideoBlock>,
    images: ResMut<'w, Assets<Image>>,
}

impl Plugin for ScriptRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoSkipTimer>()
            .init_resource::<IntroPhase>()
            .init_resource::<PendingVideo>()
            .init_resource::<WindowOverride>()
            .add_systems(
                OnEnter(AppState::Gameplay),
                (start_script_execution, start_intro_bgm),
            )
            .add_systems(OnEnter(AppState::Title), reset_engine_on_title)
            .add_systems(OnExit(AppState::Gameplay), persist_gameplay)
            .add_systems(
                Update,
                (handle_auto_skip, process_advance, update_text_reveal)
                    .chain()
                    .run_if(in_state(AppState::Gameplay)),
            );
    }
}

fn start_script_execution(
    mut dialogue: ResMut<DialogueState>,
    mut engine: ResMut<ScriptEngine>,
    mut selected_route: ResMut<SelectedRoute>,
    mut advance_ev: MessageWriter<AdvanceEvent>,
) {
    dialogue.current_text.clear();
    dialogue.current_speaker = None;
    dialogue.text_progress = 0;
    dialogue.is_displaying = false;

    if let Some(script) = selected_route.0.take() {
        engine.flags.clear();
        engine.global_flags.clear();
        engine.local_work.clear();
        engine.local_flags.clear();
        engine.dialogue_idx = 0;
        engine.finished = false;
        engine.call_stack.clear();
        engine.current_script = script;
        engine.current_line = 0;
        info!("Starting route script: {}", engine.current_script);
    }

    // Auto-start: emit the first AdvanceEvent so script execution begins
    // immediately on entering Gameplay, matching the original game behavior.
    advance_ev.write(AdvanceEvent {
        source: AdvanceSource::Auto,
    });
}

fn start_intro_bgm(
    engine: Res<ScriptEngine>,
    mut play_bgm: MessageWriter<PlayBgmMessage>,
    mut intro: ResMut<IntroPhase>,
) {
    if engine.current_line != 0 {
        return;
    }
    let is_start = engine.current_script == "main" || engine.current_script == "aiy00010";
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
    engine.global_flags.clear();
    engine.local_work.clear();
    engine.local_flags.clear();
    engine.dialogue_idx = 0;
    engine.finished = false;
    if engine.scripts.contains_key("main") {
        engine.current_script = "main".to_string();
    } else if engine.scripts.contains_key("aiy00010") {
        engine.current_script = "aiy00010".to_string();
    }
}

fn process_advance(
    mut params: ProcessAdvanceParams<'_, '_>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut completed_route: ResMut<CompletedRoute>,
    mut overlay_query: Query<
        (Entity, &mut BackgroundColor, &mut Visibility),
        With<ScreenOverlayRoot>,
    >,
    mut window_query: Query<&mut Visibility, (With<DialogueUiRoot>, Without<ScreenOverlayRoot>)>,
    mut window_override: ResMut<WindowOverride>,
    view_blocking: Res<ViewBlocking>,
) {
    let ProcessAdvanceParams {
        ref mut advance_ev,
        ref mut engine,
        ref mut dialogue,
        ref mut affection,
        ref mut backlog,
        ref mut unlock_state,
        ref mut config,
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
        ref mut play_bgmx_writer,
        ref mut stop_bgmx_writer,
        ref mut play_se_writer,
        ref mut loop_se_writer,
        ref mut stop_streaming_se_writer,
        ref mut play_voice_writer,
        ref mut scroll_bg_writer,
        ref mut animate_sprite_writer,
        ref mut settings,
        ref mut auto_skip,
        ref mut intro,
        ref mut overlay_mgr,
        ref mut restrictions,
        ref mut pending_video,
        ref mut sprite_video_mgr,
        ref mut rain_state,
        ref mut blocked_sprite,
        ref mut images,
    } = &mut params;

    for ev in advance_ev.read() {
        if ev.source == AdvanceSource::UserInput {
            auto_skip.auto_timer = None;
            auto_skip.skip_timer = None;
        }

        // If View is active, block script execution
        if view_blocking.0 {
            continue;
        }

        // If video is playing, block script execution
        if pending_video.playing {
            continue;
        }

        // If blocked on sprite video, block script execution
        if blocked_sprite.0.is_some() {
            continue;
        }

        // If previous script finished, reset dialogue for next script
        if engine.finished && engine.has_more() {
            engine.finished = false;
            dialogue.current_text.clear();
            dialogue.current_speaker = None;
            dialogue.text_progress = 0;
            dialogue.is_displaying = false;
        }

        // If script ended and no next script, wait for state transition
        if engine.finished {
            continue;
        }

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
                        clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                        engine.finished = false;
                        if !engine.jump_to_label(&target) {
                            warn!("Jump target not found: {}", target);
                        }
                    }
                    Some(ScriptCmd::Call { target }) => {
                        clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                        engine.finished = false;
                        engine.call_label(&target);
                    }
                    Some(ScriptCmd::CallScript { script, label }) => {
                        clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                        engine.finished = false;
                        engine.call_script(&script, label.as_deref());
                    }
                    Some(ScriptCmd::Return) => {
                        engine.finished = false;
                        engine.return_from_call();
                    }
                    Some(ScriptCmd::Condition {
                        var,
                        value,
                        operator,
                        goto,
                    }) => {
                        let flag_val = engine.flags.get(&var).copied().unwrap_or(0);
                        let met = match operator {
                            ConditionOp::Greater => flag_val > value,
                            ConditionOp::Less => flag_val < value,
                            ConditionOp::Equal => flag_val == value,
                            ConditionOp::NotEqual => flag_val != value,
                            ConditionOp::GreaterEqual => flag_val >= value,
                            ConditionOp::LessEqual => flag_val <= value,
                        };
                        if met {
                            clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                            if !engine.jump_to_label(&goto) {
                                warn!("Condition jump target not found: {}", goto);
                            }
                        }
                    }
                    Some(ScriptCmd::SetFlag { name, value }) => {
                        engine.flags.insert(name, value);
                    }
                    Some(ScriptCmd::SetLocalFlag { index, value }) => {
                        engine.local_flags.insert(index, value);
                    }
                    Some(ScriptCmd::StoreValueToLocalWork { index, value, expression }) => {
                        let final_val = if let Some(ref expr) = expression {
                            evaluate_script_expression(expr, &engine.flags)
                        } else {
                            value
                        };
                        engine.local_work.insert(index, final_val);
                        sync_affection_from_work(index, final_val, &mut *affection);
                    }
                    Some(ScriptCmd::LoadValueFromLocalWork { index }) => {
                        let val = engine.local_work.get(&index).copied().unwrap_or(0);
                        engine.flags.insert("tmp".to_string(), val);
                    }
                    Some(ScriptCmd::GetLocalFlag { index }) => {
                        let val = engine.local_flags.get(&index).copied().unwrap_or(0);
                        engine.flags.insert("tmp".to_string(), val);
                    }
                    Some(ScriptCmd::GetGlobalFlag { index }) => {
                        let val = engine.global_flags.get(&index).copied().unwrap_or(0);
                        engine.flags.insert("tmp".to_string(), val);
                    }
                    Some(ScriptCmd::Exif { expression }) => {
                        // In skip mode, just advance past the next command
                        if !evaluate_condition_expression(&expression, &engine.flags) {
                            let _ = engine.advance();
                        }
                    }
                    Some(ScriptCmd::Halt) => {
                        if let Some(name) = engine.detect_route_completion(config) {
                            unlock_state.mark_route_cleared(&name);
                            completed_route.0 = Some(name);
                            next_state.set(AppState::RouteEnd);
                        }
                        engine.call_stack.clear();
                        engine.current_script.clear();
                        engine.current_line = 0;
                        engine.finished = true;
                    }
                    Some(ScriptCmd::PlayMovie { file }) => {
                        info!("Video skipped: {}", file);
                    }
                    Some(ScriptCmd::AffectionChange { char_id, delta }) => {
                        *affection.0.entry(char_id).or_insert(0) += delta;
                    }
                    Some(ScriptCmd::AffectionCondition {
                        char_id,
                        value,
                        operator,
                        goto,
                    }) => {
                        let affection_val = affection.0.get(&char_id).copied().unwrap_or(0);
                        let met = match operator {
                            ConditionOp::Greater => affection_val > value,
                            ConditionOp::Less => affection_val < value,
                            ConditionOp::Equal => affection_val == value,
                            ConditionOp::NotEqual => affection_val != value,
                            ConditionOp::GreaterEqual => affection_val >= value,
                            ConditionOp::LessEqual => affection_val <= value,
                        };
                        if met {
                            clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                            if !engine.jump_to_label(&goto) {
                                warn!("AffectionCondition jump target not found: {}", goto);
                            }
                        }
                    }
                    Some(ScriptCmd::SavePoint) => {}
                    Some(ScriptCmd::UnlockCg { file }) => {
                        unlock_state.cg_unlocked.insert(file);
                    }
                    Some(ScriptCmd::SetBg { file, .. }) => {
                        set_bg_writer.write(SetBgMessage {
                            file,
                            transition: None,
                            duration: None,
                        });
                    }
                    Some(ScriptCmd::ShowFg {
                        char_id,
                        expression,
                        position,
                        ..
                    }) => {
                        show_fg_writer.write(ShowFgMessage {
                            char_id,
                            expression,
                            position,
                            transition: None,
                            duration: None,
                        });
                    }
                    Some(ScriptCmd::HideFg { char_id, .. }) => {
                        hide_fg_writer.write(HideFgMessage {
                            char_id,
                            transition: None,
                            duration: None,
                        });
                    }
                    Some(ScriptCmd::ShowFace { char_id, .. }) => {
                        show_face_writer.write(ShowFaceMessage { char_id });
                    }
                    Some(ScriptCmd::HideFace { .. }) => {
                        hide_face_writer.write(HideFaceMessage);
                    }
                    Some(ScriptCmd::ShowCg { file, .. }) => {
                        show_cg_writer.write(ShowCgMessage {
                            file: file.clone(),
                            transition: None,
                            duration: None,
                        });
                        unlock_state.cg_unlocked.insert(file);
                    }
                    Some(ScriptCmd::HideCg { .. }) => {
                        hide_cg_writer.write(HideCgMessage {
                            transition: None,
                            duration: None,
                        });
                    }
                    Some(ScriptCmd::DrawSprite {
                        id,
                        file,
                        x,
                        y,
                        z,
                        alpha,
                        priority,
                        time,
                        rotation,
                        anchor_x,
                        anchor_y,
                        blend_mode,
                    }) => {
                        draw_sprite_writer.write(DrawSpriteMessage {
                            id,
                            file,
                            x,
                            y,
                            z,
                            alpha,
                            priority,
                            time,
                            rotation,
                            anchor_x,
                            anchor_y,
                            blend_mode,
                        });
                    }
                    Some(ScriptCmd::FadeSprite { id, time }) => {
                        fade_sprite_writer.write(FadeSpriteMessage { id, time });
                    }
                    Some(ScriptCmd::MoveSprite {
                        id,
                        x,
                        y,
                        z,
                        alpha,
                        time,
                        wait,
                    }) => {
                        move_sprite_writer.write(MoveSpriteMessage {
                            id,
                            x,
                            y,
                            z,
                            alpha,
                            time,
                            wait,
                        });
                    }
                    Some(ScriptCmd::PlayBgm {
                        id,
                        volume,
                        fade_in,
                    }) => {
                        if !intro.0 {
                            play_bgm_writer.write(PlayBgmMessage {
                                id,
                                volume,
                                fade_in,
                            });
                        }
                    }
                    Some(ScriptCmd::StopBgm { id, fade_out }) => {
                        if !intro.0 {
                            stop_bgm_writer.write(StopBgmMessage { id, fade_out });
                        }
                    }
                    Some(ScriptCmd::PlayBgmX {
                        id,
                        volume,
                        fade_in,
                    }) => {
                        if !intro.0 {
                            play_bgmx_writer.write(PlayBgmXMessage {
                                id,
                                volume,
                                fade_in,
                            });
                        }
                    }
                    Some(ScriptCmd::StopBgmX { id, fade_out }) => {
                        if !intro.0 {
                            stop_bgmx_writer.write(StopBgmXMessage { id, fade_out });
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
                    Some(ScriptCmd::LoopSe {
                        file,
                        volume,
                        channel,
                    }) => {
                        loop_se_writer.write(LoopSeMessage {
                            file,
                            volume,
                            channel,
                        });
                    }
                    Some(ScriptCmd::StopStreamingSe { channel }) => {
                        stop_streaming_se_writer.write(StopStreamingSeMessage { channel });
                    }
                    Some(ScriptCmd::PlayVoice { file }) => {
                        pending_voice = Some(file.clone());
                        play_voice_writer.write(PlayVoiceMessage { file, volume: None });
                    }
                    Some(ScriptCmd::ScrollBg {
                        file,
                        x1,
                        y1,
                        x2,
                        y2,
                        ..
                    }) => {
                        scroll_bg_writer.write(ScrollBgMessage {
                            file,
                            x1,
                            y1,
                            x2,
                            y2,
                            fade: 0,
                            wait: false,
                        });
                    }
                    Some(ScriptCmd::AnimateSprite {
                        id,
                        file,
                        max,
                        frame_time,
                        style,
                        x,
                        y,
                        z,
                        anchor_x,
                        anchor_y,
                        rotation,
                        draw,
                        alpha,
                        priority,
                        ..
                    }) => {
                        animate_sprite_writer.write(AnimateSpriteMessage {
                            id,
                            file,
                            max,
                            frame_time,
                            style,
                            x,
                            y,
                            z,
                            anchor_x,
                            anchor_y,
                            rotation,
                            draw,
                            alpha,
                            priority,
                            wait: false,
                        });
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
                            *vis = if show {
                                Visibility::Visible
                            } else {
                                Visibility::Hidden
                            };
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
                    Some(ScriptCmd::View { ref char_id }) => {
                        if let Some(entry) = view_data::lookup_view_entry(char_id) {
                            commands.queue(move |world: &mut World| {
                                let mut settings = world.resource_mut::<Settings>();
                                settings.window_color_idx = entry.window_color as i32;
                            });
                        }
                    }
                    Some(ScriptCmd::SetGlobalFlag { index, value }) => {
                        engine.global_flags.insert(index, value);
                    }
                    Some(ScriptCmd::RouteFlag) => {
                        let count = config.route_unlock_flags.iter()
                            .filter(|&f| engine.global_flags.get(f).copied().unwrap_or(0) >= 1)
                            .count();
                        if count == config.route_unlock_flags.len() {
                            engine.global_flags.insert(config.all_routes_cleared_flag, 1);
                        }
                        if engine.global_flags.get(&config.full_completion_flag) != Some(&1) {
                            let all_clear = (config.ending_flag_range.0..=config.ending_flag_range.1)
                                .chain(std::iter::once(config.all_routes_cleared_flag))
                                .all(|f| engine.global_flags.get(&f).copied().unwrap_or(0) >= 1);
                            if all_clear {
                                engine.global_flags.insert(config.full_completion_flag, 1);
                            }
                        }
                    }
                    Some(ScriptCmd::GameMode { mode }) => {
                        commands.queue(move |world: &mut World| {
                            let mut settings = world.resource_mut::<Settings>();
                            settings.click_to_advance = mode == 2;
                        });
                    }
                    Some(ScriptCmd::SetValidity { mode, allowed }) => {
                        match mode {
                            crate::script::ValidityMode::Saving => restrictions.saving = allowed,
                            crate::script::ValidityMode::Loading => restrictions.loading = allowed,
                            crate::script::ValidityMode::Input => restrictions.input = allowed,
                        }
                    }
                    Some(ScriptCmd::MovieInit) => {}
                    Some(ScriptCmd::DrawSpriteEx { .. }) => {}
                    Some(ScriptCmd::WaitToFinishMoviePlayingOnSprite { .. }) => {}
                    Some(ScriptCmd::RainMja { .. }) => {}
                    Some(ScriptCmd::SetRainValid { .. }) => {}
                    Some(ScriptCmd::SetRainQuantity { .. }) => {}
                    Some(ScriptCmd::SetRainColor { .. }) => {}
                    Some(ScriptCmd::SetRainVector { .. }) => {}
                    Some(ScriptCmd::SetRainCameraAngle { .. }) => {}
                    Some(ScriptCmd::SetRainPriority { .. }) => {}
                    Some(ScriptCmd::StopAllSe) => {}
                    Some(ScriptCmd::PushHistory) => {}
                    Some(ScriptCmd::WaitVoice) => {}
                    Some(ScriptCmd::QueryMode { .. }) => {
                        engine.flags.insert("tmp".to_string(), 0);
                    }
                    Some(ScriptCmd::StreamingSeVol { .. }) => {}
                    Some(ScriptCmd::Blur { .. })
                    | Some(ScriptCmd::ShakeScreen { .. })
                    | Some(ScriptCmd::ShakeSprite { .. })
                    | Some(ScriptCmd::MonologueColor { .. })
                    | Some(ScriptCmd::Tween { .. })
                    | Some(ScriptCmd::FadeScene { .. })
                    | Some(ScriptCmd::NoOp { .. }) => {}
                    Some(cmd) => {
                        info!("Script cmd (no-op): {:?}", cmd);
                    }
                    None => break,
                }
            }
            if !engine.has_more() && !engine.finished {
                engine.finished = true;
                if !engine.call_stack.is_empty() {
                    engine.return_from_call();
                    engine.finished = false;
                } else if engine.next_script() {
                    info!("Script finished: advancing to {}", engine.current_script);
                } else {
                    info!("Script finished (no next): returning to title");
                    next_state.set(AppState::Title);
                }
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
                    clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                    engine.finished = false;
                    if !engine.jump_to_label(&target) {
                        warn!("Jump target not found: {}", target);
                    }
                }
                Some(ScriptCmd::Call { target }) => {
                    clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                    engine.finished = false;
                    engine.call_label(&target);
                }
                Some(ScriptCmd::CallScript { script, label }) => {
                    clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                    engine.finished = false;
                    engine.call_script(&script, label.as_deref());
                }
                Some(ScriptCmd::Return) => {
                    engine.finished = false;
                    engine.return_from_call();
                }
                Some(ScriptCmd::Condition {
                    var,
                    value,
                    operator,
                    goto,
                }) => {
                    let flag_val = engine.flags.get(&var).copied().unwrap_or(0);
                    let met = match operator {
                        ConditionOp::Greater => flag_val > value,
                        ConditionOp::Less => flag_val < value,
                        ConditionOp::Equal => flag_val == value,
                        ConditionOp::NotEqual => flag_val != value,
                        ConditionOp::GreaterEqual => flag_val >= value,
                        ConditionOp::LessEqual => flag_val <= value,
                    };
                    if met {
                        clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                        if !engine.jump_to_label(&goto) {
                            warn!("Condition jump target not found: {}", goto);
                        }
                    }
                }
                Some(ScriptCmd::SetFlag { name, value }) => {
                    engine.flags.insert(name, value);
                }
                Some(ScriptCmd::SetLocalFlag { index, value }) => {
                    engine.local_flags.insert(index, value);
                }
                Some(ScriptCmd::StoreValueToLocalWork { index, value, expression }) => {
                    let final_val = if let Some(ref expr) = expression {
                        evaluate_script_expression(expr, &engine.flags)
                    } else {
                        value
                    };
                    engine.local_work.insert(index, final_val);
                    sync_affection_from_work(index, final_val, &mut *affection);
                }
                Some(ScriptCmd::LoadValueFromLocalWork { index }) => {
                    let val = engine.local_work.get(&index).copied().unwrap_or(0);
                    engine.flags.insert("tmp".to_string(), val);
                }
                Some(ScriptCmd::GetLocalFlag { index }) => {
                    let val = engine.local_flags.get(&index).copied().unwrap_or(0);
                    engine.flags.insert("tmp".to_string(), val);
                }
                Some(ScriptCmd::GetGlobalFlag { index }) => {
                    let val = engine.global_flags.get(&index).copied().unwrap_or(0);
                    engine.flags.insert("tmp".to_string(), val);
                }
                Some(ScriptCmd::Exif { expression }) => {
                    if !evaluate_condition_expression(&expression, &engine.flags) {
                        let _ = engine.advance();
                    }
                }
                Some(ScriptCmd::Halt) => {
                    if let Some(name) = engine.detect_route_completion(config) {
                        unlock_state.mark_route_cleared(&name);
                        completed_route.0 = Some(name);
                        next_state.set(AppState::RouteEnd);
                    }
                    engine.call_stack.clear();
                    engine.current_script.clear();
                    engine.current_line = 0;
                    engine.finished = true;
                }
                Some(ScriptCmd::AffectionChange { char_id, delta }) => {
                    *affection.0.entry(char_id).or_insert(0) += delta;
                }
                Some(ScriptCmd::AffectionCondition {
                    char_id,
                    value,
                    operator,
                    goto,
                }) => {
                    let affection_val = affection.0.get(&char_id).copied().unwrap_or(0);
                    let met = match operator {
                        ConditionOp::Greater => affection_val > value,
                        ConditionOp::Less => affection_val < value,
                        ConditionOp::Equal => affection_val == value,
                        ConditionOp::NotEqual => affection_val != value,
                        ConditionOp::GreaterEqual => affection_val >= value,
                        ConditionOp::LessEqual => affection_val <= value,
                    };
                    if met {
                        clear_scene_sprites(overlay_mgr, &mut commands, hide_fg_writer, hide_cg_writer, &mut overlay_query);
                        if !engine.jump_to_label(&goto) {
                            warn!("AffectionCondition jump target not found: {}", goto);
                        }
                    }
                }
                Some(ScriptCmd::SavePoint) => {}
                Some(ScriptCmd::UnlockCg { file }) => {
                    unlock_state.cg_unlocked.insert(file);
                }
                Some(ScriptCmd::SetBg {
                    file,
                    transition,
                    duration,
                }) => {
                    set_bg_writer.write(SetBgMessage {
                        file,
                        transition,
                        duration: duration.map(|d| d as f64),
                    });
                }
                Some(ScriptCmd::ShowFg {
                    char_id,
                    expression,
                    position,
                    transition,
                }) => {
                    show_fg_writer.write(ShowFgMessage {
                        char_id,
                        expression,
                        position,
                        transition,
                        duration: None,
                    });
                }
                Some(ScriptCmd::HideFg {
                    char_id,
                    transition,
                }) => {
                    hide_fg_writer.write(HideFgMessage {
                        char_id,
                        transition,
                        duration: None,
                    });
                }
                Some(ScriptCmd::ShowFace { char_id, .. }) => {
                    show_face_writer.write(ShowFaceMessage { char_id });
                }
                Some(ScriptCmd::HideFace { .. }) => {
                    hide_face_writer.write(HideFaceMessage);
                }
                Some(ScriptCmd::ShowCg { file, transition }) => {
                    show_cg_writer.write(ShowCgMessage {
                        file: file.clone(),
                        transition,
                        duration: None,
                    });
                    unlock_state.cg_unlocked.insert(file);
                }
                Some(ScriptCmd::HideCg { transition }) => {
                    hide_cg_writer.write(HideCgMessage {
                        transition,
                        duration: None,
                    });
                }
                Some(ScriptCmd::DrawSprite {
                    id,
                    file,
                    x,
                    y,
                    z,
                    alpha,
                    priority,
                    time,
                    rotation,
                    anchor_x,
                    anchor_y,
                    blend_mode,
                }) => {
                    if file.contains("_tx") {}
                    draw_sprite_writer.write(DrawSpriteMessage {
                        id,
                        file,
                        x,
                        y,
                        z,
                        alpha,
                        priority,
                        time,
                        rotation,
                        anchor_x,
                        anchor_y,
                        blend_mode,
                    });
                }
                Some(ScriptCmd::FadeSprite { id, time }) => {
                    fade_sprite_writer.write(FadeSpriteMessage { id, time });
                }
                Some(ScriptCmd::MoveSprite {
                    id,
                    x,
                    y,
                    z,
                    alpha,
                    time,
                    wait,
                }) => {
                    move_sprite_writer.write(MoveSpriteMessage {
                        id,
                        x,
                        y,
                        z,
                        alpha,
                        time,
                        wait,
                    });
                }
                Some(ScriptCmd::PlayBgm {
                    id,
                    volume,
                    fade_in,
                }) => {
                    if !intro.0 {
                        play_bgm_writer.write(PlayBgmMessage {
                            id,
                            volume,
                            fade_in,
                        });
                    }
                }
                Some(ScriptCmd::StopBgm { id, fade_out }) => {
                    if !intro.0 {
                        stop_bgm_writer.write(StopBgmMessage { id, fade_out });
                    }
                }
                Some(ScriptCmd::PlayBgmX {
                    id,
                    volume,
                    fade_in,
                }) => {
                    if !intro.0 {
                        play_bgmx_writer.write(PlayBgmXMessage {
                            id,
                            volume,
                            fade_in,
                        });
                    }
                }
                Some(ScriptCmd::StopBgmX { id, fade_out }) => {
                    if !intro.0 {
                        stop_bgmx_writer.write(StopBgmXMessage { id, fade_out });
                    }
                }
                Some(ScriptCmd::PlaySe { file, volume }) => {
                    play_se_writer.write(PlaySeMessage { file, volume });
                }
                Some(ScriptCmd::LoopSe {
                    file,
                    volume,
                    channel,
                }) => {
                    loop_se_writer.write(LoopSeMessage {
                        file,
                        volume,
                        channel,
                    });
                }
                Some(ScriptCmd::StopStreamingSe { channel }) => {
                    stop_streaming_se_writer.write(StopStreamingSeMessage { channel });
                }
                Some(ScriptCmd::PlayVoice { file }) => {
                    pending_voice = Some(file.clone());
                    play_voice_writer.write(PlayVoiceMessage { file, volume: None });
                }
                Some(ScriptCmd::ScrollBg {
                    file,
                    x1,
                    y1,
                    x2,
                    y2,
                    fade,
                    wait,
                }) => {
                    scroll_bg_writer.write(ScrollBgMessage {
                        file,
                        x1,
                        y1,
                        x2,
                        y2,
                        fade,
                        wait,
                    });
                    if wait {
                        auto_skip.auto_timer =
                            Some(Timer::from_seconds(fade as f32 / 1000.0, TimerMode::Once));
                        break;
                    }
                }
                Some(ScriptCmd::AnimateSprite {
                    id,
                    file,
                    max,
                    frame_time,
                    style,
                    x,
                    y,
                    z,
                    anchor_x,
                    anchor_y,
                    rotation,
                    draw,
                    alpha,
                    priority,
                    wait,
                }) => {
                    animate_sprite_writer.write(AnimateSpriteMessage {
                        id,
                        file,
                        max,
                        frame_time,
                        style,
                        x,
                        y,
                        z,
                        anchor_x,
                        anchor_y,
                        rotation,
                        draw,
                        alpha,
                        priority,
                        wait,
                    });
                    if wait {
                        let total_ms = max as u64 * frame_time;
                        auto_skip.auto_timer = Some(Timer::from_seconds(
                            total_ms as f32 / 1000.0,
                            TimerMode::Once,
                        ));
                        break;
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
                Some(ScriptCmd::PlayMovie { file }) => {
                    let actual = map_video_file(&file);
                    let path = format!("movie/{}", actual);
                    let entity = spawn_video(&mut commands, path);
                    pending_video.playing = true;
                    pending_video.entity = Some(entity);
                    pending_video.timer = Some(Timer::from_seconds(3.0, TimerMode::Once));
                    break;
                }
                Some(ScriptCmd::Wait { duration }) => {
                    if settings.skip_mode {
                        // skip mode: continue without waiting
                    } else {
                        auto_skip.auto_timer = Some(Timer::from_seconds(
                            duration as f32 / 1000.0,
                            TimerMode::Once,
                        ));
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
                        *vis = if show {
                            Visibility::Visible
                        } else {
                            Visibility::Hidden
                        };
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
                Some(ScriptCmd::View { ref char_id }) => {
                    if let Some(entry) = view_data::lookup_view_entry(char_id) {
                        let tween_entry = view_data::lookup_tween_entry(entry.pen_type)
                            .unwrap_or_else(|| view_data::lookup_tween_entry(2).unwrap());
                        commands.spawn(ViewState {
                            char_id: char_id.clone(),
                            phase: ViewPhase::FadeOut,
                            timer: Timer::from_seconds(1.0, TimerMode::Once),
                            step_idx: 0,
                            pen_entity: None,
                            name_entity: None,
                            mask_material: None,
                            scene_entities: Vec::new(),
                            entry,
                            tween_entry,
                        });
                        break;
                    }
                }
                Some(ScriptCmd::SetGlobalFlag { index, value }) => {
                    engine.global_flags.insert(index, value);
                }
                Some(ScriptCmd::RouteFlag) => {
                    let count = config.route_unlock_flags.iter()
                        .filter(|&f| engine.global_flags.get(f).copied().unwrap_or(0) >= 1)
                        .count();
                    if count == config.route_unlock_flags.len() {
                        engine.global_flags.insert(config.all_routes_cleared_flag, 1);
                    }
                    if engine.global_flags.get(&config.full_completion_flag) != Some(&1) {
                        let all_clear = (config.ending_flag_range.0..=config.ending_flag_range.1)
                            .chain(std::iter::once(config.all_routes_cleared_flag))
                            .all(|f| engine.global_flags.get(&f).copied().unwrap_or(0) >= 1);
                        if all_clear {
                            engine.global_flags.insert(config.full_completion_flag, 1);
                        }
                    }
                }
                Some(ScriptCmd::GameMode { mode }) => {
                    commands.queue(move |world: &mut World| {
                        let mut settings = world.resource_mut::<Settings>();
                        settings.click_to_advance = mode == 2;
                    });
                }
                Some(ScriptCmd::SetValidity { mode, allowed }) => {
                    match mode {
                        crate::script::ValidityMode::Saving => restrictions.saving = allowed,
                        crate::script::ValidityMode::Loading => restrictions.loading = allowed,
                        crate::script::ValidityMode::Input => restrictions.input = allowed,
                    }
                }
                Some(ScriptCmd::MovieInit) => {}
                Some(ScriptCmd::DrawSpriteEx {
                    ref id,
                    ref file,
                    x,
                    y,
                    width,
                    height,
                    display_mode: _,
                    priority,
                    wait,
                    ..
                }) => {
                    let actual = map_video_file(file);
                    let rel_path = format!("movie/{}", actual);
                    let abs_path = std::env::current_dir()
                        .unwrap_or_default()
                        .join("assets")
                        .join(&rel_path);

                    // Stop any existing sprite video with this ID
                    crate::plugins::video::stop_sprite_video(&mut commands, sprite_video_mgr, id);

                    spawn_sprite_video(
                        &mut commands,
                        images,
                        sprite_video_mgr,
                        id.clone(),
                        &abs_path,
                        x,
                        y,
                        width,
                        height,
                        priority,
                    );

                    if wait {
                        blocked_sprite.0 = Some(id.clone());
                        break;
                    }
                }
                Some(ScriptCmd::WaitToFinishMoviePlayingOnSprite { ref sprite_id }) => {
                    blocked_sprite.0 = Some(sprite_id.clone());
                    break;
                }
                Some(ScriptCmd::RainMja {
                    ref file,
                    priority,
                    ..
                }) => {
                    let rel_path = format!("movie/{}.ogv", file);
                    let abs_path = std::env::current_dir()
                        .unwrap_or_default()
                        .join("assets")
                        .join(&rel_path);

                    start_rain_video(&mut commands, images, rain_state, &abs_path, priority);
                }
                Some(ScriptCmd::SetRainValid { enabled }) => {
                    rain_state.enabled = enabled;
                    if !enabled {
                        crate::plugins::video::stop_rain_video(&mut commands, &mut *rain_state);
                    }
                }
                Some(ScriptCmd::SetRainQuantity { density }) => {
                    rain_state.density = density;
                }
                Some(ScriptCmd::SetRainColor { r, g, b, a }) => {
                    rain_state.color = Color::srgba(
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        a as f32 / 255.0,
                    );
                    // Update existing rain entity color if active
                    if let Some(entity) = rain_state.entity {
                        if let Ok(mut entity_commands) = commands.get_entity(entity) {
                            let handle = {
                                #[cfg(not(target_os = "android"))]
                                {
                                    rain_state.gst.as_ref().map_or(Default::default(), |g| g.image_handle.clone())
                                }
                                #[cfg(target_os = "android")]
                                {
                                    Handle::default()
                                }
                            };
                            entity_commands.insert(ImageNode {
                                image: handle,
                                color: rain_state.color,
                                ..default()
                            });
                        }
                    }
                }
                Some(ScriptCmd::SetRainVector { direction }) => {
                    rain_state.direction = direction;
                }
                Some(ScriptCmd::SetRainCameraAngle { x, y, z }) => {
                    rain_state.camera_angle = (x, y, z);
                }
                Some(ScriptCmd::SetRainPriority { priority }) => {
                    rain_state.priority = priority;
                }
                Some(ScriptCmd::StopAllSe) => {
                    stop_streaming_se_writer.write(StopStreamingSeMessage { channel: 0 });
                }
                Some(ScriptCmd::PushHistory) => {}
                Some(ScriptCmd::WaitVoice) => {
                    auto_skip.auto_timer = Some(Timer::from_seconds(2.0, TimerMode::Once));
                    break;
                }
                Some(ScriptCmd::QueryMode { .. }) => {
                    engine.flags.insert("tmp".to_string(), 0);
                }
                Some(ScriptCmd::StreamingSeVol { .. }) => {}
                Some(ScriptCmd::Blur { .. })
                | Some(ScriptCmd::ShakeScreen { .. })
                | Some(ScriptCmd::ShakeSprite { .. })
                | Some(ScriptCmd::MonologueColor { .. })
                | Some(ScriptCmd::Tween { .. })
                | Some(ScriptCmd::FadeScene { .. })
                | Some(ScriptCmd::NoOp { .. }) => {}
                Some(cmd) => {
                    info!("Script cmd (no-op): {:?}", cmd);
                }
                None => break,
            }
        }

        if !engine.has_more() && !engine.finished {
            engine.finished = true;
            if !engine.call_stack.is_empty() {
                engine.return_from_call();
                engine.finished = false;
            } else if engine.next_script() {
                info!("Script finished: advancing to {}", engine.current_script);
            } else {
                info!("Script finished (no next): returning to title");
                next_state.set(AppState::Title);
            }
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
        dialogue.text_progress =
            (dialogue.text_progress + increment).min(dialogue.current_text.len());
    }
}

fn handle_auto_skip(
    time: Res<Time>,
    mut advance_ev: MessageWriter<AdvanceEvent>,
    mut auto_skip: ResMut<AutoSkipTimer>,
    dialogue: Res<DialogueState>,
    choice_state: Res<ChoiceState>,
    settings: Res<Settings>,
    view_blocking: Res<ViewBlocking>,
) {
    if view_blocking.0 {
        return;
    }

    if choice_state.active {
        auto_skip.auto_timer = None;
        auto_skip.skip_timer = None;
        return;
    }

    let text_fully_displayed =
        !dialogue.is_displaying || dialogue.text_progress >= dialogue.current_text.len();

    if settings.skip_mode {
        if !text_fully_displayed || dialogue.current_text.is_empty() {
            if dialogue.current_text.is_empty() && !dialogue.is_displaying {
                advance_ev.write(AdvanceEvent {
                    source: AdvanceSource::Skip,
                });
            }
            auto_skip.auto_timer = None;
            auto_skip.skip_timer = None;
            return;
        }
        auto_skip.auto_timer = None;
        let timer = auto_skip
            .skip_timer
            .get_or_insert_with(|| Timer::from_seconds(0.05, TimerMode::Once));
        timer.tick(time.delta());
        if timer.just_finished() {
            auto_skip.skip_timer = None;
            advance_ev.write(AdvanceEvent {
                source: AdvanceSource::Skip,
            });
        }
        return;
    }

    if settings.auto_mode {
        if !text_fully_displayed || dialogue.current_text.is_empty() {
            if dialogue.current_text.is_empty() && !dialogue.is_displaying {
                advance_ev.write(AdvanceEvent {
                    source: AdvanceSource::Auto,
                });
            }
            auto_skip.auto_timer = None;
            return;
        }
        let timer = auto_skip
            .auto_timer
            .get_or_insert_with(|| Timer::from_seconds(settings.auto_delay_secs, TimerMode::Once));
        timer.tick(time.delta());
        if timer.just_finished() {
            auto_skip.auto_timer = None;
            advance_ev.write(AdvanceEvent {
                source: AdvanceSource::Auto,
            });
        }
        return;
    }

    if let Some(timer) = &mut auto_skip.auto_timer {
        timer.tick(time.delta());
        if timer.just_finished() {
            auto_skip.auto_timer = None;
            advance_ev.write(AdvanceEvent {
                source: AdvanceSource::Auto,
            });
        }
    }
}

fn persist_gameplay(unlock_state: Res<UnlockState>, settings: Res<Settings>) {
    save_unlock_state(&unlock_state);
    crate::resources::save_settings(&settings);
}

fn clear_scene_sprites(
    overlay_mgr: &mut SpriteOverlayManager,
    commands: &mut Commands,
    hide_fg_writer: &mut MessageWriter<HideFgMessage>,
    hide_cg_writer: &mut MessageWriter<HideCgMessage>,
    overlay_query: &mut Query<(Entity, &mut BackgroundColor, &mut Visibility), With<ScreenOverlayRoot>>,
) {
    for (_, entity) in overlay_mgr.sprites.drain() {
        commands.entity(entity).despawn();
    }
    hide_fg_writer.write(HideFgMessage {
        char_id: "all".to_string(),
        transition: None,
        duration: None,
    });
    hide_cg_writer.write(HideCgMessage {
        transition: None,
        duration: None,
    });
    for (entity, mut bg, mut vis) in overlay_query.iter_mut() {
        *vis = Visibility::Hidden;
        bg.0.set_alpha(0.0);
        commands.entity(entity).remove::<OverlayTween>();
    }
}
