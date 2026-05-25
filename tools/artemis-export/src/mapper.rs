use crate::asb::{AsbCommand, AsbScript};
use bevy_vn::script::{ChoiceOption, FgPosition, Script, ScriptCmd};

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
                    if let Some(sc) = map_command(tag, cmd, config) {
                        output.push(sc);
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
) -> Option<ScriptCmd> {
    match tag {
        "Wait" => {
            let duration = cmd
                .attrs
                .get("0")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1000);
            Some(ScriptCmd::Wait { duration })
        }
        "Voice" => {
            let file = cmd.attrs.get("0")?;
            Some(ScriptCmd::PlayVoice {
                file: file.to_string(),
            })
        }
        "TatiFa" => {
            let char_id = cmd.attrs.get("0")?;
            let expression = cmd.attrs.get("1").cloned().unwrap_or("000".into());
            Some(ScriptCmd::ShowFg {
                char_id: char_id.to_string(),
                expression,
                position: FgPosition::Center,
                transition: None,
            })
        }
        "Face" => {
            let char_id = cmd.attrs.get("0").cloned().unwrap_or("0".into());
            let expression = cmd.attrs.get("1").cloned().unwrap_or("000".into());
            Some(ScriptCmd::ShowFg {
                char_id,
                expression,
                position: FgPosition::Center,
                transition: None,
            })
        }
        "ClrFace" => {
            let char_id = cmd.attrs.get("0").cloned().unwrap_or("all".into());
            Some(ScriptCmd::HideFg {
                char_id,
                transition: None,
            })
        }
        "ClrTati" => {
            Some(ScriptCmd::HideFg {
                char_id: "all".into(),
                transition: None,
            })
        }
        "BgmPlay" => {
            let id = cmd.attrs.get("0")?;
            Some(ScriptCmd::PlayBgm {
                id: id.to_string(),
                volume: None,
                fade_in: None,
            })
        }
        "BgmStop" => {
            let id = cmd.attrs.get("0").map(|s| s.to_string());
            let fade_out = cmd.attrs.get("1").and_then(|s| s.parse::<u64>().ok());
            Some(ScriptCmd::StopBgm { id, fade_out })
        }
        "SEPlay" => {
            let file = cmd.attrs.get("0")?;
            Some(ScriptCmd::PlaySe {
                file: file.to_string(),
                volume: None,
            })
        }
        "FadeFilm" => {
            let duration = cmd
                .attrs
                .get("0")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(500);
            Some(ScriptCmd::Wait { duration })
        }
        "msgon" => {
            Some(ScriptCmd::Wait { duration: 300 })
        }
        "SetJumpLabel" => None,
        "return_main" => Some(ScriptCmd::Return),
        "CallScript" => {
            let target = cmd.attrs.get("0")?;
            let label = cmd.attrs.get("1").filter(|s| !s.is_empty()).cloned();
            let script = format!("aiy{:05}", target.parse::<u32>().ok()?);
            Some(ScriptCmd::CallScript { script, label })
        }
        "calllua" => map_calllua(cmd, _config),
        _ => None,
    }
}

fn map_calllua(cmd: &AsbCommand, _config: &crate::lua_config::GameConfig) -> Option<ScriptCmd> {
    let func = cmd.attrs.get("function")?;
    match func.as_str() {
        s if s.contains("set_bg") || s.contains("setbg") => {
            let file = cmd.attrs.get("file")?;
            Some(ScriptCmd::SetBg {
                file: file.to_string(),
                transition: None,
                duration: None,
            })
        }
        s if s.contains("bgm_play") => {
            let id = cmd.attrs.get("file")?;
            let volume = cmd.attrs.get("vol").and_then(|v| v.parse::<f32>().ok()).map(|v| v / 100.0);
            let fade_in = cmd.attrs.get("time").and_then(|t| t.parse::<u64>().ok());
            Some(ScriptCmd::PlayBgm {
                id: id.to_string(),
                volume,
                fade_in,
            })
        }
        s if s.contains("bgm_stop") => {
            let id = cmd.attrs.get("file").map(|s| s.to_string());
            let fade_out = cmd.attrs.get("time").and_then(|t| t.parse::<u64>().ok());
            Some(ScriptCmd::StopBgm { id, fade_out })
        }
        s if s.contains("se_play") => {
            let file = cmd.attrs.get("file")?;
            Some(ScriptCmd::PlaySe {
                file: file.to_string(),
                volume: None,
            })
        }
        s if s.contains("voice_play") => {
            let file = cmd.attrs.get("file")?;
            Some(ScriptCmd::PlayVoice {
                file: file.to_string(),
            })
        }
        s if s.contains("show_fg") || s.contains("fg_show") => {
            let char_id = cmd.attrs.get("id")?;
            let expression = cmd.attrs.get("file").cloned().unwrap_or("000".into());
            Some(ScriptCmd::ShowFg {
                char_id: char_id.to_string(),
                expression,
                position: FgPosition::Center,
                transition: None,
            })
        }
        s if s.contains("hide_fg") || s.contains("fg_hide") => {
            let char_id = cmd.attrs.get("id").cloned().unwrap_or("all".into());
            Some(ScriptCmd::HideFg {
                char_id,
                transition: None,
            })
        }
        s if s.contains("show_cg") || s.contains("cg_show") => {
            let file = cmd.attrs.get("file")?;
            Some(ScriptCmd::ShowCg {
                file: file.to_string(),
                transition: None,
            })
        }
        s if s.contains("hide_cg") || s.contains("cg_hide") => {
            Some(ScriptCmd::HideCg { transition: None })
        }
        s if s.contains("affection_change") || s.contains("affection") => {
            let char_id = cmd.attrs.get("char_id").cloned().unwrap_or("default".into());
            let delta = cmd.attrs.get("delta").and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
            Some(ScriptCmd::AffectionChange { char_id, delta })
        }
        s if s.contains("choice") || s.contains("tags.choice") => {
            Some(ScriptCmd::Choice {
                options: vec![ChoiceOption {
                    text: format!("[choice in {}]", func),
                    affection_change: None,
                    goto: None,
                }],
            })
        }
        s if s.contains("save_point") || s.contains("quicksave") => {
            Some(ScriptCmd::SavePoint)
        }
        s if s.contains("wait_tag") || s.contains("tags.wt") || s.contains("tags.wtx") => {
            let duration = cmd.attrs.get("time").and_then(|s| s.parse::<u64>().ok()).unwrap_or(500);
            Some(ScriptCmd::Wait { duration })
        }
        s if s.contains("unlock_cg") || s.contains("cg_unlock") => {
            let file = cmd.attrs.get("file").cloned().unwrap_or("unknown".into());
            Some(ScriptCmd::UnlockCg { file })
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

    fn assert_wait(result: Option<ScriptCmd>, expected: u64) {
        assert!(matches!(result, Some(ScriptCmd::Wait { duration: d }) if d == expected));
    }

    fn assert_hide_fg(result: Option<ScriptCmd>, expected: &str) {
        assert!(matches!(result, Some(ScriptCmd::HideFg { char_id: ref c, .. }) if c == expected));
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
        assert!(matches!(r, Some(ScriptCmd::PlayVoice { ref file }) if file == "vo_001"));
    }

    #[test]
    fn test_map_tatifa() {
        let c = cmd("TatiFa", vec![("0", "001_eus"), ("1", "010003")]);
        let r = map_command("TatiFa", &c, &GameConfig::default());
        assert!(matches!(r, Some(ScriptCmd::ShowFg { ref char_id, ref expression, .. })
            if char_id == "001_eus" && expression == "010003"));
    }

    #[test]
    fn test_map_face() {
        let c = cmd("Face", vec![("0", "001_eus"), ("1", "010002")]);
        let r = map_command("Face", &c, &GameConfig::default());
        assert!(matches!(r, Some(ScriptCmd::ShowFg { ref char_id, ref expression, .. })
            if char_id == "001_eus" && expression == "010002"));
    }

    #[test]
    fn test_map_clrface() {
        assert_hide_fg(
            map_command("ClrFace", &cmd("ClrFace", vec![("0", "001")]), &GameConfig::default()),
            "001",
        );
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
        let c = cmd("BgmPlay", vec![("0", "bgm_0101")]);
        let r = map_command("BgmPlay", &c, &GameConfig::default());
        assert!(matches!(r, Some(ScriptCmd::PlayBgm { ref id, .. }) if id == "bgm_0101"));
    }

    #[test]
    fn test_map_bgm_stop() {
        let c = cmd("BgmStop", vec![("0", "bgm_0101")]);
        let r = map_command("BgmStop", &c, &GameConfig::default());
        assert!(matches!(r, Some(ScriptCmd::StopBgm { id: Some(ref id), .. }) if id == "bgm_0101"));
    }

    #[test]
    fn test_map_se_play() {
        let c = cmd("SEPlay", vec![("0", "se_001")]);
        let r = map_command("SEPlay", &c, &GameConfig::default());
        assert!(matches!(r, Some(ScriptCmd::PlaySe { ref file, .. }) if file == "se_001"));
    }

    #[test]
    fn test_map_return_main() {
        let c = cmd("return_main", vec![("0", "aiy00010")]);
        assert!(matches!(map_command("return_main", &c, &GameConfig::default()), Some(ScriptCmd::Return)));
    }

    #[test]
    fn test_map_fadefilm_as_wait() {
        assert_wait(
            map_command("FadeFilm", &cmd("FadeFilm", vec![("0", "1500")]), &GameConfig::default()),
            1500,
        );
    }

    #[test]
    fn test_map_msgon() {
        assert_wait(
            map_command("msgon", &cmd("msgon", vec![]), &GameConfig::default()),
            300,
        );
    }

    #[test]
    fn test_skip_rendering_tag() {
        let c = cmd("DrawSprite", vec![("0", "01"), ("1", "sprite_01")]);
        assert!(map_command("DrawSprite", &c, &GameConfig::default()).is_none());
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
        assert!(matches!(result, Some(ScriptCmd::SetBg { ref file, .. }) if file == "bg_0001"));
    }
}
