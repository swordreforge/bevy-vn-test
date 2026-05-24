use bevy::prelude::*;
use crate::resources::{AffectionMap, ChoiceState, DialogueState};
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

impl Plugin for ScriptRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Gameplay), start_script_execution)
            .add_systems(
                Update,
                (process_advance, update_text_reveal)
                    .run_if(in_state(AppState::Gameplay)),
            );
    }
}

fn start_script_execution(mut dialogue: ResMut<DialogueState>) {
    dialogue.current_text.clear();
    dialogue.current_speaker = None;
    dialogue.text_progress = 0;
    dialogue.is_displaying = false;
}

fn process_advance(
    mut advance_ev: MessageReader<AdvanceEvent>,
    mut engine: ResMut<ScriptEngine>,
    mut dialogue: ResMut<DialogueState>,
    mut affection: ResMut<AffectionMap>,
    mut set_bg_writer: MessageWriter<SetBgMessage>,
    mut show_fg_writer: MessageWriter<ShowFgMessage>,
    mut hide_fg_writer: MessageWriter<HideFgMessage>,
    mut show_cg_writer: MessageWriter<ShowCgMessage>,
    mut hide_cg_writer: MessageWriter<HideCgMessage>,
    mut choice_ev: MessageReader<ChoiceSelectedMessage>,
    mut choice_state: ResMut<ChoiceState>,
    mut play_bgm_writer: MessageWriter<PlayBgmMessage>,
    mut stop_bgm_writer: MessageWriter<StopBgmMessage>,
    mut play_se_writer: MessageWriter<PlaySeMessage>,
    mut play_voice_writer: MessageWriter<PlayVoiceMessage>,
) {
    for _ in advance_ev.read() {
        // If choice is active, check if user made a selection
        if choice_state.active {
            for ev in choice_ev.read() {
                if ev.index < choice_state.options.len() {
                    let option = &choice_state.options[ev.index];
                    // Apply affection change
                    if let Some((ref char_id, delta)) = option.affection_change {
                        *affection.0.entry(char_id.clone()).or_insert(0) += delta;
                    }
                    // Jump to label
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

        while engine.has_more() {
            let cmd = engine.advance().cloned();
            match cmd {
                Some(ScriptCmd::Dialogue { speaker, text }) => {
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
                Some(ScriptCmd::Return) => {
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
                        ConditionOp::GreaterEqual => flag_val >= value,
                        ConditionOp::LessEqual => flag_val <= value,
                    };
                    if met && !engine.jump_to_label(&goto) {
                        warn!("Condition jump target not found: {}", goto);
                    }
                }
                Some(ScriptCmd::AffectionChange { char_id, delta }) => {
                    *affection.0.entry(char_id).or_insert(0) += delta;
                }
                Some(ScriptCmd::SavePoint) => {}
                Some(ScriptCmd::SetBg { file, transition: _, duration: _ }) => {
                    set_bg_writer.write(SetBgMessage { file });
                }
                Some(ScriptCmd::ShowFg { char_id, expression, position, transition: _ }) => {
                    show_fg_writer.write(ShowFgMessage { char_id, expression, position });
                }
                Some(ScriptCmd::HideFg { char_id, transition: _ }) => {
                    hide_fg_writer.write(HideFgMessage { char_id });
                }
                Some(ScriptCmd::ShowCg { file, transition: _ }) => {
                    show_cg_writer.write(ShowCgMessage { file });
                }
                Some(ScriptCmd::HideCg { transition: _ }) => {
                    hide_cg_writer.write(HideCgMessage);
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
                    play_voice_writer.write(PlayVoiceMessage { file, volume: None });
                }
                Some(ScriptCmd::Choice { options }) => {
                    choice_state.active = true;
                    choice_state.options = options;
                    break;
                }
                // Keep no-op log for truly unsupported commands
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

fn update_text_reveal(time: Res<Time>, mut dialogue: ResMut<DialogueState>) {
    if dialogue.is_displaying && dialogue.text_progress < dialogue.current_text.len() {
        let chars_per_sec = 40.0;
        let increment = (time.delta_secs() * chars_per_sec) as usize;
        dialogue.text_progress = (dialogue.text_progress + increment).min(dialogue.current_text.len());
    }
}
