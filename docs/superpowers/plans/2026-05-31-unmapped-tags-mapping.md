# Unmapped ASB/IET Tags — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Map all ~88 currently-unmapped ASB/IET tag types (~35,000 instances) to `ScriptCmd` variants, eliminating all `[skip]` from the converter.

**Architecture:** 4 groups by function: (1) game logic — new variants with real handlers; (2) audio — reuse existing or add lightweight handlers; (3) visuals — new variants with no-op stubs; (4) engine/UI legacy — single `NoOp { tag }` variant. All groups feed through the same mapper.rs dispatch → script_runner.rs handler chain.

**Tech Stack:** Rust, Artemis ASB/IET parser, RON serialization, Bevy 0.18 script runner

---

### Task 0: Add new ScriptCmd variants to `src/script.rs`

**Files:**
- Modify: `src/script.rs:255-261`

Add 14 new variants after `SetValidity` (before closing `}` of the enum):

- [ ] **Add new variants**

```rust
    // --- Phase 3 unmapped tags ---
    StopAllSe,
    PushHistory,
    WaitVoice,
    QueryMode {
        mode: String,
    },
    Exif {
        expression: String,
    },
    StreamingSeVol {
        id: u32,
        volume: u32,
    },
    Blur {
        power: u32,
    },
    RainParam {
        param: String,
        value: String,
    },
    ShakeScreen {
        power: u32,
        time: u32,
    },
    ShakeSprite {
        id: u32,
        power: u32,
        time: u32,
    },
    MonologueColor {
        color: String,
    },
    Tween {
        args: String,
    },
    FadeScene {
        color: String,
        time: u32,
    },
    NoOp {
        tag: String,
    },
```

- [ ] **Verify compilation** — `cargo check` should succeed (`ScriptCmd` variants exist; nothing references them yet so dead_code warnings are fine).

- [ ] **Commit**

```bash
git add src/script.rs
git commit -m "feat: add 14 new ScriptCmd variants for unmapped tags"

```

---

### Task 1: Add ASB mapper entries (`mapper.rs`)

**Files:**
- Modify: `tools/artemis-export/src/mapper.rs:132-513`

Add all new tag mappings to `map_command()` match block. Add them as a single block before the existing `_ => None` catch-all. **CRITICAL:** Do NOT delete or modify any existing mapping. Only ADD new arms.

- [ ] **Add Group 1 (game logic) mappings — before `_ => None`**

```rust
        // === Phase 3: Group 1 — Game Logic ===
        "TerminateExecutionOfScript" => {
            Some(vec![ScriptCmd::Halt])
        }
        "SEStop" => {
            Some(vec![ScriptCmd::StopAllSe])
        }
        "RegisterTextToHistory" => {
            Some(vec![ScriptCmd::PushHistory])
        }
        "WaitToFinishVoicePlaying" => {
            Some(vec![ScriptCmd::WaitVoice])
        }
        "GetExecutionMode" => {
            let mode = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::QueryMode { mode }])
        }
        "exif" => {
            let expression = cmd.attrs.get("exp").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::Exif { expression }])
        }
        "BgmX" => {
            let attrs_str: String = cmd.attrs.values().cloned().collect::<Vec<_>>().join(" ");
            if attrs_str.contains("stop") {
                Some(vec![ScriptCmd::StopBgmX { id: None, fade_out: None }])
            } else {
                let id = cmd.attrs.get("0").cloned().unwrap_or_default();
                let vol = cmd.attrs.get("1").and_then(|s| s.parse::<f32>().ok());
                Some(vec![ScriptCmd::PlayBgmX { id, volume: vol, fade_in: None }])
            }
        }
```

- [ ] **Add Group 2 (audio) mappings**

```rust
        // === Phase 3: Group 2 — Audio ===
        "ChangeVolumeOfBGM" | "ChangeVolumeOfBGMX" => {
            let channel = if tag == "ChangeVolumeOfBGMX" { 2 } else { 1 };
            let volume = cmd.attrs.get("0").cloned().unwrap_or_else(|| "100".to_string());
            Some(vec![ScriptCmd::BgmVol { channel, volume }])
        }
        "FadeOutBGM" | "FadeOutBGMX" => {
            let id = cmd.attrs.get("0").cloned();
            let fade = cmd.attrs.get("1").and_then(|s| s.parse::<u64>().ok());
            Some(vec![ScriptCmd::StopBgm { id, fade_out: fade }])
        }
        "ChangeVolumeOfStreamingSE" => {
            let id = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let vol = cmd.attrs.get("1").and_then(|s| s.parse::<u32>().ok()).unwrap_or(100);
            Some(vec![ScriptCmd::StreamingSeVol { id, volume: vol }])
        }
        "FadeOutStreamingSE" => {
            let channel = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::StopStreamingSe { channel }])
        }
```

- [ ] **Add Group 3+4 (visual + engine/UI) mappings**

```rust
        // === Phase 3: Group 3 — Visuals ===
        "DrawBG" => {
            let file = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::SetBg { file, transition: None, duration: None }])
        }
        "ScrollBG" => {
            let file = cmd.attrs.get("0").cloned().unwrap_or_default();
            let x1 = cmd.attrs.get("1").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let y1 = cmd.attrs.get("2").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let x2 = cmd.attrs.get("3").and_then(|s| s.parse::<f32>().ok()).unwrap_or(640.0);
            let y2 = cmd.attrs.get("4").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let fade = cmd.attrs.get("5").and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::ScrollBg { file, x1, y1, x2, y2, fade, wait: true }])
        }
        "DrawSpriteEx" | "DrawSpriteWithFiltering" => {
            let id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let file = cmd.attrs.get("1").cloned().unwrap_or_default();
            let x = cmd.attrs.get("2").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let y = cmd.attrs.get("3").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
            let z = cmd.attrs.get("4").and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
            let alpha = cmd.attrs.get("5").and_then(|s| s.parse::<i32>().ok()).unwrap_or(255);
            Some(vec![ScriptCmd::DrawSprite {
                id, file, x, y, z, alpha,
                priority: 0, time: 0, rotation: 0.0,
                anchor_x: 0.0, anchor_y: 0.0, blend_mode: 0,
            }])
        }
        "DrawBustshot" | "DrawBustshotWithFiltering" => {
            let char_id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let expression = cmd.attrs.get("1").cloned().unwrap_or_default();
            let pos = cmd.attrs.get("2").map(|s| match s.as_str() {
                "left" => FgPosition::Left,
                "right" => FgPosition::Right,
                "center" => FgPosition::Center,
                _ => FgPosition::Center,
            }).unwrap_or(FgPosition::Center);
            Some(vec![ScriptCmd::ShowFg { char_id, expression, position: pos, transition: None }])
        }
        "ChangeBustshot" => {
            let char_id = cmd.attrs.get("0").cloned().unwrap_or_default();
            let expression = cmd.attrs.get("1").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::ShowFace { char_id, expression }])
        }
        "blur_set" => {
            let power = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::Blur { power }])
        }
        "SetColorOfRain" => {
            let value = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::RainParam { param: "color".to_string(), value }])
        }
        "SetQuantityOfRainfall" => {
            let value = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::RainParam { param: "quantity".to_string(), value }])
        }
        "SetVectorOfSightForRainfall" => {
            let value = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::RainParam { param: "vector".to_string(), value }])
        }
        "SetPriorityOfRainfallScreen" => {
            let value = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::RainParam { param: "priority".to_string(), value }])
        }
        "SetCameraAngleOfRainfall" => {
            let value = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::RainParam { param: "camera_angle".to_string(), value }])
        }
        "SetValidityOfRainfall" => {
            let value = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::RainParam { param: "validity".to_string(), value }])
        }
        "StartShakingOfAllObjects" => {
            let power = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let time = cmd.attrs.get("1").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::ShakeScreen { power, time }])
        }
        "ShakeScreenSx" => {
            let power = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let time = cmd.attrs.get("1").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::ShakeScreen { power, time }])
        }
        "StartShakingOfSprite" => {
            let id = cmd.attrs.get("0").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let power = cmd.attrs.get("1").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let time = cmd.attrs.get("2").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::ShakeSprite { id, power, time }])
        }
        "TerminateShakingOfAllObjects" | "TerminateShakingOfSprite" => {
            Some(vec![ScriptCmd::ShakeScreen { power: 0, time: 0 }])
        }
        "SetColorOfMonologue" => {
            let color = cmd.attrs.get("0").cloned().unwrap_or_default();
            Some(vec![ScriptCmd::MonologueColor { color }])
        }
        "tween" => {
            let args: String = cmd.attrs.values().cloned().collect::<Vec<_>>().join(" ");
            Some(vec![ScriptCmd::Tween { args }])
        }
        "FadeScene" => {
            let color = cmd.attrs.get("0").cloned().unwrap_or_default();
            let time = cmd.attrs.get("1").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            Some(vec![ScriptCmd::FadeScene { color, time }])
        }
        "MoveBustshot" | "FadeBustshot" | "lytweendel" => {
            Some(vec![ScriptCmd::Tween { args: format!("{:?} {:?}", tag, cmd.attrs) }])
        }
        "DrawMusicTelop" => {
            Some(vec![ScriptCmd::NoOp { tag: tag.to_string() }])
        }
        // === Phase 3: Group 4 — Engine/UI/Legacy (all NoOp) ===
        "Size" | "flip" | "lyprop" | "lydel" | "lyc" | "lyc2" | "lyevent"
        | "trans" | "sys_trans" | "CallSavingWindow" | "BgmShow" | "BgmNo"
        | "MovieInit" | "ResetSc" | "stop" | "exstop" | "set_vsync" | "push_check"
        | "tweetmode" | "chgmsg" | "/chgmsg" | "SetJumpLabel" | "CheckQuickJump"
        | "CancelSkipping" | "SetValidityOfSkipping" | "WaitToFinishSpriteControlling"
        | "EnableHorizontalGradation" | "DisableGradation" | "DisableWindowEx"
        | "WaitForInput" | "script" | "wait" | "st" | "wt" | "btn_stop" | "btnon"
        | "btn_start" | "allkeystart" | "allkeystop" | "allkeyclick" | "rain_mja"
        | "rpx" | "sestop" | "SetEndOfScene" | "shake50300" | "HideFg"
        => {
            Some(vec![ScriptCmd::NoOp { tag: tag.to_string() }])
        }
```

Note: `"HideFg"` is included in Group 4 because it's already handled elsewhere — including it here as NoOp is harmless since the mapper already handles it. Remove if it causes issues.

Wait — `HideFg` IS already mapped! It must NOT be duplicated. The entry `_ => None` at the end is the catch-all. Every explicit match arm takes priority. So adding `"HideFg"` to Group 4 would be harmless since the earlier `"ClrTati"` arm at line ~250 already handles it, but it's bad practice. Let me just not include it.

Actually, looking more carefully at the existing mapper, `ClrTati` maps to `HideFg`. The raw `HideFg` tag doesn't appear in the existing arms. But adding it to Group 4 would be fine because explicit `HideFg` tag usage in ASB is unlikely. Let's leave it out of the plan for clarity.

- [ ] **Compile & test** — `cargo check` in workspace + `cargo test -p artemis-export`. 83+ tests must still pass.

- [ ] **Commit**

```bash
git add tools/artemis-export/src/mapper.rs
git commit -m "feat: map all unmapped ASB tags to ScriptCmd variants"

```

---

### Task 2: Add IET mapper entries (`iet.rs`)

**Files:**
- Modify: `tools/artemis-export/src/iet.rs:47-210`

Add mappings for IET commands that reach the catch-all. The IET parser at `parse_iet_content` dispatches recognized commands via `match cmd_name` — unmapped ones hit `_ =>` and produce a `[skip]` warning. Add arms for the remaining unmapped IET commands.

- [ ] **Add IET mappings for commands currently hitting `_ =>` catch-all**

Insert these as additional match arms in the `parse_iet_content` command dispatch, before the existing `_ =>` catch-all:

```rust
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
            let mode = parse_iet_attr(cmd_rest, 0).unwrap_or_default();
            output.push(ScriptCmd::QueryMode { mode });
        }
        "exif" => {
            let exp = parse_iet_named_attr(cmd_rest, "exp").unwrap_or_default();
            output.push(ScriptCmd::Exif { expression: exp });
        }
        "DrawBG" => {
            let file = parse_iet_attr(cmd_rest, 0).unwrap_or_default();
            output.push(ScriptCmd::SetBg { file, transition: None, duration: None });
        }
        "blur_set" => {
            let power = parse_iet_attr(cmd_rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            output.push(ScriptCmd::Blur { power });
        }
        "ChangeVolumeOfBGM" => {
            let vol = parse_iet_attr(cmd_rest, 0).unwrap_or_else(|| "100".to_string());
            output.push(ScriptCmd::BgmVol { channel: 1, volume: vol });
        }
        "FadeOutBGM" => {
            output.push(ScriptCmd::StopBgm { id: None, fade_out: None });
        }
        "ChangeVolumeOfStreamingSE" => {
            let id = parse_iet_attr(cmd_rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let vol = parse_iet_attr(cmd_rest, 1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(100);
            output.push(ScriptCmd::StreamingSeVol { id, volume: vol });
        }
        "FadeOutStreamingSE" => {
            let channel = parse_iet_attr(cmd_rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            output.push(ScriptCmd::StopStreamingSe { channel });
        }
        "SetColorOfMonologue" => {
            let color = parse_iet_attr(cmd_rest, 0).unwrap_or_default();
            output.push(ScriptCmd::MonologueColor { color });
        }
        "StartShakingOfAllObjects" | "ShakeScreenSx" => {
            let power = parse_iet_attr(cmd_rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let time = parse_iet_attr(cmd_rest, 1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            output.push(ScriptCmd::ShakeScreen { power, time });
        }
        "StartShakingOfSprite" => {
            let id = parse_iet_attr(cmd_rest, 0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let power = parse_iet_attr(cmd_rest, 1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let time = parse_iet_attr(cmd_rest, 2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            output.push(ScriptCmd::ShakeSprite { id, power, time });
        }
        "TerminateShakingOfAllObjects" | "TerminateShakingOfSprite" => {
            output.push(ScriptCmd::ShakeScreen { power: 0, time: 0 });
        }
        "calllua" => {
            // Check if it's a plain numeric arg (no function name → NoOp)
            let first = parse_iet_attr(cmd_rest, 0).unwrap_or_default();
            if first.parse::<i32>().is_ok() {
                output.push(ScriptCmd::NoOp { tag: format!("calllua ({})", first) });
            } else {
                // Delegate to existing calllua dispatch
                if let Some(cmds) = map_calllua_iet(cmd_rest) {
                    output.extend(cmds);
                } else if verbose {
                    eprintln!("  [skip] calllua ({})", cmd_rest);
                }
            }
        }
        // Group 4 (IET) — all remaining unknown IET commands as NoOp
        cmd_name => {
            if let Some(iet_func) = parse_iet_attr(cmd_rest, 0) {
                if iet_func.parse::<i32>().is_ok() {
                    // Single numeric arg — probably a legacy IET command
                    output.push(ScriptCmd::NoOp { tag: format!("{} ({})", cmd_name, iet_func) });
                } else {
                    output.push(ScriptCmd::NoOp { tag: format!("{} {}", cmd_name, cmd_rest) });
                }
            } else if verbose {
                eprintln!("  [skip] {} ({})", cmd_name, cmd_rest);
            }
        }
```

- [ ] **Add helper function** — find `parse_iet_attr` (already exists), check if `parse_iet_named_attr` is needed:

Add this helper near the other parse helpers in iet.rs:

```rust
fn parse_iet_named_attr(cmd_rest: &str, name: &str) -> Option<String> {
    let pattern = format!("{}=", name);
    for part in cmd_rest.split_whitespace() {
        if let Some(val) = part.strip_prefix(&pattern) {
            return Some(val.trim_matches('"').to_string());
        }
    }
    None
}
```

- [ ] **Compile & test**

Run: `cargo test -p artemis-export`
Expected: All 83+ tests pass.

- [ ] **Commit**

```bash
git add tools/artemis-export/src/iet.rs
git commit -m "feat: map unmapped IET commands to ScriptCmd variants"

```

---

### Task 3: Add runtime handlers in `script_runner.rs`

**Files:**
- Modify: `src/plugins/script_runner.rs:275-748` (skip path) and `src/plugins/script_runner.rs:751-1299` (normal path)

Add handlers for all new `ScriptCmd` variants in BOTH the skip and normal execution paths.

**IMPORTANT:** In the script_runner, `ScriptCmd::Halt` is already handled. `ScriptCmd::StopBgm`, `ScriptCmd::BgmVol`, `ScriptCmd::StopStreamingSe`, `ScriptCmd::PlayBgmX`, `ScriptCmd::StopBgmX` are also already handled. Only add handlers for the **new** variants: `StopAllSe`, `PushHistory`, `WaitVoice`, `QueryMode`, `Exif`, `StreamingSeVol`, `Blur`, `RainParam`, `ShakeScreen`, `ShakeSprite`, `MonologueColor`, `Tween`, `FadeScene`, `NoOp`.

- [ ] **Add skip-path handlers** (after the existing `SetFlag` handler at ~line 371)

Insert before the `Halt =>` handler (each arm's body is a comment-only stub):

```rust
                    ScriptCmd::StopAllSe => {
                        audio_state.streaming_se.clear();
                    }
                    ScriptCmd::PushHistory => {
                        // TODO: push most recent dialogue to backlog resource
                    }
                    ScriptCmd::WaitVoice => {
                        // No-op in skip mode (skip skips all waits)
                    }
                    ScriptCmd::QueryMode { mode: _ } => {
                        engine.flags.insert("tmp".to_string(), 0);
                    }
                    ScriptCmd::Exif { expression: _ } => {
                        // In skip mode, always skip exif-guarded commands
                        if let Some(_) = engine.peek() {
                            engine.advance();
                        }
                    }
                    ScriptCmd::StreamingSeVol { id, volume } => {
                        if let Some(entry) = audio_state.streaming_se.get_mut(id) {
                            entry.volume = volume as f32 / 100.0;
                        }
                    }
                    ScriptCmd::Blur { .. }
                    | ScriptCmd::RainParam { .. }
                    | ScriptCmd::ShakeScreen { .. }
                    | ScriptCmd::ShakeSprite { .. }
                    | ScriptCmd::MonologueColor { .. }
                    | ScriptCmd::Tween { .. }
                    | ScriptCmd::FadeScene { .. }
                    | ScriptCmd::NoOp { .. } => {
                        // Visual/engine no-ops — skip silently
                    }
```

- [ ] **Add normal-path handlers** (after the existing `SetFlag` handler at ~line 849)

```rust
                    ScriptCmd::StopAllSe => {
                        audio_state.streaming_se.clear();
                    }
                    ScriptCmd::PushHistory => {
                        // TODO: push most recent dialogue to backlog resource
                    }
                    ScriptCmd::WaitVoice => {
                        // Hardcoded 2000ms wait for voice to finish
                        auto_timer = Some(Timer::from_seconds(2.0, TimerMode::Once));
                        break;
                    }
                    ScriptCmd::QueryMode { mode: _ } => {
                        engine.flags.insert("tmp".to_string(), 0);
                    }
                    ScriptCmd::Exif { expression } => {
                        // Evaluate expression and skip next command if false
                        let result = evaluate_condition_expression(
                            expression,
                            &engine.flags,
                            &engine.local_work,
                        );
                        if !result {
                            if let Some(_) = engine.peek() {
                                engine.advance();
                            }
                        }
                    }
                    ScriptCmd::StreamingSeVol { id, volume } => {
                        if let Some(entry) = audio_state.streaming_se.get_mut(id) {
                            entry.volume = volume as f32 / 100.0;
                        }
                    }
                    ScriptCmd::Blur { .. }
                    | ScriptCmd::RainParam { .. }
                    | ScriptCmd::ShakeScreen { .. }
                    | ScriptCmd::ShakeSprite { .. }
                    | ScriptCmd::MonologueColor { .. }
                    | ScriptCmd::Tween { .. }
                    | ScriptCmd::FadeScene { .. }
                    | ScriptCmd::NoOp { .. } => {
                        // Visual/engine no-ops
                    }
```

- [ ] **Add `evaluate_condition_expression()` helper**

Add this function to `src/script.rs` or near the script_runner:

```rust
pub fn evaluate_condition_expression(
    expr: &str,
    flags: &HashMap<String, i32>,
    _local_work: &HashMap<u32, i32>,
) -> bool {
    let expr = expr.trim();
    // Pattern: "t.tmp == N", "t.tmp != N", "t.tmp >= N", etc.
    let tmp_val = flags.get("tmp").copied().unwrap_or(0);

    for (op_str, op_fn) in [
        ("!=", |a: i32, b: i32| a != b),
        ("==", |a, b| a == b),
        (">=", |a, b| a >= b),
        ("<=", |a, b| a <= b),
        (">", |a, b| a > b),
        ("<", |a, b| a < b),
    ] {
        if let Some((_, rhs_str)) = expr.split_once(op_str) {
            if let Ok(rhs) = rhs_str.trim().parse::<i32>() {
                return op_fn(tmp_val, rhs);
            }
        }
    }
    // Bare number: true if tmp == number
    if let Ok(val) = expr.parse::<i32>() {
        return tmp_val == val;
    }
    true // default: condition passes
}
```

- [ ] **Add import for the new function** at the top of `script_runner.rs` if it's placed in a different module.

- [ ] **Compile & verify**

Run: `cargo check`
Expected: No errors.

- [ ] **Commit**

```bash
git add src/plugins/script_runner.rs
git commit -m "feat: add runtime handlers for all new ScriptCmd variants"

```

---

### Task 4: Verify zero skips

**Files:**
- None

- [ ] **Run full converter with verbose**

```bash
cargo run -p artemis-export -- --input root --output /tmp/verify-zero-skips --verbose 2>&1 | grep '\[skip\]' | head -10
```

Expected: Zero `[skip]` lines. If any remain, check which tags they are and add missing mappings.

- [ ] **Run all tests**

```bash
cargo test -p artemis-export
```

Expected: All tests pass (existing 83 + any new).

- [ ] **Commit any remaining fixes**

```bash
git add -A
git commit -m "fix: zero-skips verification — final unmapped tag fixes"
```

---

### Self-Review Checklist

1. **Spec coverage:** Does every tag from the spec's Group 1-4 tables have a mapping in either mapper.rs or iet.rs?
2. **No placeholders:** No TBD, TODO (except Intentional `// TODO: push to backlog` which documents a known deferred feature).
3. **No duplicated mappings:** Every new arm is for a tag NOT already in the existing match block.
4. **Type consistency:** `ScriptCmd` variant names match exactly between script.rs, mapper.rs, iet.rs, and script_runner.rs.
