use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct GameConfig {
    pub cg_sets: HashMap<String, Vec<String>>,
    pub bgm_files: HashMap<String, String>,
    pub scene_jumps: HashMap<String, Vec<SceneEntry>>,
    pub fg_paths: HashMap<String, String>,
    pub init_paths: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SceneEntry {
    pub file: String,
    pub label: String,
}

pub fn extract_config(root: &std::path::Path) -> anyhow::Result<GameConfig> {
    let mut config = GameConfig::default();
    let csv_path = root.join("system/csv.lua");
    let csv_content = std::fs::read_to_string(&csv_path)?;

    extract_init_table(&csv_content, &mut config);
    extract_cg_table(&csv_content, &mut config);
    extract_bgm_table(&csv_content, &mut config);
    extract_scene_table(&csv_content, &mut config);
    extract_fg_table(&csv_content, &mut config);

    Ok(config)
}

fn extract_init_table(content: &str, config: &mut GameConfig) {
    let target_keys = [
        "bg_path", "ev_path", "fg_path", "bgm_path", "se_path",
        "voice_path", "thumbnail_path", "face_path",
    ];

    let Some(body) = extract_table_body_raw(content, "init =") else { return };
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.contains('=') { continue; }
        for key in &target_keys {
            let quoted = format!("\"{}\"", key);
            if trimmed.contains(&quoted) {
                if let Some(rhs) = trimmed.split('=').nth(1) {
                    let raw = rhs.trim().trim_end_matches(',');
                    let path = extract_lua_value(raw);
                    if !path.is_empty() {
                        config.init_paths.insert(key.to_string(), path);
                    }
                }
            }
        }
    }
}

fn extract_cg_table(content: &str, config: &mut GameConfig) {
    let Some(body) = extract_table_body(content, "csv.extra_cgmode =") else { return };
    for entry in split_table_entries(&body) {
        if let Some((key, values)) = parse_key_value_table(&entry) {
            let files: Vec<String> = values
                .into_iter()
                .filter_map(|v| {
                    let s = v.trim();
                    if s.is_empty() || s == "\"\"" || s.parse::<i32>().is_ok() {
                        None
                    } else {
                        let cleaned = extract_lua_value(s);
                        if cleaned.is_empty() { None } else { Some(cleaned) }
                    }
                })
                .collect();
            if !files.is_empty() {
                config.cg_sets.insert(key, files);
            }
        }
    }
}

fn extract_bgm_table(content: &str, config: &mut GameConfig) {
    let Some(body) = extract_table_body(content, "csv.extra_bgm =") else { return };
    for entry in split_table_entries(&body) {
        if let Some((key, values)) = parse_key_value_table(&entry) {
            if values.len() >= 2 {
                let ogg = extract_lua_value(values[1].trim());
                if !ogg.is_empty() {
                    config.bgm_files.insert(key, ogg);
                }
            }
        }
    }
}

fn extract_scene_table(content: &str, config: &mut GameConfig) {
    let Some(body) = extract_table_body(content, "csv.extra_event =") else { return };
    for entry in split_table_entries(&body) {
        if let Some((key, values)) = parse_key_value_table(&entry) {
            if values.len() >= 3 {
                let file_num = values[2].trim().to_string();
                let label = if values.len() >= 4 {
                    let raw = extract_lua_value(values[3].trim());
                    if raw.is_empty() { String::new() } else { raw }
                } else {
                    String::new()
                };
                if !file_num.is_empty() && file_num != "\"\"" {
                    let num = file_num.parse::<i32>().unwrap_or(0);
                    config.scene_jumps.entry(key).or_default().push(SceneEntry {
                        file: format!("{:05}", num),
                        label,
                    });
                }
            }
        }
    }
}

fn extract_fg_table(content: &str, config: &mut GameConfig) {
    let Some(body) = extract_table_body(content, "csv.fg =") else { return };
    for entry in split_table_entries(&body) {
        if let Some((key, values)) = parse_key_value_table(&entry) {
            if !values.is_empty() {
                let dir = extract_lua_value(values[0].trim());
                if !dir.is_empty() && dir.ends_with('/') {
                    let dir_trimmed = dir.trim_end_matches('/').to_string();
                    config.fg_paths.insert(key, dir_trimmed);
                }
            }
        }
    }
}

fn extract_lua_value(s: &str) -> String {
    let s = s.trim();
    if s.starts_with("[[") {
        let inner = s.trim_start_matches("[[").trim_end_matches("]]");
        inner.to_string()
    } else if s.starts_with('"') {
        let inner = s.trim_start_matches('"').trim_end_matches('"');
        inner.to_string()
    } else {
        s.to_string()
    }
}

fn extract_table_body(content: &str, marker: &str) -> Option<String> {
    let start = content.find(marker)?;
    let after = &content[start + marker.len()..];
    let brace_start = after.find('{')?;
    let mut depth = 1i32;
    let mut in_long = false;
    let chars: Vec<char> = after.chars().collect();
    let mut i = brace_start + 1;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '[' && i + 1 < chars.len() && chars[i + 1] == '[' && !in_long {
            in_long = true;
            i += 2;
            continue;
        }
        if ch == ']' && i + 1 < chars.len() && chars[i + 1] == ']' && in_long {
            in_long = false;
            i += 2;
            continue;
        }
        if in_long {
            i += 1;
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(after[brace_start + 1..i].to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn extract_table_body_raw(content: &str, marker: &str) -> Option<String> {
    extract_table_body(content, marker)
}

fn split_table_entries(body: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut in_long = false;
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '[' && i + 1 < chars.len() && chars[i + 1] == '[' && !in_long {
            in_long = true;
            i += 2;
            continue;
        }
        if ch == ']' && i + 1 < chars.len() && chars[i + 1] == ']' && in_long {
            in_long = false;
            i += 2;
            continue;
        }
        if in_long {
            i += 1;
            continue;
        }

        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                let entry = body[start..i].trim().to_string();
                if !entry.is_empty() {
                    entries.push(entry);
                }
                start = i + 1;
            }
            _ => {}
        }

        i += 1;
    }

    let remaining = body[start..].trim().to_string();
    if !remaining.is_empty() {
        entries.push(remaining);
    }

    entries
}

fn parse_key_value_table(entry: &str) -> Option<(String, Vec<String>)> {
    let entry = entry.trim();
    let eq_pos = entry.find('=')?;
    let key_part = entry[..eq_pos].trim();
    let rest = entry[eq_pos + 1..].trim();

    let key = if key_part.starts_with('[') && key_part.ends_with(']') {
        let inner = &key_part[1..key_part.len() - 1];
        if inner.starts_with('"') && inner.ends_with('"') {
            inner[1..inner.len() - 1].to_string()
        } else if inner.starts_with("[[") && inner.ends_with("]]") {
            inner[2..inner.len() - 2].to_string()
        } else {
            return None;
        }
    } else {
        return None;
    };

    if !rest.starts_with('{') || !rest.ends_with('}') {
        return None;
    }

    let inner = &rest[1..rest.len() - 1];
    let values = split_top_level(inner);
    Some((key, values))
}

fn split_top_level(s: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut in_long = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '[' && i + 1 < chars.len() && chars[i + 1] == '[' && !in_long {
            in_long = true;
            i += 2;
            continue;
        }
        if ch == ']' && i + 1 < chars.len() && chars[i + 1] == ']' && in_long {
            in_long = false;
            i += 2;
            continue;
        }
        if in_long {
            i += 1;
            continue;
        }

        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                let item = s[start..i].trim().to_string();
                if !item.is_empty() && item != "\"\"" {
                    items.push(item);
                }
                start = i + 1;
            }
            _ => {}
        }

        i += 1;
    }

    let last = s[start..].trim().to_string();
    if !last.is_empty() && last != "\"\"" {
        items.push(last);
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_lua_value_quoted() {
        assert_eq!(extract_lua_value(r#""hello""#), "hello");
    }

    #[test]
    fn test_extract_lua_value_long_bracket() {
        assert_eq!(extract_lua_value("[[hello]]"), "hello");
        assert_eq!(extract_lua_value("[[001_eus/]]"), "001_eus/");
    }

    #[test]
    fn test_extract_cg_set() {
        let content = r#"
            csv.extra_cgmode = {
                ["eve_0101"] = { 1, 1, [[eve_010101]], [[eve_010102]], [[eve_010103]], "", },
            }
        "#;
        let mut config = GameConfig::default();
        extract_cg_table(content, &mut config);
        assert!(config.cg_sets.contains_key("eve_0101"));
        assert_eq!(config.cg_sets["eve_0101"], vec!["eve_010101", "eve_010102", "eve_010103"]);
    }

    #[test]
    fn test_extract_bgm_entry() {
        let content = r#"
            csv.extra_bgm = {
                ["bgm_0101"] = { 1, [[bgm_0101_a]], [[bgm_0101]], 135000, },
            }
        "#;
        let mut config = GameConfig::default();
        extract_bgm_table(content, &mut config);
        assert_eq!(config.bgm_files.get("bgm_0101").unwrap(), "bgm_0101_a");
    }

    #[test]
    fn test_extract_scene_entry() {
        let content = r#"
            csv.extra_event = {
                ["201"] = { 1, 1, 50220, [[scene]], "", },
            }
        "#;
        let mut config = GameConfig::default();
        extract_scene_table(content, &mut config);
        let jumps = config.scene_jumps.get("201").unwrap();
        assert_eq!(jumps[0].file, "50220");
        assert_eq!(jumps[0].label, "scene");
    }

    #[test]
    fn test_extract_fg_entry() {
        let content = r#"
            csv.fg = {
                ["01"] = { [[001_eus/]], },
                ["02"] = { [[002_eri/]], },
            }
        "#;
        let mut config = GameConfig::default();
        extract_fg_table(content, &mut config);
        assert_eq!(config.fg_paths.get("01").unwrap(), "001_eus");
        assert_eq!(config.fg_paths.get("02").unwrap(), "002_eri");
    }

    #[test]
    fn test_extract_init_paths() {
        let content = r#"
            init = {
                ["bg_path"] = "image/bg/",
                ["fg_path"] = "image/fg/",
                ["bgm_path"] = "bgm/",
            }
        "#;
        let mut config = GameConfig::default();
        extract_init_table(content, &mut config);
        assert_eq!(config.init_paths.get("bg_path").unwrap(), "image/bg/");
        assert_eq!(config.init_paths.get("fg_path").unwrap(), "image/fg/");
        assert_eq!(config.init_paths.get("bgm_path").unwrap(), "bgm/");
    }

    #[test]
    fn test_extract_multiple_cg_sets() {
        let content = r#"
            csv.extra_cgmode = {
                ["eve_0101"] = { 1, 1, [[eve_010101]], [[eve_010102]], },
                ["eve_0102"] = { 1, 2, [[eve_010201]], },
                ["hcg_0101"] = { 1, 3, [[hcg_010101]], [[hcg_010102]], [[hcg_010103]], },
            }
        "#;
        let mut config = GameConfig::default();
        extract_cg_table(content, &mut config);
        assert_eq!(config.cg_sets.len(), 3);
        assert_eq!(config.cg_sets["eve_0101"].len(), 2);
        assert_eq!(config.cg_sets["eve_0102"].len(), 1);
        assert_eq!(config.cg_sets["hcg_0101"].len(), 3);
    }

    #[test]
    fn test_skip_numeric_key_in_extra_event() {
        let content = r#"
            csv.extra_event = {
                [1] = "image/image/ui/title/",
                ["201"] = { 1, 1, 50220, "", "", },
            }
        "#;
        let mut config = GameConfig::default();
        extract_scene_table(content, &mut config);
        assert!(config.scene_jumps.contains_key("201"));
        assert_eq!(config.scene_jumps.len(), 1);
    }
}
