use crate::asb::{AsbCommand, AsbScript};
use bevy_vn::script::{ChoiceOption, FgPosition, OverlayColor, Script, ScriptCmd};

pub fn map_script(
    asb: &AsbScript,
    config: &crate::lua_config::GameConfig,
    verbose: bool,
) -> Script {
    let mut output = Script::new();
    let mut pending_speaker: Option<String> = None;

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
        "SEPlay" => {
            let file = cmd.attrs.get("0")?;
            Some(vec![ScriptCmd::PlaySe {
                file: file.to_string(),
                volume: None,
            }])
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
            Some(vec![ScriptCmd::PlaySe {
                file: file.to_string(),
                volume: None,
            }])
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
}
