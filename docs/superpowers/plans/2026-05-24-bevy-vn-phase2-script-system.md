# Phase 2: Script System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Load `.bscript.ron` scripts from disk, execute them sequentially, drive dialogue output with text reveal and click-to-advance.

**Architecture:** ScriptRunner plugin processes `ScriptCmd` sequentially, mutates `DialogueState` for text display. A dedicated `TextRevealTimer` drives character-by-character reveal. `AdvanceEvent` gates progression — advance script when text is fully shown. Non-dialogue commands (audio, background, etc.) are logged as no-ops, ready for Phase 3 wiring.

**Tech Stack:** Rust, Bevy 0.18, RON (asset format), `std::time::Instant` for reveal timing.

---

### Task 1: ScriptEngine helpers

**Files:**
- Modify: `src/script.rs`
- Test: N/A (internal data methods)

- [ ] **Step 1: Add find_label, advance, and peek methods to ScriptEngine**

```rust
// Append to impl ScriptEngine block in src/script.rs

impl ScriptEngine {
    pub fn load(&mut self, name: &str, script: Vec<ScriptCmd>) {
        self.current_script = name.to_string();
        self.scripts.insert(name.to_string(), script);
        self.current_line = 0;
        self.call_stack.clear();
    }

    pub fn advance(&mut self) -> Option<&ScriptCmd> {
        let idx = self.current_line;
        self.current_line += 1;
        self.scripts.get(&self.current_script)?.get(idx)
    }

    pub fn peek(&self) -> Option<&ScriptCmd> {
        let idx = self.current_line;
        self.scripts.get(&self.current_script)?.get(idx)
    }

    pub fn jump_to_label(&mut self, label: &str) {
        if let Some(script) = self.scripts.get(&self.current_script) {
            for (i, cmd) in script.iter().enumerate() {
                if let ScriptCmd::Label { name } = cmd {
                    if name == label {
                        self.current_line = i + 1;
                        return;
                    }
                }
            }
        }
    }

    pub fn call_label(&mut self, label: &str) {
        self.call_stack.push((self.current_script.clone(), self.current_line));
        self.jump_to_label(label);
    }

    pub fn return_from_call(&mut self) {
        if let Some((script, line)) = self.call_stack.pop() {
            self.current_script = script;
            self.current_line = line;
        }
    }

    pub fn has_more(&self) -> bool {
        self.scripts
            .get(&self.current_script)
            .map_or(false, |s| self.current_line < s.len())
    }
}
```

Also add the `scripts` HashMap to the struct:

```rust
#[derive(Resource, Default)]
pub struct ScriptEngine {
    pub current_script: String,
    pub current_line: usize,
    pub call_stack: Vec<(String, usize)>,
    pub flags: HashMap<String, i32>,
    pub scripts: HashMap<String, Vec<ScriptCmd>>,
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check
```

Expected: compilation succeeds.

---

### Task 2: Script loader system

**Files:**
- Create: `src/plugins/script_loader.rs`
- Modify: `src/plugins/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create ScriptLoaderPlugin**

```rust
// src/plugins/script_loader.rs
use bevy::prelude::*;
use crate::script::{ScriptCmd, ScriptEngine};
use std::fs;

pub struct ScriptLoaderPlugin;

impl Plugin for ScriptLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_test_script);
    }
}

fn load_test_script(mut engine: ResMut<ScriptEngine>) {
    let path = "assets/scripts/test.bscript.ron";
    match fs::read_to_string(path) {
        Ok(content) => {
            match ron::from_str::<Vec<ScriptCmd>>(&content) {
                Ok(script) => {
                    engine.load("test", script);
                    info!("Loaded script: {}", path);
                }
                Err(e) => {
                    error!("Failed to parse {}: {}", path, e);
                }
            }
        }
        Err(e) => {
            error!("Failed to read {}: {}", path, e);
        }
    }
}
```

- [ ] **Step 2: Register module and plugin**

```rust
// src/plugins/mod.rs
pub mod title;
pub mod inputs;
pub mod affection;
pub mod save_load;
pub mod dialogue;
pub mod settings;
pub mod gallery;
pub mod script_loader;
```

```rust
// src/main.rs — add use line and plugin registration
use plugins::script_loader::ScriptLoaderPlugin;

// in main():
        .add_plugins(ScriptLoaderPlugin)
// add before .run()
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check
```

Expected: compilation succeeds.

---

### Task 3: ScriptRunner plugin (core execution loop)

**Files:**
- Create: `src/plugins/script_runner.rs`
- Modify: `src/plugins/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create ScriptRunnerPlugin**

```rust
// src/plugins/script_runner.rs
use bevy::prelude::*;
use crate::components::*;
use crate::resources::{AffectionMap, DialogueState};
use crate::script::{ScriptCmd, ScriptEngine};
use crate::state::AppState;
use crate::plugins::inputs::AdvanceEvent;

pub struct ScriptRunnerPlugin;

impl Plugin for ScriptRunnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Gameplay), start_script_execution)
            .add_systems(Update, (
                process_advance,
                update_text_reveal,
            ).run_if(in_state(AppState::Gameplay)));
    }
}

fn start_script_execution(mut dialogue: ResMut<DialogueState>) {
    dialogue.current_text.clear();
    dialogue.current_speaker = None;
    dialogue.text_progress = 0;
    dialogue.is_displaying = false;
}

fn process_advance(
    mut advance_ev: EventReader<AdvanceEvent>,
    mut engine: ResMut<ScriptEngine>,
    mut dialogue: ResMut<DialogueState>,
    mut affection: ResMut<AffectionMap>,
) {
    for _ in advance_ev.read() {
        // If text is still revealing, complete it immediately
        if dialogue.is_displaying && dialogue.text_progress < dialogue.current_text.len() {
            dialogue.text_progress = dialogue.current_text.len();
            continue;
        }

        // If text is fully displayed, consume it and advance
        if dialogue.is_displaying && dialogue.text_progress >= dialogue.current_text.len() {
            dialogue.is_displaying = false;
            continue;
        }

        // Execute next command(s) until we hit a Dialogue or blocking command
        while engine.has_more() {
            let cmd = engine.advance();
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
                    engine.jump_to_label(target);
                }
                Some(ScriptCmd::Call { target }) => {
                    engine.call_label(target);
                }
                Some(ScriptCmd::Return) => {
                    engine.return_from_call();
                }
                Some(ScriptCmd::Condition { var, value, operator, goto }) => {
                    let flag_val = engine.flags.get(&var).copied().unwrap_or(0);
                    let met = match operator {
                        crate::script::ConditionOp::Greater => flag_val > value,
                        crate::script::ConditionOp::Less => flag_val < value,
                        crate::script::ConditionOp::Equal => flag_val == value,
                        crate::script::ConditionOp::GreaterEqual => flag_val >= value,
                        crate::script::ConditionOp::LessEqual => flag_val <= value,
                    };
                    if met {
                        engine.jump_to_label(&goto);
                    }
                }
                Some(ScriptCmd::AffectionChange { char_id, delta }) => {
                    *affection.0.entry(char_id).or_insert(0) += delta;
                }
                Some(ScriptCmd::SavePoint) => {
                    // handled by save system; no-op here
                }
                Some(ScriptCmd::Wait { duration: _ }) => {
                    // TODO: Phase 4 — proper wait with timer
                }
                // Log non-blocking commands as info (Phase 3+ will wire these)
                Some(cmd) => {
                    info!("Script cmd (no-op): {:?}", cmd);
                }
                None => break,
            }
        }

        // If no more commands, mark done
        if !engine.has_more() && !dialogue.is_displaying {
            info!("Script finished: {}", engine.current_script);
        }
    }
}

fn update_text_reveal(
    time: Res<Time>,
    mut dialogue: ResMut<DialogueState>,
) {
    if dialogue.is_displaying && dialogue.text_progress < dialogue.current_text.len() {
        let chars_per_sec = 40.0; // TODO: read from Settings
        let increment = (time.delta_secs() * chars_per_sec) as usize;
        dialogue.text_progress = (dialogue.text_progress + increment).min(dialogue.current_text.len());
    }
}
```

Wait — `AdvanceEvent` is a `Message` (Bevy 0.18), not an `Event`. So I need `MessageReader` not `EventReader`. Let me fix that:

```rust
fn process_advance(
    mut advance_ev: MessageReader<AdvanceEvent>,
    ...
```

Also I need to import `MessageReader`. In Bevy 0.18, `Message` is auto-imported via `bevy::prelude::*` but let me check... The inputs.rs uses `MessageWriter`. So I need `use bevy::prelude::*` which should have `MessageReader`.

- [ ] **Step 2: Register module**

```rust
// src/plugins/mod.rs
pub mod script_runner;
```

```rust
// src/main.rs
use plugins::script_runner::ScriptRunnerPlugin;

// in main():
        .add_plugins(ScriptRunnerPlugin)
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check
```

Expected: compilation succeeds.

---

### Task 4: Create example test script

**Files:**
- Create: `assets/scripts/test.bscript.ron`
- Create: `assets/scripts/` (if missing)

- [ ] **Step 1: Write test script**

```ron
(
    Dialogue(speaker: Some("ナユタ"), text: "目が覚めたか。"),
    Dialogue(speaker: Some("ナユタ"), text: "長い間、眠っていたようだ。"),
    Dialogue(speaker: None, text: "ここはどこだろう。周りを見渡すが、見知らぬ場所だ。"),
    AffectionChange(char_id: "nayuta", delta: 1),
    Dialogue(speaker: Some("ナユタ"), text: "お前のその目は、何もかもを見透かす——"),
    Dialogue(speaker: Some("ナユタ"), text: "そう信じている。"),
)
```

- [ ] **Step 2: Verify compilation and assets**

```bash
mkdir -p /home/swordreforge/Downloads/bevy-vn/assets/scripts
# file created in step 1
cargo check
```

Expected: `cargo check` passes.

---

### Task 5: Integration wiring & end-to-end verify

**Files:**
- Modify: `src/plugins/dialogue.rs` — clean up, remove AdvanceEvent coupling (ScriptRunner now owns that)
- Verify: full compile + run

- [ ] **Step 1: Clean up DialoguePlugin to remove AdvanceEvent dependency**

The current DialoguePlugin doesn't directly use AdvanceEvent, so no changes needed. But ensure `update_dialogue` system reads `DialogueState` correctly (it already does).

- [ ] **Step 2: Full build check**

```bash
cargo build
```

Expected: binary compiles without errors or warnings.

- [ ] **Step 3: Quick smoke test (launch and exit)**

```bash
timeout 3 cargo run 2>&1 || true
```

Expected: Window opens, title screen appears. Look for `info: Loaded script: assets/scripts/test.bscript.ron` in output. Click to transition to Gameplay. Click again to advance through dialogue lines.

---

### Self-Review Checklist

- **ScriptEngine.load()** → initializes current_script, resets line/call_stack?
- **ScriptEngine.advance()** → returns current cmd and increments line?
- **ScriptEngine.jump_to_label()** → finds Label by name, sets line to next index?
- **process_advance** → on first click (after text shown?), starts executing cmds, breaks at Dialogue?
- **update_text_reveal** → increments text_progress by time delta?
- **Double-click skip** → clicks while text revealing complete text instantly?
- **Click on fully revealed text** → sets is_displaying = false, next click advances?
- **Edge: script with no Dialogue commands** → runs through all, logs info, no crash?
- **Edge: jump to non-existent label** → silently no-ops (graceful degradation)?
- **Edge: empty script** → has_more returns false immediately, no crash?
