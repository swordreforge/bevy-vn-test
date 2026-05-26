use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use crate::resources::{AffectionMap, Backlog, BacklogEntry, ChoiceState, DialogueState, Settings, UnlockState};
use crate::audio_messages::{
    PlayBgmMessage, StopBgmMessage, PlaySeMessage, PlayVoiceMessage,
};
use crate::choice_messages::ChoiceSelectedMessage;
use crate::script::{ConditionOp, ScriptCmd, ScriptEngine};
use crate::state::AppState;
use crate::plugins::inputs::AdvanceEvent;
use crate::rendering_messages::{
    SetBgMessage, ShowFgMessage, HideFgMessage, ShowCgMessage, HideCgMessage,
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
    show_cg_writer: MessageWriter<'w, ShowCgMessage>,
    hide_cg_writer: MessageWriter<'w, HideCgMessage>,
    choice_ev: MessageReader<'w, 's, ChoiceSelectedMessage>,
    choice_state: ResMut<'w, ChoiceState>,
    play_bgm_writer: MessageWriter<'w, PlayBgmMessage>,
    stop_bgm_writer: MessageWriter<'w, StopBgmMessage>,
    play_se_writer: MessageWriter<'w, PlaySeMessage>,
    play_voice_writer: MessageWriter<'w, PlayVoiceMessage>,
    settings: Res<'w, Settings>,
    auto_skip: ResMut<'w, AutoSkipTimer>,
}

impl Plugin for ScriptRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoSkipTimer>()
            .add_systems(OnEnter(AppState::Gameplay), start_script_execution)
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
    mut engine: ResMut<ScriptEngine>,
) {
    dialogue.current_text.clear();
    dialogue.current_speaker = None;
    dialogue.text_progress = 0;
    dialogue.is_displaying = false;
    engine.dialogue_idx = 0;
}

fn process_advance(mut params: ProcessAdvanceParams<'_, '_>) {
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
        ref mut show_cg_writer,
        ref mut hide_cg_writer,
        ref mut choice_ev,
        ref mut choice_state,
        ref mut play_bgm_writer,
        ref mut stop_bgm_writer,
        ref mut play_se_writer,
        ref mut play_voice_writer,
        ref settings,
        ref mut auto_skip,
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
                        engine.dialogue_idx += 1;
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
                    Some(ScriptCmd::ShowCg { file, .. }) => {
                        show_cg_writer.write(ShowCgMessage { file: file.clone(), transition: None, duration: None });
                        unlock_state.cg_unlocked.insert(file);
                    }
                    Some(ScriptCmd::HideCg { .. }) => {
                        hide_cg_writer.write(HideCgMessage { transition: None, duration: None });
                    }
                    Some(ScriptCmd::PlayBgm { id, volume, fade_in }) => {
                        play_bgm_writer.write(PlayBgmMessage { id, volume, fade_in });
                    }
                    Some(ScriptCmd::StopBgm { id, fade_out }) => {
                        stop_bgm_writer.write(StopBgmMessage { id, fade_out });
                    }
                    Some(ScriptCmd::PlaySe { file, volume }) => {
                        play_se_writer.write(PlaySeMessage { file, volume });
                    }
                    Some(ScriptCmd::PlayVoice { file }) => {
                        pending_voice = Some(file.clone());
                        play_voice_writer.write(PlayVoiceMessage { file, volume: None });
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
                    engine.dialogue_idx += 1;
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
                Some(ScriptCmd::ShowCg { file, transition }) => {
                    show_cg_writer.write(ShowCgMessage { file: file.clone(), transition, duration: None });
                    unlock_state.cg_unlocked.insert(file);
                }
                Some(ScriptCmd::HideCg { transition }) => {
                    hide_cg_writer.write(HideCgMessage { transition, duration: None });
                }
                Some(ScriptCmd::PlayBgm { id, volume, fade_in }) => {
                    play_bgm_writer.write(PlayBgmMessage { id, volume, fade_in });
                }
                Some(ScriptCmd::StopBgm { id, fade_out }) => {
                    stop_bgm_writer.write(StopBgmMessage { id, fade_out });
                }
                Some(ScriptCmd::PlaySe { file, volume }) => {
                    play_se_writer.write(PlaySeMessage { file, volume });
                }
                Some(ScriptCmd::PlayVoice { file }) => {
                    pending_voice = Some(file.clone());
                    play_voice_writer.write(PlayVoiceMessage { file, volume: None });
                }
                Some(ScriptCmd::Choice { options }) => {
                    choice_state.active = true;
                    choice_state.options = options;
                    break;
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

    if !text_fully_displayed || dialogue.current_text.is_empty() {
        auto_skip.auto_timer = None;
        auto_skip.skip_timer = None;
        return;
    }

    if settings.auto_mode && !settings.skip_mode {
        let timer = auto_skip.auto_timer.get_or_insert_with(|| {
            Timer::from_seconds(2.0, TimerMode::Once)
        });
        timer.tick(time.delta());
        if timer.just_finished() {
            advance_ev.write(AdvanceEvent);
            auto_skip.auto_timer = None;
        }
    }

    if settings.skip_mode {
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
