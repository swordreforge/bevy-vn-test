use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AsbScript {
    pub blocks: Vec<AsbBlock>,
}

#[derive(Debug, Clone)]
pub struct AsbBlock {
    pub label: String,
    pub commands: Vec<AsbCommand>,
}

#[derive(Debug, Clone)]
pub struct AsbCommand {
    pub tag: String,
    pub attrs: HashMap<String, String>,
}

fn read_u32_le(data: &[u8], pos: &mut usize) -> Option<u32> {
    if *pos + 4 > data.len() {
        return None;
    }
    let val = u32::from_le_bytes([
        data[*pos],
        data[*pos + 1],
        data[*pos + 2],
        data[*pos + 3],
    ]);
    *pos += 4;
    Some(val)
}

fn read_string(data: &[u8], pos: &mut usize) -> Option<String> {
    let len = read_u32_le(data, pos)? as usize;
    if *pos + len + 1 > data.len() {
        return None;
    }
    let s = std::str::from_utf8(&data[*pos..*pos + len])
        .ok()?
        .to_string();
    *pos += len;
    if data[*pos] != 0 {
        return None;
    }
    *pos += 1;
    Some(s)
}

pub fn parse_asb(path: &Path) -> Result<AsbScript> {
    let data = std::fs::read(path)?;

    if data.len() < 9 || &data[0..4] != b"ASB\0" || data[4] != 0 {
        bail!("Invalid magic: expected b\"ASB\\0\\0\"");
    }

    let mut pos: usize = 5;
    let num_items = read_u32_le(&data, &mut pos).unwrap_or(0) as usize;

    let mut blocks: Vec<AsbBlock> = Vec::new();
    let mut current_label: Option<String> = None;
    let mut current_commands: Vec<AsbCommand> = Vec::new();

    for _ in 0..num_items {
        if pos + 4 > data.len() {
            break;
        }
        let item_type = match read_u32_le(&data, &mut pos) {
            Some(t) => t,
            None => break,
        };

        match item_type {
            1 => {
                let name = match read_string(&data, &mut pos) {
                    Some(s) => s,
                    None => break,
                };
                if let Some(prev) = current_label.take() {
                    blocks.push(AsbBlock {
                        label: prev,
                        commands: std::mem::take(&mut current_commands),
                    });
                }
                current_label = Some(name);
            }
            0 => {
                let tag = match read_string(&data, &mut pos) {
                    Some(s) => s,
                    None => break,
                };
                let _line_number = read_u32_le(&data, &mut pos).unwrap_or(0);
                let num_attrs = read_u32_le(&data, &mut pos).unwrap_or(0) as usize;

                let mut attrs = HashMap::with_capacity(num_attrs);
                for _ in 0..num_attrs {
                    let key = match read_string(&data, &mut pos) {
                        Some(s) => s,
                        None => break,
                    };
                    let val = match read_string(&data, &mut pos) {
                        Some(s) => s,
                        None => break,
                    };
                    attrs.insert(key, val);
                }

                current_commands.push(AsbCommand { tag, attrs });
            }
            _ => break,
        }
    }

    if let Some(label) = current_label {
        blocks.push(AsbBlock {
            label,
            commands: current_commands,
        });
    }

    Ok(AsbScript { blocks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_small_asb() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/artemis-export/test_small.asb");
        let script = parse_asb(&path).unwrap();
        assert!(!script.blocks.is_empty(), "Expected at least one block");
        let main = script.blocks.iter().find(|b| b.label == "main");
        assert!(main.is_some(), "Expected block labeled 'main'");
        let main = main.unwrap();
        assert_eq!(main.commands.len(), 12, "Expected 12 commands in 'main' block");
        let tags: Vec<_> = main.commands.iter().map(|c| c.tag.as_str()).collect();
        assert!(tags.contains(&"SetValidityOfLoading"));
        assert!(tags.contains(&"SetValidityOfSaving"));
        assert!(tags.contains(&"Size"));
        assert!(tags.contains(&"Window"));
        assert!(tags.contains(&"Blackout"));
        assert!(tags.contains(&"DrawScene"));
        assert!(tags.contains(&"FadeFilm"));
        assert!(tags.contains(&"Wait"));
        assert!(tags.contains(&"SetGlobalFlag"));
        assert!(tags.contains(&"RouteFlag"));
        assert!(tags.contains(&"Fadeout"));
    }

    #[test]
    fn test_parse_check_attrs() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/artemis-export/test_small.asb");
        let script = parse_asb(&path).unwrap();
        let main = script.blocks.iter().find(|b| b.label == "main").unwrap();
        let wait = main.commands.iter().find(|c| c.tag == "Wait").unwrap();
        assert_eq!(wait.attrs.get("0").map(|s| s.as_str()), Some("8000"));
    }
}
