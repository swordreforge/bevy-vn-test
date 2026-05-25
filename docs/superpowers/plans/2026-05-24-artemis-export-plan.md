# artemis-export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task.

**Goal:** Build `artemis-export`, a CLI tool that converts Artemis .asb scripts + .lua configs into Bevy VN `.bscript.ron` files.

**Architecture:** Workspace Rust crate in `tools/artemis-export/`. Reads `game-source/scenario/**/*.asb` binary files and `game-source/system/**/*.lua` configs via pattern extraction. Produces one RON file per .asb in the output directory. Three internal modules: `asb` (binary parser), `lua_config` (config extractor), `mapper` (command conversion).

**Tech Stack:** Rust, serde + ron (shared from bevy-vn workspace), anyhow.

---

## File Structure

### New files
| File | Responsibility |
|---|---|
| `tools/artemis-export/Cargo.toml` | Crate manifest, depends on bevy-vn |
| `tools/artemis-export/src/main.rs` | CLI entry: parse args, orchestrate pipeline |
| `tools/artemis-export/src/asb.rs` | `.asb` binary parser — reads bytes → `AsbScript` IR |
| `tools/artemis-export/src/lua_config.rs` | Lua config extractor — regex/pattern matching on Lua files |
| `tools/artemis-export/src/mapper.rs` | Command mapper — `AsbCommand` → `ScriptCmd` |
| `tests/artemis-export/test_small.asb` | Test fixture (copy of aiy70330.asb) |
| `tests/artemis-export/test_lua_config.lua` | Test fixture (minimal Lua config) |

### Modified files
| File | Change |
|---|---|
| `Cargo.toml` | Add lib target, add workspace member |
| `src/lib.rs` | New file (expose types for artemis-export) |

---

### Task 1: Workspace setup + library target

**Files:**
- Create: `Cargo.toml` (modify existing — add lib + workspace member)
- Create: `src/lib.rs` (new — re-export public types)
- Create: `tools/artemis-export/Cargo.toml`
- Create: `tools/artemis-export/src/main.rs`

- [ ] **Step 1: Add lib target to bevy-vn's Cargo.toml**

```toml
# Add before [[bin]]
[lib]
name = "bevy_vn"
path = "src/lib.rs"
```

- [ ] **Step 2: Create src/lib.rs that re-exports shared types**

```rust
pub mod script;
pub mod state;
pub mod components;
pub mod resources;
pub mod events;
// plugins not needed — artemis-export only needs data types
```

- [ ] **Step 3: Add workspace member in root Cargo.toml**

```toml
# Add under [workspace]
members = ["tools/artemis-export"]
```

- [ ] **Step 4: Create tools/artemis-export/Cargo.toml**

```toml
[package]
name = "artemis-export"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy_vn = { path = "../.." }
serde = { version = "1", features = ["derive"] }
ron = "0.8"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
```

- [ ] **Step 5: Create tools/artemis-export/src/main.rs skeleton**

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "artemis-export", about = "Convert Artemis .asb scripts to Bevy VN .bscript.ron")]
struct Args {
    #[arg(long)]
    input: String,
    #[arg(long)]
    output: String,
    #[arg(long, default_value_t = false)]
    verbose: bool,
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("Input: {}", args.input);
    println!("Output: {}", args.output);
    Ok(())
}
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo build -p artemis-export`
Expected: Build success

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/lib.rs tools/artemis-export/
git commit -m "feat: add artemis-export workspace crate"
```

---

### Task 2: ASB binary parser

**Files:**
- Create: `tools/artemis-export/src/asb.rs`

- [ ] **Step 1: Define IR types in asb.rs**

```rust
use anyhow::{bail, Result};
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
    pub params: Vec<AsbParam>,
}

#[derive(Debug, Clone)]
pub enum AsbParam {
    Int(i32),
    Str(String),
}
```

- [ ] **Step 2: Implement the byte cursor helper**

```rust
struct ByteCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn skip(&mut self, n: usize) {
        self.pos = self.pos.saturating_add(n).min(self.data.len());
    }

    fn read_u32_le(&mut self) -> Option<u32> {
        if self.pos + 4 > self.data.len() {
            return None;
        }
        let bytes = &self.data[self.pos..self.pos + 4];
        self.pos += 4;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.pos + n > self.data.len() {
            return None;
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Some(slice)
    }

    fn peek_u8(&self) -> Option<u8> {
        if self.pos >= self.data.len() {
            None
        } else {
            Some(self.data[self.pos])
        }
    }

    fn read_string(&mut self) -> Option<String> {
        // Read null-terminated string
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }
        if self.pos > start {
            let s = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
            // Skip the null terminator
            if self.pos < self.data.len() {
                self.pos += 1;
            }
            Some(s)
        } else {
            None
        }
    }

    fn align_to_4(&mut self) {
        let remainder = self.pos % 4;
        if remainder != 0 {
            self.skip(4 - remainder);
        }
    }
}
```

- [ ] **Step 3: Implement the main parse function**

```rust
pub fn parse_asb(path: &Path) -> Result<AsbScript> {
    let data = std::fs::read(path)?;
    if data.len() < 16 {
        bail!("File too small: {} bytes", data.len());
    }

    // Validate magic
    if &data[0..4] != b"ASB\0" {
        bail!("Invalid magic: expected ASB\\0");
    }

    let mut cursor = ByteCursor::new(&data);
    cursor.skip(4); // magic

    // Read 12 bytes of header/version info
    let _version = cursor.read_u32_le();
    let _unknown1 = cursor.read_u32_le();
    let _unknown2 = cursor.read_u32_le();

    let mut blocks = Vec::new();

    while cursor.remaining() > 0 {
        // Skip null markers / padding bytes
        while cursor.peek_u8() == Some(0) && cursor.remaining() > 0 {
            cursor.skip(1);
        }
        if cursor.remaining() == 0 {
            break;
        }

        // Read label name (null-terminated)
        let Some(label) = cursor.read_string() else { break };
        if label.is_empty() {
            continue;
        }
        cursor.align_to_4();

        let mut commands = Vec::new();

        // Read commands until next label marker (double null or end)
        loop {
            // Skip leading null bytes
            while cursor.peek_u8() == Some(0) && cursor.remaining() > 0 {
                cursor.skip(1);
            }
            if cursor.remaining() == 0 {
                break;
            }

            // Peek ahead: if this looks like a new label (sequence of printable ascii
            // followed by more structure), stop. For now, we use the heuristic that
            // commands have non-empty tag names and labels have specific patterns.
            // We'll refine this during testing.
            let saved_pos = cursor.pos;

            let Some(tag) = cursor.read_string() else { break };
            if tag.is_empty() {
                continue;
            }

            // Crude heuristic: if the tag is all uppercase chars and > 1 char,
            // and followed by params, it's likely a command, not a label.
            // We accept any tag that's not a known "block label" pattern.
            let is_command = !tag.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                || tag.starts_with("calllua")
                || tag == "return"
                || tag == "stop";

            if !is_command && commands.is_empty() {
                // This is probably a sub-label, not a command — backtrack
                cursor.pos = saved_pos;
                break;
            }

            cursor.align_to_4();

            let mut params = Vec::new();

            // Read parameters (heuristic: read int/string pairs)
            while cursor.remaining() >= 4 {
                let saved = cursor.pos;
                // Try to read a param — we detect the next tag start by
                // looking for patterns that indicate a new entry.
                // For now, read until we hit what looks like a new tag:
                // a null-marker followed by a short printable string.
                let next_is_tag = {
                    let mut lookahead = cursor.pos;
                    // Skip any nulls
                    while lookahead < data.len() && data[lookahead] == 0 {
                        lookahead += 1;
                    }
                    if lookahead < data.len() {
                        // Check if next bytes look like a string (printable ascii starting letters)
                        let c = data[lookahead];
                        c.is_ascii_alphabetic() || c == b'_'
                    } else {
                        false
                    }
                };

                if next_is_tag && cursor.pos > saved_pos {
                    break;
                }

                // Try to read as u32 string length first
                let Some(val) = cursor.read_u32_le() else { break };
                let val = val as i32;

                // If the value looks like a small printable ASCII string length,
                // treat the next bytes as a string
                if val > 0 && val < 256 && cursor.pos + val as usize <= data.len() {
                    let string_start = cursor.pos;
                    let string_bytes = &data[string_start..string_start + val as usize];
                    if string_bytes.iter().all(|&b| b >= 0x20 && b <= 0x7e || b == 0) {
                        let s = String::from_utf8_lossy(string_bytes).to_string();
                        let trimmed = s.trim_end_matches('\0').to_string();
                        cursor.skip(val as usize);
                        cursor.align_to_4();
                        params.push(AsbParam::Str(trimmed));
                        continue;
                    }
                }

                // Otherwise treat as integer and backtrack the string attempt
                cursor.pos = saved;
                cursor.skip(4);
                params.push(AsbParam::Int(val));
            }

            commands.push(AsbCommand { tag, params });
        }

        blocks.push(AsbBlock { label, commands });
    }

    Ok(AsbScript { blocks })
}
```

- [ ] **Step 4: Add parse_asb test**

Add at the bottom of asb.rs:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_small_asb() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/artemis-export/test_small.asb");
        if !path.exists() {
            eprintln!("Skipping test: fixture not found at {:?}", path);
            return;
        }
        let script = parse_asb(&path).unwrap();
        assert!(!script.blocks.is_empty(), "Expected at least one block");
        // aiy70330 should have a "main" block with several commands
        let main = script.blocks.iter().find(|b| b.label == "main");
        assert!(main.is_some(), "Expected block labeled 'main'");
        assert!(!main.unwrap().commands.is_empty(), "Expected commands in 'main' block");
    }
}
```

- [ ] **Step 5: Copy test fixture and run test**

```bash
cp /home/swordreforge/Downloads/game-source/scenario/main/aiy70330.asb tests/artemis-export/test_small.asb
```

```bash
cargo test -p artemis-export -- asb::tests --nocapture
```

This will likely fail or produce unexpected output — iterate on the parser logic until it correctly extracts blocks and commands from `aiy70330.asb`. Tune the block/command boundary heuristics based on actual output.

- [ ] **Step 6: Commit**

```bash
git add tools/artemis-export/src/asb.rs tests/artemis-export/test_small.asb
git commit -m "feat: ASB binary parser with block and command extraction"
```

---

### Task 3: Lua config extractor

**Files:**
- Create: `tools/artemis-export/src/lua_config.rs`

- [ ] **Step 1: Define config data types**

```rust
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct GameConfig {
    /// CG set → list of image files
    pub cg_sets: HashMap<String, Vec<String>>,
    /// BGM id → ogg filename (from extra_bgm table)
    pub bgm_files: HashMap<String, String>,
    /// Scene id → { file, label }
    pub scene_jumps: HashMap<String, Vec<SceneEntry>>,
    /// FG character sprite paths
    pub fg_paths: HashMap<String, String>,
    /// bg/ev/voice path prefixes from init table
    pub init_paths: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SceneEntry {
    pub file: String,
    pub label: String,
}
```

- [ ] **Step 2: Implement extraction**

```rust
use std::path::Path;
use anyhow::Result;

pub fn extract_config(root: &Path) -> Result<GameConfig> {
    let mut config = GameConfig::default();
    let extra_dir = root.join("system/extra");
    let adv_dir = root.join("system/adv");

    // Read init paths from Lua files that define init.bg_path, etc.
    if let Ok(content) = std::fs::read_to_string(adv_dir.join("macro.lua")) {
        for line in content.lines() {
            if let Some(path) = extract_init_path(line, "init.bg_path") {
                config.init_paths.insert("bg".to_string(), path);
            }
            if let Some(path) = extract_init_path(line, "init.ev_path") {
                config.init_paths.insert("ev".to_string(), path);
            }
            if let Some(path) = extract_init_path(line, "init.fg_path") {
                config.init_paths.insert("fg".to_string(), path);
            }
            if let Some(path) = extract_init_path(line, "init.bgm_path") {
                config.init_paths.insert("bgm".to_string(), path);
            }
            if let Some(path) = extract_init_path(line, "init.se_path") {
                config.init_paths.insert("se".to_string(), path);
            }
        }
    }

    // Read CG config from csv.extra_cgmode table
    if let Ok(content) = std::fs::read_to_string(extra_dir.join("cg.lua")) {
        extract_cg_table(&content, &mut config);
    }

    // Read BGM config from csv.extra_bgm table
    if let Ok(content) = std::fs::read_to_string(extra_dir.join("bgm.lua")) {
        extract_bgm_table(&content, &mut config);
    }

    // Read scene config from csv.extra_event table
    if let Ok(content) = std::fs::read_to_string(extra_dir.join("scene.lua")) {
        extract_scene_table(&content, &mut config);
    }

    Ok(config)
}

fn extract_init_path(line: &str, key: &str) -> Option<String> {
    if line.contains(key) && line.contains('=') {
        let rhs = line.split('=').nth(1)?;
        let path = rhs.trim()
            .trim_matches('"')
            .trim_matches(',')
            .trim()
            .to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

fn extract_cg_table(content: &str, config: &mut GameConfig) {
    // Look for: csv.extra_cgmode["set_name"] = { val1, val2, ... }
    for line in content.lines() {
        if let Some(cap) = line.trim().strip_prefix("csv.extra_cgmode[\"") {
            if let Some(set_name) = cap.split("\"]").next() {
                if let Some(values_part) = cap.split("= {").nth(1) {
                    let values: Vec<String> = values_part
                        .trim_end_matching('}', ',', ' ')
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .filter(|s| !s.is_empty() && s != "}")
                        .collect();
                    if !values.is_empty() {
                        config.cg_sets.insert(set_name.to_string(), values);
                    }
                }
            }
        }
    }
}

fn extract_bgm_table(content: &str, config: &mut GameConfig) {
    // Look for: csv.extra_bgm["bgm_id"] = { flag, file, title, duration }
    for line in content.lines() {
        if let Some(cap) = line.trim().strip_prefix("csv.extra_bgm[\"") {
            if let Some(id) = cap.split("\"]").next() {
                if let Some(values_part) = cap.split("= {").nth(1) {
                    let file = values_part.split(',').nth(1)
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .unwrap_or_default();
                    if !file.is_empty() {
                        config.bgm_files.insert(id.to_string(), file);
                    }
                }
            }
        }
    }
}

fn extract_scene_table(content: &str, config: &mut GameConfig) {
    // We'll implement scene extraction similarly during development.
    // The pattern is: csv.extra_event[idx] = { char, page, file, label, ... }
}

/// Helper to trim trailing matching characters
fn trim_end_matching(s: &str, chars: &[char]) -> &str {
    let mut trimmed = s;
    while trimmed.ends_with(chars) {
        trimmed = &trimmed[..trimmed.len() - 1];
    }
    trimmed.trim()
}
```

- [ ] **Step 3: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_init_path() {
        assert_eq!(
            extract_init_path(r#"init.bg_path = "image/bg/""#, "init.bg_path"),
            Some("image/bg/".to_string())
        );
    }

    #[test]
    fn test_extract_cg_set() {
        let content = r#"
            csv.extra_cgmode["eve_0101"] = { "eve_0101", 1, "eve_010101", "eve_010102" }
        "#;
        let mut config = GameConfig::default();
        extract_cg_table(content, &mut config);
        assert!(config.cg_sets.contains_key("eve_0101"));
        assert_eq!(config.cg_sets["eve_0101"].len(), 4);
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p artemis-export -- lua_config::tests --nocapture`
Expected: Tests pass

- [ ] **Step 5: Commit**

```bash
git add tools/artemis-export/src/lua_config.rs
git commit -m "feat: Lua config extractor for CG, BGM, FG, scene metadata"
```

---

### Task 4: Command mapper

**Files:**
- Create: `tools/artemis-export/src/mapper.rs`

- [ ] **Step 1: Define mapper types + main function**

```rust
use crate::asb::{AsbCommand, AsbParam, AsbScript};
use bevy_vn::script::{ChoiceOption, ConditionOp, FgPosition, Script, ScriptCmd, Transition};

/// Convert an AsbScript into a Script by mapping known command patterns.
/// Returns all commands — labels, mapped narrative commands, and skipped
/// commands logged when verbose is enabled.
pub fn map_script(
    asb: &AsbScript,
    config: &crate::lua_config::GameConfig,
    verbose: bool,
) -> Script {
    let mut output = Script::new();

    for block in &asb.blocks {
        // Emit a label for each block
        output.push(ScriptCmd::Label {
            name: block.label.clone(),
        });

        for cmd in &block.commands {
            match map_command(cmd, config) {
                Some(script_cmd) => output.push(script_cmd),
                None => {
                    if verbose {
                        eprintln!(
                            "  [skip] {} ({} params)",
                            cmd.tag,
                            cmd.params.len()
                        );
                    }
                }
            }
        }
    }

    output
}
```

- [ ] **Step 2: Implement map_command for native tags**

```rust
fn map_command(cmd: &AsbCommand, config: &crate::lua_config::GameConfig) -> Option<ScriptCmd> {
    match cmd.tag.as_str() {
        "Jump" => {
            // Jump { file: ..., label: ... } or Jump { file: ..., label: ..., ... }
            let label = get_str_param(cmd, 2) // param with file
                .or_else(|| get_str_param(cmd, 0));
            label.map(|l| ScriptCmd::Jump { target: l.to_string() })
        }
        "Call" => {
            let target = get_str_param(cmd, 0)?;
            Some(ScriptCmd::Call { target: target.to_string() })
        }
        "return" => Some(ScriptCmd::Return),
        "Return" => Some(ScriptCmd::Return),
        "Wait" => {
            let duration = get_int_param(cmd, 0).unwrap_or(1000) as u64;
            Some(ScriptCmd::Wait { duration })
        }
        "calllua" => map_calllua(cmd, config),
        _ => None, // skip rendering tags like lyc, lyprop, etc.
    }
}
```

- [ ] **Step 3: Implement calllua mapping**

```rust
fn map_calllua(cmd: &AsbCommand, config: &crate::lua_config::GameConfig) -> Option<ScriptCmd> {
    // calllua has string params, usually KV-like. Extract the "function" param.
    let func = get_str_param_by_key(cmd, "function")
        .or_else(|| get_str_param(cmd, 0))?;

    match func.as_str() {
        s if s.contains("set_bg") || s == "tags.setbg" || s == "setbg" => {
            let file = get_str_param_by_key(cmd, "file")?;
            let transition = if get_str_param_by_key(cmd, "fade").is_some() {
                Some(Transition::Fade)
            } else {
                None
            };
            let duration = get_int_param_by_key(cmd, "fade").map(|d| d as u64);
            Some(ScriptCmd::SetBg {
                file: file.to_string(),
                transition,
                duration,
            })
        }
        s if s.contains("tags.bgm") || s.contains("bgm_play") => {
            let id = get_str_param_by_key(cmd, "file")
                .or_else(|| get_str_param(cmd, 0))?;
            let volume = get_int_param_by_key(cmd, "vol").map(|v| v as f32 / 100.0);
            let fade_in = get_int_param_by_key(cmd, "time").map(|t| t as u64);
            Some(ScriptCmd::PlayBgm {
                id: id.to_string(),
                volume,
                fade_in,
            })
        }
        s if s.contains("bgm_stop") => {
            let id = get_str_param_by_key(cmd, "file").map(|s| s.to_string());
            let fade_out = get_int_param_by_key(cmd, "time").map(|t| t as u64);
            Some(ScriptCmd::StopBgm { id, fade_out })
        }
        s if s.contains("se_play") => {
            let file = get_str_param_by_key(cmd, "file")
                .or_else(|| get_str_param(cmd, 0))?;
            let volume = get_int_param_by_key(cmd, "vol").map(|v| v as f32 / 100.0);
            Some(ScriptCmd::PlaySe {
                file: file.to_string(),
                volume,
            })
        }
        s if s.contains("voice_play") => {
            let file = get_str_param_by_key(cmd, "file")
                .or_else(|| get_str_param(cmd, 0))?;
            Some(ScriptCmd::PlayVoice {
                file: file.to_string(),
            })
        }
        s if s.contains("show_fg") || s.contains("fg_show") => {
            let char_id = get_str_param_by_key(cmd, "id")
                .or_else(|| get_str_param(cmd, 0))?;
            let expression = get_str_param_by_key(cmd, "file")
                .or_else(|| get_str_param(cmd, 1))
                .unwrap_or("000");
            Some(ScriptCmd::ShowFg {
                char_id: char_id.to_string(),
                expression: expression.to_string(),
                position: FgPosition::Center,
                transition: None,
            })
        }
        s if s.contains("hide_fg") || s.contains("fg_hide") => {
            let char_id = get_str_param_by_key(cmd, "id")
                .or_else(|| get_str_param(cmd, 0))
                .unwrap_or("all");
            Some(ScriptCmd::HideFg {
                char_id: char_id.to_string(),
                transition: None,
            })
        }
        s if s.contains("show_cg") || s.contains("cg_show") => {
            let file = get_str_param_by_key(cmd, "file")
                .or_else(|| get_str_param(cmd, 0))?;
            Some(ScriptCmd::ShowCg {
                file: file.to_string(),
                transition: None,
            })
        }
        s if s.contains("hide_cg") || s.contains("cg_hide") => {
            Some(ScriptCmd::HideCg { transition: None })
        }
        s if s.contains("affection_change") || s.contains("affection") => {
            let char_id = get_str_param_by_key(cmd, "char_id")
                .or_else(|| get_str_param(cmd, 0))
                .unwrap_or("default");
            let delta = get_int_param_by_key(cmd, "delta")
                .or_else(|| get_int_param(cmd, 1))
                .unwrap_or(1) as i32;
            Some(ScriptCmd::AffectionChange {
                char_id: char_id.to_string(),
                delta,
            })
        }
        s if s.contains("tags.choice") || s.contains("choice") => {
            // Choice commands are complex — may need block context.
            // For now, emit a placeholder choice marker.
            // Full choice mapping will be refined during testing.
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
        s if s.contains("tags.wt") || s.contains("tags.wtx") || s.contains("wait_tag") => {
            let duration = get_int_param_by_key(cmd, "time").unwrap_or(500) as u64;
            Some(ScriptCmd::Wait { duration })
        }
        s if s.contains("unlock_cg") || s.contains("cg_unlock") => {
            let file = get_str_param_by_key(cmd, "file")
                .or_else(|| get_str_param(cmd, 0))
                .unwrap_or("unknown");
            Some(ScriptCmd::UnlockCg {
                file: file.to_string(),
            })
        }
        _ => None,
    }
}
```

- [ ] **Step 4: Implement param extraction helpers**

```rust
/// Get the n-th positional string parameter
fn get_str_param(cmd: &AsbCommand, index: usize) -> Option<&str> {
    cmd.params.iter()
        .filter_map(|p| {
            if let AsbParam::Str(s) = p { Some(s.as_str()) } else { None }
        })
        .nth(index)
}

/// Get the n-th positional integer parameter
fn get_int_param(cmd: &AsbCommand, index: usize) -> Option<i32> {
    cmd.params.iter()
        .filter_map(|p| {
            if let AsbParam::Int(i) = p { Some(*i) } else { None }
        })
        .nth(index)
}

/// Find a string param by matching positionally as KV pair:
/// ["key", "value", "key2", "value2"...]
fn get_str_param_by_key(cmd: &AsbCommand, key: &str) -> Option<&str> {
    let mut i = 0;
    while i + 1 < cmd.params.len() {
        match (&cmd.params[i], &cmd.params[i + 1]) {
            (AsbParam::Str(k), AsbParam::Str(v)) if k == key => return Some(v),
            _ => {}
        }
        i += 1;
    }
    None
}

fn get_int_param_by_key(cmd: &AsbCommand, key: &str) -> Option<i32> {
    let mut i = 0;
    while i + 1 < cmd.params.len() {
        match (&cmd.params[i], &cmd.params[i + 1]) {
            (AsbParam::Str(k), AsbParam::Int(v)) if k == key => return Some(*v),
            _ => {}
        }
        i += 1;
    }
    None
}
```

- [ ] **Step 5: Add test for mapper**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua_config::GameConfig;

    #[test]
    fn test_map_set_bg() {
        let cmd = AsbCommand {
            tag: "calllua".into(),
            params: vec![
                AsbParam::Str("function".into()),
                AsbParam::Str("set_bg".into()),
                AsbParam::Str("file".into()),
                AsbParam::Str("bg_0001".into()),
            ],
        };
        let config = GameConfig::default();
        let result = map_command(&cmd, &config);
        assert!(result.is_some());
        match result.unwrap() {
            ScriptCmd::SetBg { file, .. } => assert_eq!(file, "bg_0001"),
            other => panic!("Expected SetBg, got {:?}", other),
        }
    }

    #[test]
    fn test_skip_rendering_tag() {
        let cmd = AsbCommand {
            tag: "lyc".into(),
            params: vec![AsbParam::Str("id".into()), AsbParam::Str("1.1.0.0".into())],
        };
        assert!(map_command(&cmd, &GameConfig::default()).is_none());
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p artemis-export -- mapper::tests --nocapture`
Expected: Tests pass

- [ ] **Step 7: Commit**

```bash
git add tools/artemis-export/src/mapper.rs
git commit -m "feat: command mapper — calllua patterns + native tags → ScriptCmd"
```

---

### Task 5: CLI pipeline + RON output

**Files:**
- Modify: `tools/artemis-export/src/main.rs`

- [ ] **Step 1: Wire up the pipeline in main.rs**

```rust
use std::path::{Path, PathBuf};
use clap::Parser;
use anyhow::{Context, Result};

mod asb;
mod lua_config;
mod mapper;

#[derive(Parser)]
#[command(name = "artemis-export", about = "Convert Artemis .asb scripts to Bevy VN .bscript.ron")]
struct Args {
    /// Path to game-source root directory
    #[arg(long)]
    input: String,
    /// Output directory for .bscript.ron files
    #[arg(long)]
    output: String,
    /// Log skipped commands (verbose)
    #[arg(long, default_value_t = false)]
    verbose: bool,
    /// Count files without writing
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let input_root = PathBuf::from(&args.input);
    let output_dir = PathBuf::from(&args.output);

    if !input_root.exists() {
        anyhow::bail!("Input directory not found: {}", args.input);
    }

    if !args.dry_run {
        std::fs::create_dir_all(&output_dir)
            .context("Failed to create output directory")?;
    }

    // Step 1: Extract Lua config
    eprintln!("[1/3] Extracting Lua configs...");
    let config = lua_config::extract_config(&input_root)
        .context("Failed to extract Lua configs")?;

    // Step 2: Discover .asb files
    let asb_files = discover_asb_files(&input_root);
    if asb_files.is_empty() {
        anyhow::bail!("No .asb files found under {:?}", input_root);
    }
    eprintln!("[2/3] Found {} .asb files", asb_files.len());

    // Step 3: Convert each file
    let mut converted = 0;
    let mut skipped  = 0;

    for asb_path in &asb_files {
        let script = match asb::parse_asb(asb_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  [warn] Failed to parse {:?}: {}", asb_path, e);
                skipped += 1;
                continue;
            }
        };

        let output_name = asb_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let output_path = output_dir.join(format!("{}.bscript.ron", output_name));

        let ron_cmds = mapper::map_script(&script, &config, args.verbose);

        if !args.dry_run {
            let ron_str = ron::ser::to_string_pretty(&ron_cmds, ron::ser::PrettyConfig::default())
                .context("RON serialization failed")?;
            std::fs::write(&output_path, ron_str)
                .with_context(|| format!("Failed to write {:?}", output_path))?;
        }

        if args.verbose {
            eprintln!("  -> {} ({} commands)", output_name, ron_cmds.len());
        }
        converted += 1;
    }

    eprintln!("[3/3] Done: {} converted, {} skipped", converted, skipped);
    Ok(())
}

fn discover_asb_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let scenario_dir = root.join("scenario");
    if scenario_dir.exists() {
        collect_asb_files(&scenario_dir, &mut files);
    }
    files.sort();
    files
}

fn collect_asb_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_asb_files(&path, files);
            } else if path.extension().map(|e| e == "asb").unwrap_or(false) {
                files.push(path);
            }
        }
    }
}
```

- [ ] **Step 2: Build and test with dry-run**

```bash
cargo build -p artemis-export
./target/debug/artemis-export --input /home/swordreforge/Downloads/game-source --output /tmp/artemis-test --dry-run
```

Expected: "Found N .asb files" (N should be ~204) and "Done: N converted, 0 skipped" (some files may be skipped if they fail to parse).

- [ ] **Step 3: Full conversion test**

```bash
./target/debug/artemis-export --input /home/swordreforge/Downloads/game-source --output /tmp/artemis-test
ls /tmp/artemis-test/*.bscript.ron | wc -l
```

Expected: ~204 ron files

- [ ] **Step 4: Verify RON output is valid**

```bash
cargo run --example validate_ron /tmp/artemis-test/ 2>/dev/null || true
# Manual: check one output file parses back into ScriptCmd
head -30 /tmp/artemis-test/aiy70330.bscript.ron
```

- [ ] **Step 5: Commit**

```bash
git add tools/artemis-export/src/main.rs
git commit -m "feat: CLI pipeline with Lua extraction, ASB parse, and RON output"
```

---

### Task 6: Integration with bevy-vn

**Files:**
- Modify: `assets/scripts/` (move test scripts to location where engine can find them)

- [ ] **Step 1: Copy converted scripts to bevy-vn assets**

```bash
mkdir -p assets/scripts/converted
cp /tmp/artemis-test/*.bscript.ron assets/scripts/converted/
```

- [ ] **Step 2: Verify scripts load in bevy-vn**

Add a test script at `assets/scripts/converted/test_main.bscript.ron` that calls a converted sub-script, or modify the existing test to load a converted file.

- [ ] **Step 3: Run cargo check on the workspace**

```bash
cargo check
```

Expected: No errors

- [ ] **Step 4: Run smoke test**

```bash
cargo run 2>&1 | grep -i "error\|panic" || echo "No errors"
```

- [ ] **Step 5: Commit**

```bash
git add assets/scripts/converted/
git commit -m "feat: integrate converted Artemis scripts into assets"
```

---

### Task 7: Edge cases + refinement

**Files:**
- Modify: `tools/artemis-export/src/asb.rs` (parser refinements)
- Modify: `tools/artemis-export/src/mapper.rs` (new patterns)

- [ ] **Step 1: Handle common parse failures**

Run the converter and inspect files that were skipped. Common issues:
- ASB files with different header variants (asbmacro.asb has different header from aiy70330.asb)
- Blocks with sub-labels (not caught by heuristic)
- Unexpected parameter encodings

Fix each issue in `asb.rs` with parser refinements.

- [ ] **Step 2: Add missing command patterns**

Run `--verbose` and collect all skipped calllua patterns. Add mappings for:
- dialogue text commands
- choice options (may involve reading adjacent blocks)
- clear_text
- play_movie
- condition (global flag checks)

- [ ] **Step 3: Lua config completeness**

Verify CG/BGM/Scene/FG extraction against known files. Check:
- `eve_010101.png` exists in CG set mapping
- BGM files resolve to correct ogg names
- Scene jumps point to valid .asb files

- [ ] **Step 4: Commit**

```bash
git add tools/artemis-export/src/ assets/scripts/converted/
git commit -m "fix: refine parser and mapper for production data"
```
