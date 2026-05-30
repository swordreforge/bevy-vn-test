use crate::asb::{AsbCommand, AsbScript};
use bevy_vn::script::{ChoiceOption, FgPosition, OverlayColor, Script, ScriptCmd, Transition, ValidityMode};
use std::collections::HashMap;

pub fn map_script(
    asb: &AsbScript,
    config: &crate::lua_config::GameConfig,
    verbose: bool,
) -> Script {
    let mut output = Script::new();
    let mut pending_speaker: Option<String> = None;

    let mut pending_choice_opts: Vec<ChoiceOption> = Vec::new();
    let mut pending_exp_values: Vec<i32> = Vec::new();
    let mut last_choice_idx: Option<usize> = None;

    for block in &asb.blocks {
        output.push(ScriptCmd::Label {
            name: block.label.clone(),
        });

        for cmd in &block.commands {
            match cmd.tag.as_str() {
                "name" => {
                    pending_speaker = cmd.attrs.get("0").cloned();
                }
                "print" => {
                    if let Some(text) = cmd.attrs.get("data") {
                        output.push(ScriptCmd::Dialogue {
                            speaker: pending_speaker.take(),
                            text: text.to_string(),
                        });
                    }
                }
                "click" | "rp2" | "ruby" | "/ruby" => {}
                "sel_init" => {
                    pending_choice_opts.clear();
                    pending_exp_values.clear();
                }
                "sel_text" => {
                    let text = cmd.attrs.get("text").cloned().unwrap_or_default();
                    let exp_val = cmd
                        .attrs
                        .get("exp")
                        .and_then(|s| s.strip_prefix("t.ens:"))
                        .and_then(|s| s.parse::<i32>().ok());
                    if let Some(ev) = exp_val {
                        pending_exp_values.push(ev);
                    }
                    pending_choice_opts.push(ChoiceOption {
                        text,
                        affection_change: None,
                        goto: None,
                    });
                }
                "select" => {
                    let options = std::mem::take(&mut pending_choice_opts);
                    last_choice_idx = Some(output.len());
                    output.push(ScriptCmd::Choice { options });
                }
                "Select" => {}
                "exswitch" => {
                    if let Some(data) = cmd.attrs.get("data") {
                        update_choice_gotos(
                            &mut output,
                            &mut last_choice_idx,
                            data,
                            &pending_exp_values,
                        );
                    }
                    pending_exp_values.clear();
                }
                tag => {
                    if let Some(commands) = map_command(tag, cmd, config) {
                        output.extend(commands);
                    } else if verbose {
                        eprintln!("  [skip] {} ({})", tag, cmd.attrs.len());
                    }
                }
            }
        }
    }

    output
}

fn update_choice_gotos(
    output: &mut Script,
    last_choice_idx: &mut Option<usize>,
    data: &str,
    exp_values: &[i32],
) {
    let parts: Vec<&str> = data.split("<>").collect();
    if parts.len() < 2 {
        return;
    }

    let mut branch_map: HashMap<i32, String> = HashMap::new();
    let mut default_target: Option<String> = None;

    for part in &parts[1..] {
        if let Some((key, target)) = part.split_once(':') {
            if key == "default" {
                default_target = Some(target.to_string());
            } else if let Ok(val) = key.parse::<i32>() {
                branch_map.insert(val, target.to_string());
            }
        }
    }

    if let Some(idx) = last_choice_idx.take() {
        if idx < output.len() {
            if let ScriptCmd::Choice { ref mut options } = &mut output[idx] {
                for (i, opt) in options.iter_mut().enumerate() {
                    let exp_val = exp_values.get(i).copied().unwrap_or(i as i32);
                    if let Some(target) = branch_map.get(&exp_val) {
                        opt.goto = Some(target.clone());
                    } else if let Some(ref default) = default_target {
                        opt.goto = Some(default.clone());
                    }
                }
            }
        }
    }
}

fn map_command(
    tag: &str,
    cmd: &AsbCommand,
    _config: &crate::lua_config::GameConfig,
) -> Option<Vec<ScriptCmd>> {
    match tag {
        "Wait" => {
            let duration = cmd
                .attrs
                .get("0")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1000);
            Some(vec![ScriptCmd::Wait { duration }])
        }
        "Voice" => {
            let file = cmd.attrs.get("0")?;
            Some(vec![ScriptCmd::PlayVoice {
                file: file.to_string(),
            }])
        }
        "Tati" => {
            let char_id = cmd.attrs.get("0")?;
            let expression = cmd.attrs.get("1").cloned().unwrap_or("000".into());
            Some(vec![ScriptCmd::ShowFg {
                char_id: char_id.to_string(),
                expression,
                position: FgPosition::Center,
                transition: None,
            }])
        }
        "TatiFa" => {
            let char_id = cmd.attrs.get("0")?;
            let expression = cmd.attrs.get("1").cloned().unwrap_or("000".into());
            Some(vec![ScriptCmd::ShowFg {
                char_id: char_id.to_string(),
                expression,
                position: FgPosition::Center,
                transition: None,
            }])
        }
        "Face" => {
            let char_id = cmd.attrs.get("0").cloned().unwrap_or("0".into());
            let expression = cmd.attrs.get("1").cloned().unwrap_or("000".into());
            Some(vec![ScriptCmd::ShowFace { char_id, expression }])
        }
        "ClrFace" => {
            Some(vec![ScriptCmd::HideFace { char_id: "all".into() }])
        }
        "DrawSprite" => {
            let id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let file = cmd.attrs.get("1").cloned().unwrap_or_default();
            let x = cmd.attrs.get("3").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y = cmd.attrs.get("4").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let z = cmd.attrs.get("5").and_then(|s| s.parse().ok()).unwrap_or(0);
            let alpha = cmd.attrs.get("12").and_then(|s| s.parse().ok()).unwrap_or(255);
            let priority = cmd.attrs.get("13").and_then(|s| s.parse().ok()).unwrap_or(0);
            let time = cmd.attrs.get("14").and_then(|s| s.parse().ok()).unwrap_or(0);
            let rotation = cmd.attrs.get("6").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let anchor_x = cmd.attrs.get("7").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.5);
            let anchor_y = cmd.attrs.get("8").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.5);
            let blend_mode = cmd.attrs.get("2").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::DrawSprite { id, file, x, y, z, alpha, priority, time, rotation, anchor_x, anchor_y, blend_mode }])
        }
        "DrawSpriteWithFiltering" => {
            let id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let file = cmd.attrs.get("1").cloned().unwrap_or_default();
            let x = cmd.attrs.get("4").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y = cmd.attrs.get("5").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let alpha = cmd.attrs.get("9").and_then(|s| s.parse().ok()).unwrap_or(255);
            let priority = cmd.attrs.get("10").and_then(|s| s.parse().ok()).unwrap_or(0);
            let time = cmd.attrs.get("11").and_then(|s| s.parse().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::DrawSprite { id, file, x, y, z: 0, alpha, priority, time, rotation: 0.0, anchor_x: 0.5, anchor_y: 0.5, blend_mode: 0 }])
        }
        "FadeSprite" => {
            let id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let time = cmd.attrs.get("1").and_then(|s| s.parse().ok()).unwrap_or(500);
            Some(vec![ScriptCmd::FadeSprite { id, time }])
        }
        "FadeSpriteWithFiltering" => {
            let id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let time = cmd.attrs.get("3").and_then(|s| s.parse().ok()).unwrap_or(500);
            Some(vec![ScriptCmd::FadeSprite { id, time }])
        }
        "MoveSprite" => {
            let id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let x = cmd.attrs.get("1").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y = cmd.attrs.get("2").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let z = cmd.attrs.get("3").and_then(|s| s.parse().ok()).unwrap_or(0);
            let alpha = cmd.attrs.get("5").and_then(|s| s.parse().ok()).unwrap_or(255);
            let time = cmd.attrs.get("8").and_then(|s| s.parse().ok()).unwrap_or(0);
            let wait = cmd.attrs.get("9").map(|s| s == "TRUE").unwrap_or(false);
            Some(vec![ScriptCmd::MoveSprite { id, x, y, z, alpha, time, wait }])
        }
        "AnimateSprite" => {
            let id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let file = cmd.attrs.get("1").cloned().unwrap_or_default();
            let max: u32 = cmd.attrs.get("2").and_then(|v| v.parse().ok()).unwrap_or(1);
            let frame_time: u64 = cmd.attrs.get("3").and_then(|v| v.parse().ok()).unwrap_or(200);
            let style: u32 = cmd.attrs.get("4").and_then(|v| v.parse().ok()).unwrap_or(0);
            let x: f32 = cmd.attrs.get("6").and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let y: f32 = cmd.attrs.get("7").and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let z: i32 = cmd.attrs.get("8").and_then(|v| v.parse().ok()).unwrap_or(0);
            let anchor_x: f32 = cmd.attrs.get("9").and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let anchor_y: f32 = cmd.attrs.get("10").and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let rotation: f32 = cmd.attrs.get("11").and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let draw: u32 = cmd.attrs.get("14").and_then(|v| v.parse().ok()).unwrap_or(0);
            let alpha: i32 = cmd.attrs.get("15").and_then(|v| v.parse().ok()).unwrap_or(255);
            let priority: i32 = cmd.attrs.get("16").and_then(|v| v.parse().ok()).unwrap_or(0);
            let wait: bool = cmd.attrs.get("18").map_or(false, |v| v == "1");
            Some(vec![ScriptCmd::AnimateSprite { id, file, max, frame_time, style, x, y, z, anchor_x, anchor_y, rotation, draw, alpha, priority, wait }])
        }
        "ClrTati" => {
            Some(vec![ScriptCmd::HideFg {
                char_id: "all".into(),
                transition: None,
            }])
        }
        "Back" => {
            let num = cmd.attrs.get("0")?;
            Some(vec![ScriptCmd::SetBg {
                file: format!("bg_{}.jpg", num),
                transition: None,
                duration: None,
            }])
        }
        "BgmPlay" => {
            let file_id = cmd.attrs.get("1")?;
            let volume = match cmd.attrs.get("2").map(|s| s.as_str()) {
                Some("MIN") => Some(0.5),
        _ => None,
            };
            Some(vec![ScriptCmd::PlayBgm {
                id: file_id.to_string(),
                volume,
                fade_in: None,
            }])
        }
        "BgmStop" => {
            let fade_out = match cmd.attrs.get("1").map(|s| s.as_str()) {
                Some("FADE") => Some(500u64),
                _ => Some(0u64),
            };
            Some(vec![ScriptCmd::StopBgm { id: None, fade_out }])
        }
        "BgmVol" => {
            let channel = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(0);
            let volume = cmd.attrs.get("1").cloned().unwrap_or("HIGH".into());
            Some(vec![ScriptCmd::BgmVol { channel, volume }])
        }
        "SEPlay" => {
            let file = cmd.attrs.get("0")?;
            Some(vec![ScriptCmd::PlaySe {
                file: file.to_string(),
                volume: None,
            }])
        }
        "LoopSE" => {
            let channel = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(0);
            let file = cmd.attrs.get("1")?;
            Some(vec![ScriptCmd::LoopSe {
                file: file.to_string(),
                volume: None,
                channel,
            }])
        }
        "StopStreamingSE" => {
            let channel = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::StopStreamingSe { channel }])
        }
        "FadeFilm" => {
            let duration = cmd.attrs.get("0").and_then(|s| s.parse::<u64>().ok()).unwrap_or(500);
            Some(vec![ScriptCmd::ClearOverlay { time: duration }])
        }
        "msgon" => {
            Some(vec![ScriptCmd::Window { show: true, time: None }])
        }
        "SetJumpLabel" => None,
        "return_main" => Some(vec![ScriptCmd::Return]),
        "CallScript" => {
            let target = cmd.attrs.get("0")?;
            let label = cmd.attrs.get("1").filter(|s| !s.is_empty()).cloned();
            let script = format!("aiy{:05}", target.parse::<u32>().ok()?);
            Some(vec![ScriptCmd::CallScript { script, label }])
        }
        "calllua" => map_calllua(cmd, _config),
        "Fadeout" => {
            let fade_type = cmd.attrs.get("0").map(|s| s.as_str()).unwrap_or("BLACK");
            let speed_str = cmd.attrs.get("1").map(|s| s.as_str()).unwrap_or("NORMAL");
            let time: u64 = match speed_str {
                "FAST" => 500,
                "SLOW" => 1500,
                _ => 1000,
            };
            let color = match fade_type {
                "WHITE" | "SA" => OverlayColor::White,
                _ => OverlayColor::Black,
            };
            let wait_time = time / 2;
            let bg_file = match fade_type {
                "BLACK" => "bg_0000",
                _ => "bg_9999",
            };
            Some(vec![
                ScriptCmd::ScreenOverlay { color, time },
                ScriptCmd::Wait { duration: wait_time },
                ScriptCmd::SetBg { file: bg_file.to_string(), transition: None, duration: None },
                ScriptCmd::ClearOverlay { time: 0 },
            ])
        }
        "Blackout" => {
            let time = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(1000);
            Some(vec![ScriptCmd::ScreenOverlay { color: OverlayColor::Black, time }])
        }
        "WhiteoutBySA" => {
            let time = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(1000);
            Some(vec![ScriptCmd::ScreenOverlay { color: OverlayColor::White, time }])
        }
        "Window" => {
            let show = cmd.attrs.get("0").map(|s| s.as_str()) != Some("OFF");
            Some(vec![ScriptCmd::Window { show, time: None }])
        }
        "DisableWindow" => {
            let time = cmd.attrs.get("0").and_then(|s| s.parse::<u64>().ok());
            Some(vec![ScriptCmd::Window { show: false, time }])
        }
        "EnableWindow" => {
            let time = cmd.attrs.get("0").and_then(|s| s.parse::<u64>().ok());
            Some(vec![ScriptCmd::Window { show: true, time }])
        }
        "ChangeWindowColor" => {
            let color_idx = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::ChangeWindowColor { color_idx }])
        }
        "ChangeWindowDesign" => {
            let design = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::ChangeWindowDesign { design }])
        }
        "Quake" | "Jishin" => {
            let power = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(5.0);
            let time = cmd.attrs.get("1").and_then(|s| s.parse().ok()).unwrap_or(500);
            Some(vec![ScriptCmd::Quake { power, time }])
        }
        "Flash" => {
            let color_val = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let color = if color_val == 0 { OverlayColor::White } else { OverlayColor::Black };
            let time = cmd.attrs.get("1").and_then(|s| s.parse().ok()).unwrap_or(500);
            let alpha = cmd.attrs.get("2").and_then(|s| s.parse().ok()).unwrap_or(128);
            Some(vec![ScriptCmd::Flash { color, time, alpha }])
        }
        "View" => {
            let char_id = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::View { char_id }])
        }
        "ViewEnd" => {
            Some(vec![ScriptCmd::View { char_id: "ViewEnd".into() }])
        }
        "Event" => {
            let file = cmd.attrs.get("0").cloned().unwrap_or_default();
            let file = format!("eve_{}", file);
            let transition = map_event_transition(cmd.attrs.get("1"));
            Some(vec![
                ScriptCmd::Window { show: false, time: None },
                ScriptCmd::HideFg { char_id: "all".into(), transition: None },
                ScriptCmd::ShowCg { file, transition },
            ])
        }
        "EventMN" => {
            let file = cmd.attrs.get("0").cloned().unwrap_or_default();
            let file = format!("mon_{}", file);
            let transition = map_event_transition(cmd.attrs.get("1"));
            Some(vec![
                ScriptCmd::Window { show: false, time: None },
                ScriptCmd::HideFg { char_id: "all".into(), transition: None },
                ScriptCmd::ShowCg { file, transition },
            ])
        }
        "EventCut" => {
            let file = cmd.attrs.get("0").cloned().unwrap_or_default();
            let file = format!("cut_{}", file);
            let transition = map_event_transition(cmd.attrs.get("1"));
            Some(vec![
                ScriptCmd::Window { show: false, time: None },
                ScriptCmd::HideFg { char_id: "all".into(), transition: None },
                ScriptCmd::ShowCg { file, transition },
            ])
        }
        "DrawScene" => {
            let file = cmd.attrs.get("0").cloned().unwrap_or_default();
            let prefix = cmd.attrs.get("2").cloned().unwrap_or_default();
            let file = format!("{}{}", prefix, file);
            let transition = map_event_transition(cmd.attrs.get("1"));
            Some(vec![
                ScriptCmd::Window { show: false, time: None },
                ScriptCmd::HideFg { char_id: "all".into(), transition: None },
                ScriptCmd::ShowCg { file, transition },
            ])
        }
        "SetGlobalFlag" => {
            let index = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let value = cmd.attrs.get("1").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::SetGlobalFlag { index, value }])
        }
        "GetGlobalFlag" => {
            let index = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::GetGlobalFlag { index }])
        }
        "StoreValueToLocalWork" => {
            let index = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let raw = cmd.attrs.get("1").cloned().unwrap_or_default();
            let (value, expression) = if raw.starts_with("t.tmp") {
                (0, Some(raw))
            } else {
                (raw.parse::<i32>().unwrap_or(0), None)
            };
            Some(vec![ScriptCmd::StoreValueToLocalWork { index, value, expression }])
        }
        "LoadValueFromLocalWork" => {
            let index = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::LoadValueFromLocalWork { index }])
        }
        "GetLocalFlag" => {
            let index = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::GetLocalFlag { index }])
        }
        "SetLocalFlag" => {
            let index = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let value = cmd.attrs.get("1").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::SetLocalFlag { index, value }])
        }
        "RouteFlag" => {
            Some(vec![ScriptCmd::RouteFlag])
        }
        "GameMode" => {
            let mode = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::GameMode { mode }])
        }
        "SetValidityOfSaving" => {
            let allowed = cmd.attrs.get("0").map(|s| s != "0").unwrap_or(true);
            Some(vec![ScriptCmd::SetValidity { mode: ValidityMode::Saving, allowed }])
        }
        "SetValidityOfLoading" => {
            let allowed = cmd.attrs.get("0").map(|s| s != "0").unwrap_or(true);
            Some(vec![ScriptCmd::SetValidity { mode: ValidityMode::Loading, allowed }])
        }
        "SetValidityOfInput" => {
            let allowed = cmd.attrs.get("0").map(|s| s != "0").unwrap_or(true);
            Some(vec![ScriptCmd::SetValidity { mode: ValidityMode::Input, allowed }])
        }
        "SavePoint" => {
            Some(vec![ScriptCmd::SavePoint])
        }
        "Refresh" => None,
        "NextDay" => {
            Some(vec![
                ScriptCmd::Window { show: false, time: None },
                ScriptCmd::ScreenOverlay { color: OverlayColor::Black, time: 0 },
                ScriptCmd::ClearOverlay { time: 500 },
                ScriptCmd::Wait { duration: 2000 },
                ScriptCmd::ScreenOverlay { color: OverlayColor::Black, time: 1000 },
            ])
        }
        _ => None,
    }
}

fn map_event_transition(attr: Option<&String>) -> Option<Transition> {
    match attr.map(|s| s.as_str()) {
        Some("SUDDEN") | Some("0") => None,
        Some("FAST") | Some("1") => Some(Transition::Fade),
        Some("FADE") | Some("2") => Some(Transition::Fade),
        Some("CROSS") | Some("3") => Some(Transition::Fade),
        Some("DISSOLVE") | Some("4") => Some(Transition::Dissolve),
        _ => None,
    }
}

fn map_calllua(cmd: &AsbCommand, _config: &crate::lua_config::GameConfig) -> Option<Vec<ScriptCmd>> {
    let func = cmd.attrs.get("function")?;
    match func.as_str() {
        s if s.contains("set_bg") || s.contains("setbg") => {
            let file = cmd.attrs.get("file")?;
            Some(vec![ScriptCmd::SetBg {
                file: file.to_string(),
                transition: None,
                duration: None,
            }])
        }
        s if s.contains("ScrollBGenq") || s.contains("scroll_bg") => {
            let file = cmd.attrs.get("file").or_else(|| cmd.attrs.get("0")).cloned().unwrap_or_default();
            let x1 = cmd.attrs.get("1").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y1 = cmd.attrs.get("2").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let x2 = cmd.attrs.get("4").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let y2 = cmd.attrs.get("5").and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let fade = cmd.attrs.get("9").and_then(|s| s.parse().ok()).unwrap_or(0);
            let wait = cmd.attrs.get("10").map(|s| s == "TRUE").unwrap_or(false);
            Some(vec![ScriptCmd::ScrollBg { file, x1, y1, x2, y2, fade, wait }])
        }
        s if s.contains("ChangeVolumeOfBGM") || s.contains("bgm_fade") => {
            let channel = cmd.attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(0);
            let vol_val = cmd.attrs.get("1").and_then(|s| s.parse::<u32>().ok()).unwrap_or(80);
            let volume = match vol_val {
                0 => "MIN",
                30 => "LOW",
                80 => "NORM",
                _ => "HIGH",
            }.to_string();
            Some(vec![ScriptCmd::BgmVol { channel, volume }])
        }
        s if s.contains("bgm_play") => {
            let id = cmd.attrs.get("file")?;
            let volume = cmd.attrs.get("vol").and_then(|v| v.parse::<f32>().ok()).map(|v| v / 100.0);
            let fade_in = cmd.attrs.get("time").and_then(|t| t.parse::<u64>().ok());
            Some(vec![ScriptCmd::PlayBgm {
                id: id.to_string(),
                volume,
                fade_in,
            }])
        }
        s if s.contains("bgm_stop") => {
            let id = cmd.attrs.get("file").map(|s| s.to_string());
            let fade_out = cmd.attrs.get("time").and_then(|t| t.parse::<u64>().ok());
            Some(vec![ScriptCmd::StopBgm { id, fade_out }])
        }
        s if s.contains("se_play") => {
            let file = cmd.attrs.get("file")?;
            let has_loop = cmd.attrs.get("loop").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0) != 0;
            if has_loop {
                let channel = cmd.attrs.get("id").and_then(|s| s.parse().ok()).unwrap_or(0);
                Some(vec![ScriptCmd::LoopSe {
                    file: file.to_string(),
                    volume: None,
                    channel,
                }])
            } else {
                Some(vec![ScriptCmd::PlaySe {
                    file: file.to_string(),
                    volume: None,
                }])
            }
        }
        s if s.contains("se_stop") => {
            let channel = cmd.attrs.get("id").and_then(|s| s.parse().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::StopStreamingSe { channel }])
        }
        s if s.contains("voice_play") => {
            let file = cmd.attrs.get("file")?;
            Some(vec![ScriptCmd::PlayVoice {
                file: file.to_string(),
            }])
        }
        s if s.contains("show_fg") || s.contains("fg_show") => {
            let char_id = cmd.attrs.get("id")?;
            let expression = cmd.attrs.get("file").cloned().unwrap_or("000".into());
            Some(vec![ScriptCmd::ShowFg {
                char_id: char_id.to_string(),
                expression,
                position: FgPosition::Center,
                transition: None,
            }])
        }
        s if s.contains("hide_fg") || s.contains("fg_hide") => {
            let char_id = cmd.attrs.get("id").cloned().unwrap_or("all".into());
            Some(vec![ScriptCmd::HideFg {
                char_id,
                transition: None,
            }])
        }
        s if s.contains("show_cg") || s.contains("cg_show") => {
            let file = cmd.attrs.get("file")?;
            Some(vec![ScriptCmd::ShowCg {
                file: file.to_string(),
                transition: None,
            }])
        }
        s if s.contains("hide_cg") || s.contains("cg_hide") => {
            Some(vec![ScriptCmd::HideCg { transition: None }])
        }
        s if s.contains("affection_change") || s.contains("affection") => {
            let char_id = cmd.attrs.get("char_id").cloned().unwrap_or("default".into());
            let delta = cmd.attrs.get("delta").and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
            Some(vec![ScriptCmd::AffectionChange { char_id, delta }])
        }
        s if s.contains("choice") || s.contains("tags.choice") => {
            Some(vec![ScriptCmd::Choice {
                options: vec![ChoiceOption {
                    text: format!("[choice in {}]", func),
                    affection_change: None,
                    goto: None,
                }],
            }])
        }
        s if s.contains("save_point") || s.contains("quicksave") => {
            Some(vec![ScriptCmd::SavePoint])
        }
        s if s.contains("wait_tag") || s.contains("tags.wt") || s.contains("tags.wtx") => {
            let duration = cmd.attrs.get("time").and_then(|s| s.parse::<u64>().ok()).unwrap_or(500);
            Some(vec![ScriptCmd::Wait { duration }])
        }
        s if s.contains("unlock_cg") || s.contains("cg_unlock") => {
            let file = cmd.attrs.get("file").cloned().unwrap_or("unknown".into());
            Some(vec![ScriptCmd::UnlockCg { file }])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua_config::GameConfig;
    use std::collections::HashMap;

    fn cmd(tag: &str, attrs: Vec<(&str, &str)>) -> AsbCommand {
        let mut map = HashMap::new();
        for (k, v) in attrs {
            map.insert(k.to_string(), v.to_string());
        }
        AsbCommand {
            tag: tag.to_string(),
            attrs: map,
        }
    }

    fn assert_wait(result: Option<Vec<ScriptCmd>>, expected: u64) {
        assert!(result.is_some());
        let cmds = result.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::Wait { duration: d } if *d == expected));
    }

    fn assert_hide_fg(result: Option<Vec<ScriptCmd>>, expected: &str) {
        assert!(result.is_some());
        let cmds = result.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::HideFg { char_id: ref c, .. } if c == expected));
    }

    #[test]
    fn test_map_wait() {
        assert_wait(
            map_command("Wait", &cmd("Wait", vec![("0", "2000")]), &GameConfig::default()),
            2000,
        );
    }

    #[test]
    fn test_map_wait_default() {
        assert_wait(
            map_command("Wait", &cmd("Wait", vec![]), &GameConfig::default()),
            1000,
        );
    }

    #[test]
    fn test_map_voice() {
        let r = map_command("Voice", &cmd("Voice", vec![("0", "vo_001")]), &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::PlayVoice { ref file } if file == "vo_001"));
    }

    #[test]
    fn test_map_tatifa() {
        let c = cmd("TatiFa", vec![("0", "001_eus"), ("1", "010003")]);
        let r = map_command("TatiFa", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::ShowFg { ref char_id, ref expression, .. }
            if char_id == "001_eus" && expression == "010003"));
    }

    #[test]
    fn test_map_face() {
        let c = cmd("Face", vec![("0", "350101"), ("1", "000")]);
        let r = map_command("Face", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::ShowFace { ref char_id, ref expression }
            if char_id == "350101" && expression == "000"));
    }

    #[test]
    fn test_map_clrface() {
        let r = map_command("ClrFace", &cmd("ClrFace", vec![]), &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::HideFace { .. }));
    }

    #[test]
    fn test_map_clrtati() {
        assert_hide_fg(
            map_command("ClrTati", &cmd("ClrTati", vec![]), &GameConfig::default()),
            "all",
        );
    }

    #[test]
    fn test_map_bgm_play() {
        let c = cmd("BgmPlay", vec![("1", "bgm_0101")]);
        let r = map_command("BgmPlay", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::PlayBgm { ref id, .. } if id == "bgm_0101"));
    }

    #[test]
    fn test_map_bgm_stop() {
        let c = cmd("BgmStop", vec![("0", "bgm_0101")]);
        let r = map_command("BgmStop", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::StopBgm { id: None, .. }));
    }

    #[test]
    fn test_map_se_play() {
        let c = cmd("SEPlay", vec![("0", "se_001")]);
        let r = map_command("SEPlay", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::PlaySe { ref file, .. } if file == "se_001"));
    }

    #[test]
    fn test_map_return_main() {
        let c = cmd("return_main", vec![("0", "aiy00010")]);
        let r = map_command("return_main", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::Return));
    }

    #[test]
    fn test_map_fadefilm_as_clear_overlay() {
        let r = map_command("FadeFilm", &cmd("FadeFilm", vec![("0", "1500")]), &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::ClearOverlay { time } if *time == 1500));
    }

    #[test]
    fn test_map_msgon() {
        let r = map_command("msgon", &cmd("msgon", vec![]), &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::Window { show: true, .. }));
    }

    #[test]
    fn test_draw_sprite() {
        let c = cmd("DrawSprite", vec![
            ("0", "01"), ("1", "sprite_01"), ("3", "100"), ("4", "200"),
            ("5", "50"), ("12", "255"), ("13", "10"), ("14", "300"),
        ]);
        let r = map_command("DrawSprite", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::DrawSprite { ref id, ref file, x, y, z, alpha, priority, time, rotation, anchor_x, anchor_y, blend_mode }
            if id == "01" && file == "sprite_01" && *x == 100.0 && *y == 200.0 && *z == 50 && *alpha == 255 && *priority == 10 && *time == 300
            && *rotation == 0.0 && *anchor_x == 0.5 && *anchor_y == 0.5 && *blend_mode == 0));
    }

    #[test]
    fn test_draw_sprite_with_transform() {
        let c = cmd("DrawSprite", vec![
            ("0", "fx_01"), ("1", "sparkle"), ("2", "1"), ("3", "400"), ("4", "300"),
            ("5", "10"), ("6", "45.0"), ("7", "0.5"), ("8", "0.0"), ("12", "200"),
            ("13", "5"), ("14", "500"),
        ]);
        let r = map_command("DrawSprite", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::DrawSprite { ref id, ref file, x, y, z, alpha, priority, time, rotation, anchor_x, anchor_y, blend_mode }
            if id == "fx_01" && file == "sparkle" && *x == 400.0 && *y == 300.0 && *z == 10
            && *alpha == 200 && *priority == 5 && *time == 500
            && *rotation == 45.0 && *anchor_x == 0.5 && *anchor_y == 0.0 && *blend_mode == 1));
    }

    #[test]
    fn test_fade_sprite() {
        let c = cmd("FadeSprite", vec![("0", "01"), ("1", "500")]);
        let r = map_command("FadeSprite", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::FadeSprite { ref id, time }
            if id == "01" && *time == 500));
    }

    #[test]
    fn test_move_sprite() {
        let c = cmd("MoveSprite", vec![
            ("0", "01"), ("1", "300"), ("2", "400"), ("3", "0"),
            ("5", "128"), ("8", "1000"), ("9", "TRUE"),
        ]);
        let r = map_command("MoveSprite", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::MoveSprite { ref id, x, y, z, alpha, time, wait }
            if id == "01" && *x == 300.0 && *y == 400.0 && *z == 0 && *alpha == 128 && *time == 1000 && *wait));
    }

    #[test]
    fn test_map_dialogue_sequence() {
        let asb = AsbScript {
            blocks: vec![crate::asb::AsbBlock {
                label: "main".into(),
                commands: vec![
                    cmd("name", vec![("0", "ナユタ")]),
                    cmd("print", vec![("data", "目が覚めたか。")]),
                    cmd("click", vec![]),
                    cmd("rp2", vec![]),
                    cmd("print", vec![("data", "調子はどうだ？")]),
                    cmd("click", vec![]),
                    cmd("rp2", vec![]),
                ],
            }],
        };
        let result = map_script(&asb, &GameConfig::default(), false);
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[0], ScriptCmd::Label { name } if name == "main"));
        assert!(
            matches!(&result[1], ScriptCmd::Dialogue { speaker: Some(s), text } if s == "ナユタ" && text == "目が覚めたか。")
        );
        assert!(
            matches!(&result[2], ScriptCmd::Dialogue { speaker: None, text } if text == "調子はどうだ？")
        );
    }

    #[test]
    fn test_map_script_emits_labels() {
        let asb = AsbScript {
            blocks: vec![crate::asb::AsbBlock {
                label: "main".into(),
                commands: vec![],
            }],
        };
        let result = map_script(&asb, &GameConfig::default(), false);
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], ScriptCmd::Label { name } if name == "main"));
    }

    #[test]
    fn test_map_calllua_set_bg() {
        let c = cmd("calllua", vec![("function", "set_bg"), ("file", "bg_0001")]);
        let result = map_command("calllua", &c, &GameConfig::default());
        assert!(result.is_some());
        let cmds = result.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::SetBg { ref file, .. } if file == "bg_0001"));
    }

    #[test]
    fn test_map_blackout() {
        let cmd = cmd("Blackout", vec![("0", "1500")]);
        let result = map_command("Blackout", &cmd, &GameConfig::default());
        assert!(result.is_some());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            ScriptCmd::ScreenOverlay { color, time } => {
                assert!(matches!(color, OverlayColor::Black));
                assert_eq!(*time, 1500);
            }
            _ => panic!("Expected ScreenOverlay"),
        }
    }

    #[test]
    fn test_map_fadeout_black() {
        let cmd = cmd("Fadeout", vec![("0", "BLACK"), ("1", "FAST")]);
        let result = map_command("Fadeout", &cmd, &GameConfig::default());
        assert!(result.is_some());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 4);
        assert!(matches!(&commands[0], ScriptCmd::ScreenOverlay { .. }));
        assert!(matches!(&commands[1], ScriptCmd::Wait { .. }));
        assert!(matches!(&commands[2], ScriptCmd::SetBg { .. }));
        assert!(matches!(&commands[3], ScriptCmd::ClearOverlay { .. }));
    }

    #[test]
    fn test_map_window_off() {
        let cmd = cmd("Window", vec![("0", "OFF")]);
        let result = map_command("Window", &cmd, &GameConfig::default());
        assert!(result.is_some());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            ScriptCmd::Window { show, .. } => assert!(!show),
            _ => panic!("Expected Window"),
        }
    }

    #[test]
    fn test_map_change_window_color() {
        let cmd = cmd("ChangeWindowColor", vec![("0", "2")]);
        let result = map_command("ChangeWindowColor", &cmd, &GameConfig::default());
        assert!(result.is_some());
        let commands = result.unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            ScriptCmd::ChangeWindowColor { color_idx } => assert_eq!(*color_idx, 2),
            _ => panic!("Expected ChangeWindowColor"),
        }
    }

    // ── Choice / exswitch tests ──

    fn choice_script(
        blocks: Vec<Vec<(&str, Vec<(&str, &str)>)>>,
    ) -> Script {
        let asb = AsbScript {
            blocks: blocks.into_iter().map(|cmds| crate::asb::AsbBlock {
                label: "main".into(),
                commands: cmds.into_iter().map(|(tag, attrs)| cmd(tag, attrs)).collect(),
            }).collect(),
        };
        map_script(&asb, &GameConfig::default(), false)
    }

    #[test]
    fn test_choice_basic_two_options() {
        let result = choice_script(vec![vec![
            ("sel_init", vec![]),
            ("sel_text", vec![("text", "选项A"), ("exp", "t.ens:0")]),
            ("sel_text", vec![("text", "选项B"), ("exp", "t.ens:1")]),
            ("select", vec![]),
        ]]);
        let choice_idx = result.iter().position(|c| matches!(c, ScriptCmd::Choice { .. }))
            .expect("Should have a Choice command");
        if let ScriptCmd::Choice { options } = &result[choice_idx] {
            assert_eq!(options.len(), 2);
            assert_eq!(options[0].text, "选项A");
            assert_eq!(options[1].text, "选项B");
            assert!(options[0].goto.is_none());
            assert!(options[1].goto.is_none());
        } else {
            panic!("Expected Choice");
        }
    }

    #[test]
    fn test_choice_ignores_sel_init_select() {
        let result = choice_script(vec![vec![
            ("sel_init", vec![]),
            ("sel_text", vec![("text", "选项")]),
            ("select", vec![]),
            ("Select", vec![]),
        ]]);
        let choice_count = result.iter().filter(|c| matches!(c, ScriptCmd::Choice { .. })).count();
        assert_eq!(choice_count, 1);
        // sel_init, Select should not appear in output
        let labels = result.iter().filter(|c| matches!(c, ScriptCmd::Label { .. }));
        assert!(labels.count() >= 1);
    }

    #[test]
    fn test_choice_with_exswitch_goto() {
        let result = choice_script(vec![vec![
            ("sel_init", vec![]),
            ("sel_text", vec![("text", "行く"), ("exp", "t.ens:0")]),
            ("sel_text", vec![("text", "止まる"), ("exp", "t.ens:1")]),
            ("select", vec![]),
            ("Select", vec![]),
            ("exswitch", vec![("data", "t.tmp<>0:route_a<>1:route_b<>default:route_default")]),
        ]]);
        let choice_idx = result.iter().position(|c| matches!(c, ScriptCmd::Choice { .. }))
            .expect("Should have a Choice command");
        if let ScriptCmd::Choice { options } = &result[choice_idx] {
            assert_eq!(options.len(), 2);
            assert_eq!(options[0].goto.as_deref(), Some("route_a"));
            assert_eq!(options[1].goto.as_deref(), Some("route_b"));
        } else {
            panic!("Expected Choice");
        }
    }

    #[test]
    fn test_choice_exswitch_default_fallback() {
        let result = choice_script(vec![vec![
            ("sel_init", vec![]),
            ("sel_text", vec![("text", "X"), ("exp", "t.ens:0")]),
            ("sel_text", vec![("text", "Y"), ("exp", "t.ens:2")]),
            ("select", vec![]),
            ("Select", vec![]),
            ("exswitch", vec![("data", "t.tmp<>0:zero_path<>default:fallback")]),
        ]]);
        let choice_idx = result.iter().position(|c| matches!(c, ScriptCmd::Choice { .. }))
            .expect("Should have a Choice command");
        if let ScriptCmd::Choice { options } = &result[choice_idx] {
            assert_eq!(options.len(), 2);
            assert_eq!(options[0].goto.as_deref(), Some("zero_path"));
            assert_eq!(options[1].goto.as_deref(), Some("fallback"));
        } else {
            panic!("Expected Choice");
        }
    }

    #[test]
    fn test_choice_exswitch_cross_block() {
        let asb = AsbScript {
            blocks: vec![
                crate::asb::AsbBlock {
                    label: "main".into(),
                    commands: vec![
                        cmd("sel_init", vec![]),
                        cmd("sel_text", vec![("text", "是"), ("exp", "t.ens:0")]),
                        cmd("sel_text", vec![("text", "否"), ("exp", "t.ens:1")]),
                        cmd("select", vec![]),
                    ],
                },
                crate::asb::AsbBlock {
                    label: "SelectItem000001".into(),
                    commands: vec![
                        cmd("Select", vec![]),
                        cmd("exswitch", vec![("data", "t.tmp<>0:yes_branch<>1:no_branch<>default:common")]),
                    ],
                },
            ],
        };
        let result = map_script(&asb, &GameConfig::default(), false);
        let choice_idx = result.iter().position(|c| matches!(c, ScriptCmd::Choice { .. }))
            .expect("Should have a Choice command");
        if let ScriptCmd::Choice { options } = &result[choice_idx] {
            assert_eq!(options.len(), 2);
            assert_eq!(options[0].goto.as_deref(), Some("yes_branch"));
            assert_eq!(options[1].goto.as_deref(), Some("no_branch"));
        } else {
            panic!("Expected Choice");
        }
    }

    #[test]
    fn test_choice_no_exswitch() {
        let result = choice_script(vec![vec![
            ("sel_init", vec![]),
            ("sel_text", vec![("text", "继续"), ("exp", "t.ens:0")]),
            ("select", vec![]),
            // No exswitch — goto remains None
        ]]);
        let choice_idx = result.iter().position(|c| matches!(c, ScriptCmd::Choice { .. }))
            .expect("Should have a Choice command");
        if let ScriptCmd::Choice { options } = &result[choice_idx] {
            assert_eq!(options.len(), 1);
            assert!(options[0].goto.is_none());
        } else {
            panic!("Expected Choice");
        }
    }

    #[test]
    fn test_choice_empty_exp_values() {
        let result = choice_script(vec![vec![
            ("sel_init", vec![]),
            ("sel_text", vec![("text", "A")]),  // no exp attribute
            ("sel_text", vec![("text", "B")]),  // no exp attribute
            ("select", vec![]),
            ("Select", vec![]),
            ("exswitch", vec![("data", "t.tmp<>0:branch_0<>1:branch_1<>default:fallback")]),
        ]]);
        let choice_idx = result.iter().position(|c| matches!(c, ScriptCmd::Choice { .. }))
            .expect("Should have a Choice command");
        if let ScriptCmd::Choice { options } = &result[choice_idx] {
            assert_eq!(options.len(), 2);
            // Without exp values, fall back to index-based mapping
            assert_eq!(options[0].goto.as_deref(), Some("branch_0"));
            assert_eq!(options[1].goto.as_deref(), Some("branch_1"));
        } else {
            panic!("Expected Choice");
        }
    }

    // ── Local work / flag map_command tests ──

    #[test]
    fn test_map_store_value_to_local_work() {
        let c = cmd("StoreValueToLocalWork", vec![("0", "1"), ("1", "5")]);
        let r = map_command("StoreValueToLocalWork", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::StoreValueToLocalWork { index, value, expression }
            if *index == 1 && *value == 5 && expression.is_none()));
    }

    #[test]
    fn test_map_store_value_to_local_work_expression() {
        let c = cmd("StoreValueToLocalWork", vec![("0", "1"), ("1", "t.tmp+1")]);
        let r = map_command("StoreValueToLocalWork", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::StoreValueToLocalWork { index, value, expression }
            if *index == 1 && *value == 0 && expression.as_deref() == Some("t.tmp+1")));
    }

    #[test]
    fn test_map_store_value_to_local_work_expression_plus2() {
        let c = cmd("StoreValueToLocalWork", vec![("0", "4"), ("1", "t.tmp+2")]);
        let r = map_command("StoreValueToLocalWork", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::StoreValueToLocalWork { index, value, expression }
            if *index == 4 && *value == 0 && expression.as_deref() == Some("t.tmp+2")));
    }

    #[test]
    fn test_map_load_value_from_local_work() {
        let c = cmd("LoadValueFromLocalWork", vec![("0", "3")]);
        let r = map_command("LoadValueFromLocalWork", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::LoadValueFromLocalWork { index }
            if *index == 3));
    }

    #[test]
    fn test_map_get_local_flag() {
        let c = cmd("GetLocalFlag", vec![("0", "42")]);
        let r = map_command("GetLocalFlag", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::GetLocalFlag { index }
            if *index == 42));
    }

    #[test]
    fn test_map_set_local_flag() {
        let c = cmd("SetLocalFlag", vec![("0", "7"), ("1", "1")]);
        let r = map_command("SetLocalFlag", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::SetLocalFlag { index, value }
            if *index == 7 && *value == 1));
    }

    #[test]
    fn test_map_get_global_flag() {
        let c = cmd("GetGlobalFlag", vec![("0", "51")]);
        let r = map_command("GetGlobalFlag", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::GetGlobalFlag { index }
            if *index == 51));
    }

    #[test]
    fn test_map_save_point() {
        let c = cmd("SavePoint", vec![]);
        let r = map_command("SavePoint", &c, &GameConfig::default());
        assert!(r.is_some());
        let cmds = r.unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(&cmds[0], ScriptCmd::SavePoint));
    }

    #[test]
    fn test_map_refresh_is_ignored() {
        let c = cmd("Refresh", vec![]);
        let r = map_command("Refresh", &c, &GameConfig::default());
        assert!(r.is_none());
    }
}
