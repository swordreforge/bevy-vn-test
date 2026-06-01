use anyhow::{bail, Context, Result};
use bevy_vn::script::{ChoiceOption, ConditionOp, FgPosition, Script, ScriptCmd};
use std::collections::HashMap;
use std::path::Path;

pub fn parse_iet(path: &Path, verbose: bool) -> Result<Script> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read .iet file: {:?}", path))?;
    parse_iet_content(&text, verbose)
}

fn parse_iet_content(text: &str, verbose: bool) -> Result<Script> {
    let mut output = Script::new();
    let mut if_stack: Vec<IfFrame> = Vec::new();
    let mut if_counter: u32 = 0;

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with("//") || line.starts_with("/*") || line.starts_with("*/") || line.starts_with('*') && !line.starts_with("*/") && is_label_line(line) {
            if is_label_line(line) {
                let label_name = line.trim_start_matches('*').trim();
                if !label_name.is_empty() {
                    output.push(ScriptCmd::Label {
                        name: label_name.to_string(),
                    });
                }
            }
            continue;
        }

        if line.starts_with("/*") || line.starts_with("//") {
            continue;
        }

        if !line.starts_with('[') {
            continue;
        }

        if !line.ends_with(']') {
            continue;
        }

        let inner = &line[1..line.len() - 1];
        let (cmd_name, rest) = split_cmd(inner);

        match cmd_name {
            "CallScript" => {
                let target = rest.trim();
                output.push(ScriptCmd::CallScript {
                    script: target.to_string(),
                    label: None,
                });
            }
            "StoreValueToLocalWork" => {
                let (idx_str, val_str) = split_two_args(rest);
                let index = idx_str.trim().parse::<u32>().context("Invalid StoreValueToLocalWork index")?;
                let raw = val_str.trim();
                let (value, expression) = if raw.starts_with("t.tmp") {
                    (0, Some(raw.to_string()))
                } else {
                    (raw.parse::<i32>().context("Invalid StoreValueToLocalWork value")?, None)
                };
                output.push(ScriptCmd::StoreValueToLocalWork { index, value, expression });
            }
            "LoadValueFromLocalWork" => {
                let idx_str = rest.trim();
                let index = idx_str.parse::<u32>().context("Invalid LoadValueFromLocalWork index")?;
                output.push(ScriptCmd::LoadValueFromLocalWork { index });
            }
            "GetLocalFlag" => {
                let idx_str = rest.trim();
                let index = idx_str.parse::<u32>().context("Invalid GetLocalFlag index")?;
                output.push(ScriptCmd::GetLocalFlag { index });
            }
            "SetGlobalFlag" => {
                let (idx_str, val_str) = split_two_args(rest);
                if let (Ok(index), Ok(value)) = (idx_str.trim().parse::<u32>(), val_str.trim().parse::<i32>()) {
                    output.push(ScriptCmd::SetGlobalFlag { index, value });
                }
            }
            "TerminateExecutionOfScript" | "stop" => {
                output.push(ScriptCmd::Halt);
            }
            "SetValidityOfLoading" | "SetValidityOfSaving" => {}
            "return" => {
                output.push(ScriptCmd::Return);
            }
            "jump" => {
                let attrs = parse_attrs(rest);
                if let Some(target) = attrs.get("label").or_else(|| attrs.get("target")) {
                    output.push(ScriptCmd::Jump {
                        target: target.to_string(),
                    });
                } else if let Some(pos_target) = rest.trim().split_whitespace().next() {
                    if !pos_target.is_empty() && !pos_target.contains('=') {
                        output.push(ScriptCmd::Jump {
                            target: pos_target.to_string(),
                        });
                    }
                }
            }
            "call" => {
                let target = rest.trim();
                if !target.is_empty() && !target.contains('=') {
                    output.push(ScriptCmd::Call {
                        target: target.to_string(),
                    });
                }
            }
            "wait" | "wt" | "wt0" => {
                let attrs = parse_attrs(rest);
                let duration = attrs.get("time")
                    .and_then(|s| s.trim_start_matches("$t.").parse::<u64>().ok())
                    .or_else(|| {
                        let first = rest.trim().split_whitespace().next()?;
                        first.parse::<u64>().ok()
                    })
                    .unwrap_or(if cmd_name == "wt0" { 0 } else { 500 });
                output.push(ScriptCmd::Wait { duration });
            }
            "trans" => {
                // transition: skip for now (visual effect, no engine equivalent)
            }
            "sys_trans" => {}
            "calllua" => {
                let attrs = parse_attrs(rest);
                if let Some(cmds) = map_calllua_iet(&attrs, verbose) {
                    output.extend(cmds);
                } else if verbose {
                    let func = attrs.get("function").map(|s| s.as_str()).unwrap_or("?");
                    eprintln!("  [iet] unknown calllua: {}", func);
                }
            }
            "var" => {}
            "yesno" => {}
            "key_start" | "allkeystart" | "allkeystop" | "key_kill" => {}
            "msg_show" => {
                output.push(ScriptCmd::Window { show: true, time: None });
            }
            "btn_start" | "btn_attack" | "btnstat" | "btnoff" | "btnon" => {}
            "lyc" | "lydel" | "lyevent" => {}
            "quicksave" => {
                output.push(ScriptCmd::SavePoint);
            }
            "save_on" | "save_off" => {}
            "if" => {
                let id = if_counter;
                if_counter += 1;

                let estimate = extract_estimate(rest).context("Failed to parse if condition")?;

                let (negated_op, value) = match parse_condition_ext(&estimate) {
                    Ok(r) => r,
                    Err(e) => {
                        if verbose {
                            eprintln!("  [iet] warning: {}", e);
                        }
                        continue;
                    }
                };

                let else_label = format!("iet_else_{}", id);
                let endif_label = format!("iet_endif_{}", id);

                if_stack.push(IfFrame {
                    has_else: false,
                    else_label: else_label.clone(),
                    endif_label: endif_label.clone(),
                });

                output.push(ScriptCmd::Condition {
                    var: "tmp".to_string(),
                    value,
                    operator: negated_op,
                    goto: else_label,
                });
            }
            "else" => {
                let frame = if_stack.last_mut().context("else without if")?;
                frame.has_else = true;
                output.push(ScriptCmd::Jump {
                    target: frame.endif_label.clone(),
                });
                output.push(ScriptCmd::Label {
                    name: frame.else_label.clone(),
                });
            }
            "/if" => {
                let frame = if_stack.pop().context("/if without if")?;
                if !frame.has_else {
                    output.push(ScriptCmd::Label {
                        name: frame.else_label.clone(),
                    });
                }
                output.push(ScriptCmd::Label {
                    name: frame.endif_label.clone(),
                });
            }
            // === Phase 3: IET unmapped commands ===
            "SEStop" => {
                output.push(ScriptCmd::StopAllSe);
            }
            "RegisterTextToHistory" => {
                output.push(ScriptCmd::PushHistory);
            }
            "WaitToFinishVoicePlaying" => {
                output.push(ScriptCmd::WaitVoice);
            }
            "GetExecutionMode" => {
                let mode = parse_iet_attr(rest, 0).unwrap_or_default();
                output.push(ScriptCmd::QueryMode { mode });
            }
            "exif" => {
                let exp = parse_iet_named_attr(rest, "exp").unwrap_or_default();
                output.push(ScriptCmd::Exif { expression: exp });
            }
            "DrawBG" => {
                let file = parse_iet_attr(rest, 0).unwrap_or_default();
                output.push(ScriptCmd::SetBg { file, transition: None, duration: None });
            }
            "blur_set" => {
                let power = parse_iet_attr(rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                output.push(ScriptCmd::Blur { power });
            }
            "ChangeVolumeOfBGM" => {
                let vol = parse_iet_attr(rest, 0).unwrap_or_else(|| "100".to_string());
                output.push(ScriptCmd::BgmVol { channel: 1, volume: vol });
            }
            "FadeOutBGM" => {
                output.push(ScriptCmd::StopBgm { id: None, fade_out: None });
            }
            "ChangeVolumeOfStreamingSE" => {
                let id = parse_iet_attr(rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                let vol = parse_iet_attr(rest, 1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(100);
                output.push(ScriptCmd::StreamingSeVol { id, volume: vol });
            }
            "FadeOutStreamingSE" => {
                let channel = parse_iet_attr(rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                output.push(ScriptCmd::StopStreamingSe { channel });
            }
            "SetColorOfMonologue" => {
                let color = parse_iet_attr(rest, 0).unwrap_or_default();
                output.push(ScriptCmd::MonologueColor { color });
            }
            "StartShakingOfAllObjects" | "ShakeScreenSx" => {
                let power = parse_iet_attr(rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                let time = parse_iet_attr(rest, 1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                output.push(ScriptCmd::ShakeScreen { power, time });
            }
            "StartShakingOfSprite" => {
                let id = parse_iet_attr(rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                let power = parse_iet_attr(rest, 1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                let time = parse_iet_attr(rest, 2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                output.push(ScriptCmd::ShakeSprite { id, power, time });
            }
            "TerminateShakingOfAllObjects" | "TerminateShakingOfSprite" => {
                output.push(ScriptCmd::ShakeScreen { power: 0, time: 0 });
            }
            "calllua" => {
                let first = parse_iet_attr(rest, 0).unwrap_or_default();
                if first.parse::<i32>().is_ok() {
                    output.push(ScriptCmd::NoOp { tag: format!("calllua ({})", first) });
                } else if verbose {
                    eprintln!("  [iet] skip: calllua ({})", rest);
                }
            }
            // Group 4 (IET) — remaining unknown IET commands as NoOp
            cmd_name => {
                if let Some(iet_func) = parse_iet_attr(rest, 0) {
                    if iet_func.parse::<i32>().is_ok() {
                        output.push(ScriptCmd::NoOp { tag: format!("{} ({})", cmd_name, iet_func) });
                    } else {
                        output.push(ScriptCmd::NoOp { tag: format!("{} {}", cmd_name, rest) });
                    }
                } else if verbose {
                    eprintln!("  [iet] skip: {} ({})", cmd_name, rest);
                }
            }
        }
    }

    if !if_stack.is_empty() {
        bail!("Unclosed if block(s) in .iet file");
    }

    Ok(output)
}

struct IfFrame {
    has_else: bool,
    else_label: String,
    endif_label: String,
}

fn is_label_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('*') && !trimmed.starts_with("*/") && trimmed.len() > 1
}

fn parse_iet_attr(rest: &str, index: usize) -> Option<String> {
    let parts: Vec<&str> = rest.trim().split_whitespace().collect();
    parts.get(index).map(|s| s.to_string())
}

fn parse_iet_named_attr(cmd_rest: &str, name: &str) -> Option<String> {
    let pattern = format!("{}=", name);
    for part in cmd_rest.split_whitespace() {
        if let Some(val) = part.strip_prefix(&pattern) {
            return Some(val.trim_matches('"').to_string());
        }
    }
    None
}

fn split_cmd(inner: &str) -> (&str, &str) {
    if let Some(pos) = inner.find(|c: char| c.is_whitespace()) {
        (&inner[..pos], &inner[pos..])
    } else {
        (inner, "")
    }
}

fn split_two_args(rest: &str) -> (&str, &str) {
    let rest = rest.trim();
    if let Some(pos) = rest.find(|c: char| c.is_whitespace()) {
        (&rest[..pos], &rest[pos..].trim_start())
    } else {
        (rest, "")
    }
}

fn extract_estimate(rest: &str) -> Option<String> {
    let rest = rest.trim();
    if !rest.starts_with("estimate=") {
        return None;
    }
    let val = rest.trim_start_matches("estimate=");
    let val = val.trim();
    if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
        Some(val[1..val.len() - 1].to_string())
    } else {
        Some(val.to_string())
    }
}

fn parse_attrs(rest: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let rest = rest.trim();
    let mut i = 0;
    let chars: Vec<char> = rest.chars().collect();
    while i < chars.len() {
        // skip whitespace
        while i < chars.len() && chars[i].is_whitespace() { i += 1; }
        if i >= chars.len() { break; }

        // check if it's key=value
        let start = i;
        while i < chars.len() && chars[i] != '=' && !chars[i].is_whitespace() { i += 1; }
        if i >= chars.len() || chars[i] != '=' {
            // positional arg (not key=value), skip
            while i < chars.len() && !chars[i].is_whitespace() { i += 1; }
            continue;
        }
        let key: String = chars[start..i].iter().collect();
        i += 1; // skip '='

        // skip whitespace before value
        while i < chars.len() && chars[i].is_whitespace() { i += 1; }
        if i >= chars.len() { break; }

        let value = if chars[i] == '"' {
            i += 1; // skip opening "
            let vstart = i;
            while i < chars.len() && chars[i] != '"' { i += 1; }
            let v: String = chars[vstart..i].iter().collect();
            if i < chars.len() { i += 1; } // skip closing "
            v
        } else if chars[i] == '\'' {
            i += 1;
            let vstart = i;
            while i < chars.len() && chars[i] != '\'' { i += 1; }
            let v: String = chars[vstart..i].iter().collect();
            if i < chars.len() { i += 1; }
            v
        } else {
            let vstart = i;
            while i < chars.len() && !chars[i].is_whitespace() { i += 1; }
            chars[vstart..i].iter().collect()
        };

        map.insert(key.to_string(), value);
    }
    map
}

fn map_calllua_iet(attrs: &HashMap<String, String>, verbose: bool) -> Option<Vec<ScriptCmd>> {
    let func = attrs.get("function")?;
    let f = func.as_str();

    if f.contains("choice") || f.contains("tags.choice") {
        let options = vec![ChoiceOption {
            text: format!("[choice in {}]", func),
            affection_change: None,
            goto: None,
        }];
        return Some(vec![ScriptCmd::Choice { options }]);
    }

    if f.contains("set_bg") || f.contains("setbg") {
        let file = attrs.get("file")?;
        return Some(vec![ScriptCmd::SetBg {
            file: file.to_string(),
            transition: None,
            duration: None,
        }]);
    }

    if f.contains("bgm_play") {
        let id = attrs.get("file")?;
        let volume = attrs.get("vol").and_then(|v| v.parse::<f32>().ok()).map(|v| v / 100.0);
        let fade_in = attrs.get("time").and_then(|t| t.parse::<u64>().ok());
        return Some(vec![ScriptCmd::PlayBgm { id: id.to_string(), volume, fade_in }]);
    }

    if f.contains("bgm_stop") {
        let id = attrs.get("file").map(|s| s.to_string());
        let fade_out = attrs.get("time").and_then(|t| t.parse::<u64>().ok());
        return Some(vec![ScriptCmd::StopBgm { id, fade_out }]);
    }

    if f.contains("se_play") {
        let file = attrs.get("file")?;
        let has_loop = attrs.get("loop").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0) != 0;
        if has_loop {
            let channel = attrs.get("id").and_then(|s| s.parse().ok()).unwrap_or(0);
            return Some(vec![ScriptCmd::LoopSe { file: file.to_string(), volume: None, channel }]);
        } else {
            return Some(vec![ScriptCmd::PlaySe { file: file.to_string(), volume: None }]);
        }
    }

    if f.contains("se_stop") {
        let channel = attrs.get("id").and_then(|s| s.parse().ok()).unwrap_or(0);
        return Some(vec![ScriptCmd::StopStreamingSe { channel }]);
    }

    if f.contains("voice_play") {
        let file = attrs.get("file")?;
        return Some(vec![ScriptCmd::PlayVoice { file: file.to_string() }]);
    }

    if f.contains("show_fg") || f.contains("fg_show") {
        let char_id = attrs.get("id")?;
        let expression = attrs.get("file").cloned().unwrap_or("000".into());
        return Some(vec![ScriptCmd::ShowFg {
            char_id: char_id.to_string(),
            expression,
            position: FgPosition::Center,
            transition: None,
        }]);
    }

    if f.contains("hide_fg") || f.contains("fg_hide") {
        let char_id = attrs.get("id").cloned().unwrap_or("all".into());
        return Some(vec![ScriptCmd::HideFg { char_id, transition: None }]);
    }

    if f.contains("show_cg") || f.contains("cg_show") {
        let file = attrs.get("file")?;
        return Some(vec![ScriptCmd::ShowCg { file: file.to_string(), transition: None }]);
    }

    if f.contains("hide_cg") || f.contains("cg_hide") {
        return Some(vec![ScriptCmd::HideCg { transition: None }]);
    }

    if f.contains("affection_change") || f.contains("affection") {
        let char_id = attrs.get("char_id").cloned().unwrap_or("default".into());
        let delta = attrs.get("delta").and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
        return Some(vec![ScriptCmd::AffectionChange { char_id, delta }]);
    }

    if f.contains("save_point") || f.contains("quicksave") {
        return Some(vec![ScriptCmd::SavePoint]);
    }

    if f.contains("wait_tag") || f.contains("tags.wt") || f.contains("tags.wtx") {
        let duration = attrs.get("time").and_then(|s| s.parse::<u64>().ok()).unwrap_or(500);
        return Some(vec![ScriptCmd::Wait { duration }]);
    }

    if f.contains("unlock_cg") || f.contains("cg_unlock") {
        let file = attrs.get("file").cloned().unwrap_or("unknown".into());
        return Some(vec![ScriptCmd::UnlockCg { file }]);
    }

    if f.contains("scroll_bg") || f.contains("ScrollBGenq") {
        let file = attrs.get("file").or_else(|| attrs.get("0")).cloned().unwrap_or_default();
        let x1 = attrs.get("1").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let y1 = attrs.get("2").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let x2 = attrs.get("4").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let y2 = attrs.get("5").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let fade = attrs.get("9").and_then(|s| s.parse().ok()).unwrap_or(0);
        let wait = attrs.get("10").map(|s| s == "TRUE").unwrap_or(false);
        return Some(vec![ScriptCmd::ScrollBg { file, x1, y1, x2, y2, fade, wait }]);
    }

    if f.contains("ChangeVolumeOfBGM") || f.contains("bgm_fade") {
        let channel = attrs.get("0").and_then(|s| s.parse().ok()).unwrap_or(0);
        let vol_val = attrs.get("1").and_then(|s| s.parse::<u32>().ok()).unwrap_or(80);
        let volume = match vol_val {
            0 => "MIN",
            30 => "LOW",
            80 => "NORM",
            _ => "HIGH",
        }.to_string();
        return Some(vec![ScriptCmd::BgmVol { channel, volume }]);
    }

    if verbose {
        eprintln!("  [iet] calllua: {} (unhandled)", func);
    }
    None
}

fn parse_condition_ext(expr: &str) -> Result<(ConditionOp, i32)> {
    let expr = expr.trim();
    let expr = expr.replace("$t.tmp", "tmp");

    // Handle OR conditions: take only the first condition
    let expr = if let Some(pos) = expr.find("||") {
        expr[..pos].trim().to_string()
    } else {
        expr
    };

    // Handle system status conditions: $s.status.xxx == Y
    if expr.contains("$s.") || expr.contains("s.status") {
        // These are system/runtime conditions; treat as always-true by
        // emitting a condition that never jumps (NotEqual on impossible value)
        return Ok((ConditionOp::NotEqual, -99999));
    }

    // Handle $t.xxx variables: treat as tmp
    let expr = expr.replace("$t.", "tmp.");

    let expr = expr.replace(" ", "");

    let (negated_op, value_str) = if let Some(rest) = expr.strip_prefix("tmp==") {
        (ConditionOp::NotEqual, rest.to_string())
    } else if let Some(rest) = expr.strip_prefix("tmp!=") {
        (ConditionOp::Equal, rest.to_string())
    } else if let Some(rest) = expr.strip_prefix("tmp>=") {
        (ConditionOp::Less, rest.to_string())
    } else if let Some(rest) = expr.strip_prefix("tmp<=") {
        (ConditionOp::Greater, rest.to_string())
    } else if let Some(rest) = expr.strip_prefix("tmp>") {
        (ConditionOp::LessEqual, rest.to_string())
    } else if let Some(rest) = expr.strip_prefix("tmp<") {
        (ConditionOp::GreaterEqual, rest.to_string())
    } else {
        bail!("Cannot parse condition: {}", expr);
    };

    let value = value_str.parse::<i32>()
        .context(format!("Invalid condition value: {}", value_str))?;
    Ok((negated_op, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_iet() {
        let input = r#"
*main
[CallScript aiy00010]
[CallScript aiy00020]
[TerminateExecutionOfScript]
"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Label { name } if name == "main"));
        assert!(matches!(&result[1], ScriptCmd::CallScript { script, .. } if script == "aiy00010"));
        assert!(matches!(&result[2], ScriptCmd::CallScript { script, .. } if script == "aiy00020"));
        assert!(matches!(&result[3], ScriptCmd::Halt));
    }

    #[test]
    fn test_parse_if_else() {
        let input = r#"
*main
[LoadValueFromLocalWork 9]
[if estimate="$t.tmp == 0"]
[CallScript aiy10230]
[TerminateExecutionOfScript]
[else]
[CallScript aiy10200]
[/if]
[CallScript aiy20010]
"#;
        let result = parse_iet_content(input, false).unwrap();

        let labels: Vec<&str> = result.iter().filter_map(|c| match c {
            ScriptCmd::Label { name } => Some(name.as_str()),
            _ => None,
        }).collect();
        assert!(labels.contains(&"iet_else_0"));
        assert!(labels.contains(&"iet_endif_0"));

        let has_condition = result.iter().any(|c| matches!(c, ScriptCmd::Condition { var, operator: ConditionOp::NotEqual, value: 0, goto } if var == "tmp" && goto == "iet_else_0"));
        assert!(has_condition, "Should have NotEqual condition jumping to else label");

        let has_jump_endif = result.iter().any(|c| matches!(c, ScriptCmd::Jump { target } if target == "iet_endif_0"));
        assert!(has_jump_endif, "Then block should jump to endif");
    }

    #[test]
    fn test_parse_if_without_else() {
        let input = r#"
*main
[GetLocalFlag 202]
[if estimate="$t.tmp != 0"]
[CallScript aiy20220]
[/if]
[CallScript aiy20230]
"#;
        let result = parse_iet_content(input, false).unwrap();

        let has_condition = result.iter().any(|c| matches!(c, ScriptCmd::Condition { operator: ConditionOp::Equal, value: 0, .. }));
        assert!(has_condition, "!= 0 should negate to Equal 0");
    }

    #[test]
    fn test_nested_if() {
        let input = r#"
*main
[GetLocalFlag 203]
[if estimate="$t.tmp != 0"]
[CallScript aiy30280]
[TerminateExecutionOfScript]
[else]
[GetLocalFlag 206]
[if estimate="$t.tmp != 0"]
[CallScript aiy30280a]
[TerminateExecutionOfScript]
[else]
[CallScript aiy30270]
[/if]
[/if]
"#;
        let result = parse_iet_content(input, false).unwrap();
        let labels: Vec<&str> = result.iter().filter_map(|c| match c {
            ScriptCmd::Label { name } => Some(name.as_str()),
            _ => None,
        }).collect();
        assert!(labels.contains(&"iet_else_0"));
        assert!(labels.contains(&"iet_endif_0"));
        assert!(labels.contains(&"iet_else_1"));
        assert!(labels.contains(&"iet_endif_1"));
    }

    #[test]
    fn test_store_load_localwork() {
        let input = r#"
*main
[StoreValueToLocalWork 1 0]
[StoreValueToLocalWork 4 0]
[LoadValueFromLocalWork 9]
"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(result.iter().any(|c| matches!(c, ScriptCmd::StoreValueToLocalWork { index: 1, value: 0, .. })));
        assert!(result.iter().any(|c| matches!(c, ScriptCmd::StoreValueToLocalWork { index: 4, value: 0, .. })));
        assert!(result.iter().any(|c| matches!(c, ScriptCmd::LoadValueFromLocalWork { index: 9 })));
    }

    #[test]
    fn test_condition_negation() {
        let (op, val) = parse_condition_ext("$t.tmp == 0").unwrap();
        assert!(matches!(op, ConditionOp::NotEqual));
        assert_eq!(val, 0);

        let (op, val) = parse_condition_ext("$t.tmp != 0").unwrap();
        assert!(matches!(op, ConditionOp::Equal));
        assert_eq!(val, 0);

        let (op, val) = parse_condition_ext("$t.tmp >= 3").unwrap();
        assert!(matches!(op, ConditionOp::Less));
        assert_eq!(val, 3);
    }

    #[test]
    fn test_return_command() {
        let input = "*main\n[return]\n";
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Label { name } if name == "main"));
        assert!(matches!(&result[1], ScriptCmd::Return));
    }

    #[test]
    fn test_jump_command() {
        let input = r#"[jump label="target_label"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Jump { target } if target == "target_label"));
    }

    #[test]
    fn test_stop_command() {
        let input = "[stop]";
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Halt));
    }

    #[test]
    fn test_wait_command() {
        let input = r#"[wait time="1000"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Wait { duration: 1000 }));
    }

    #[test]
    fn test_wt_commands() {
        let input = "[wt]\n[wt0]";
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Wait { duration: 500 }));
        assert!(matches!(&result[1], ScriptCmd::Wait { duration: 0 }));
    }

    #[test]
    fn test_calllua_choice() {
        let input = r#"[calllua function="tags.choice"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Choice { .. }));
    }

    #[test]
    fn test_system_if_condition() {
        let input = "[if estimate=\"$s.status.commandskip == 1\"]\n[CallScript aiy00010]\n[/if]";
        let result = parse_iet_content(input, false).unwrap();
        assert!(result.iter().any(|c| matches!(c, ScriptCmd::Condition { .. })));
    }

    #[test]
    fn test_or_condition() {
        let input = "[if estimate=\"$t.tmp == 0 || s.status.controlskip == 1\"]\n[CallScript aiy00010]\n[/if]";
        let result = parse_iet_content(input, false).unwrap();
        assert!(result.iter().any(|c| matches!(c, ScriptCmd::Condition { .. })));
    }

    #[test]
    fn test_calllua_set_bg() {
        let input = r#"[calllua function="set_bg" file="bg_0001"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::SetBg { file, .. } if file == "bg_0001"));
    }

    #[test]
    fn test_calllua_voice_play() {
        let input = r#"[calllua function="voice_play" file="aiy010000010"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::PlayVoice { file } if file == "aiy010000010"));
    }

    #[test]
    fn test_calllua_save_point() {
        let input = r#"[calllua function="quicksave"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::SavePoint));
    }

    #[test]
    fn test_calllua_affection() {
        let input = r#"[calllua function="affection_change" char_id="01" delta="1"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::AffectionChange { char_id, delta } if char_id == "01" && *delta == 1));
    }

    #[test]
    fn test_calllua_bgm_play() {
        let input = r#"[calllua function="bgm_play" file="bgm_0101" vol="80"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::PlayBgm { id, volume: Some(0.8), .. } if id == "bgm_0101"));
    }

    #[test]
    fn test_parse_attrs_quoted() {
        let attrs = parse_attrs(r#"function="hello" id="123" mode="enable""#);
        assert_eq!(attrs.get("function").unwrap(), "hello");
        assert_eq!(attrs.get("id").unwrap(), "123");
        assert_eq!(attrs.get("mode").unwrap(), "enable");
    }

    #[test]
    fn test_parse_attrs_mixed() {
        let attrs = parse_attrs(r#"id="1.120" mode="disable""#);
        assert_eq!(attrs.get("id").unwrap(), "1.120");
        assert_eq!(attrs.get("mode").unwrap(), "disable");
    }

    #[test]
    fn test_set_global_flag() {
        let input = "[SetGlobalFlag 55 1]";
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::SetGlobalFlag { index: 55, value: 1 }));
    }

    #[test]
    fn test_call_command() {
        let input = "[call some_label]";
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Call { target } if target == "some_label"));
    }

    #[test]
    fn test_calllua_hide_fg() {
        let input = r#"[calllua function="hide_fg" id="350101"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::HideFg { char_id, .. } if char_id == "350101"));
    }

    #[test]
    fn test_calllua_wait_tag() {
        let input = r#"[calllua function="tags.wt" time="2000"]"#;
        let result = parse_iet_content(input, false).unwrap();
        assert!(matches!(&result[0], ScriptCmd::Wait { duration: 2000 }));
    }
}
