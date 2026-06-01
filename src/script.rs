use crate::resources::RouteConfig;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum FgPosition {
    Left,
    Center,
    Right,
    OffScreen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Transition {
    Fade,
    Dissolve,
    Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptCmd {
    Dialogue {
        speaker: Option<String>,
        text: String,
    },
    Choice {
        options: Vec<ChoiceOption>,
    },
    SetBg {
        file: String,
        transition: Option<Transition>,
        duration: Option<u64>,
    },
    ShowFg {
        char_id: String,
        expression: String,
        position: FgPosition,
        transition: Option<Transition>,
    },
    HideFg {
        char_id: String,
        transition: Option<Transition>,
    },
    ShowFace {
        char_id: String,
        expression: String,
    },
    HideFace {
        char_id: String,
    },
    ShowCg {
        file: String,
        transition: Option<Transition>,
    },
    HideCg {
        transition: Option<Transition>,
    },
    PlayBgm {
        id: String,
        volume: Option<f32>,
        fade_in: Option<u64>,
    },
    StopBgm {
        id: Option<String>,
        fade_out: Option<u64>,
    },
    PlayBgmX {
        id: String,
        volume: Option<f32>,
        fade_in: Option<u64>,
    },
    StopBgmX {
        id: Option<String>,
        fade_out: Option<u64>,
    },
    PlaySe {
        file: String,
        volume: Option<f32>,
    },
    LoopSe {
        file: String,
        volume: Option<f32>,
        channel: u32,
    },
    StopStreamingSe {
        channel: u32,
    },
    PlayVoice {
        file: String,
    },
    AffectionChange {
        char_id: String,
        delta: i32,
    },
    AffectionCondition {
        char_id: String,
        value: i32,
        operator: ConditionOp,
        goto: String,
    },
    Jump {
        target: String,
    },
    Call {
        target: String,
    },
    CallScript {
        script: String,
        label: Option<String>,
    },
    Return,
    Condition {
        var: String,
        value: i32,
        operator: ConditionOp,
        goto: String,
    },
    SavePoint,
    UnlockCg {
        file: String,
    },
    ClearText,
    Wait {
        duration: u64,
    },
    DrawSprite {
        id: String,
        file: String,
        x: f32,
        y: f32,
        z: i32,
        alpha: i32,
        priority: i32,
        #[serde(default)]
        time: u64,
        #[serde(default)]
        rotation: f32,
        #[serde(default)]
        anchor_x: f32,
        #[serde(default)]
        anchor_y: f32,
        #[serde(default)]
        blend_mode: i32,
    },
    FadeSprite {
        id: String,
        time: u64,
    },
    MoveSprite {
        id: String,
        x: f32,
        y: f32,
        z: i32,
        alpha: i32,
        time: u64,
        wait: bool,
    },
    PlayMovie {
        file: String,
    },
    Label {
        name: String,
    },
    SetFlag {
        name: String,
        value: i32,
    },
    Halt,
    ScreenOverlay {
        color: OverlayColor,
        time: u64,
    },
    ClearOverlay {
        time: u64,
    },
    Window {
        show: bool,
        #[allow(dead_code)]
        time: Option<u64>,
    },
    ChangeWindowColor {
        color_idx: i32,
    },
    ChangeWindowDesign {
        design: i32,
    },
    BgmVol {
        channel: u32,
        volume: String,
    },
    Quake {
        power: f32,
        time: u64,
    },
    Flash {
        color: OverlayColor,
        time: u64,
        alpha: u8,
    },
    ScrollBg {
        file: String,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        fade: u64,
        wait: bool,
    },
    AnimateSprite {
        id: String,
        file: String,
        max: u32,
        frame_time: u64,
        style: u32,
        x: f32,
        y: f32,
        z: i32,
        anchor_x: f32,
        anchor_y: f32,
        rotation: f32,
        draw: u32,
        alpha: i32,
        priority: i32,
        wait: bool,
    },
    View {
        char_id: String,
    },
    SetGlobalFlag {
        index: u32,
        value: i32,
    },
    GetGlobalFlag {
        index: u32,
    },
    SetLocalFlag {
        index: u32,
        value: i32,
    },
    StoreValueToLocalWork {
        index: u32,
        value: i32,
        #[serde(default)]
        expression: Option<String>,
    },
    LoadValueFromLocalWork {
        index: u32,
    },
    GetLocalFlag {
        index: u32,
    },
    RouteFlag,
    GameMode {
        mode: u32,
    },
    SetValidity {
        mode: ValidityMode,
        allowed: bool,
    },
    Exif {
        expression: String,
    },
    MovieInit,
    DrawSpriteEx {
        id: String,
        file: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        blend_mode: i32,
        display_mode: i32,
        priority: i32,
        visible: bool,
        wait: bool,
    },
    WaitToFinishMoviePlayingOnSprite {
        sprite_id: String,
    },
    RainMja {
        file: String,
        loop_file: Option<String>,
        priority: i32,
        time: Option<u64>,
    },
    SetRainValid {
        enabled: bool,
    },
    SetRainQuantity {
        density: u32,
    },
    SetRainColor {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    SetRainVector {
        direction: u32,
    },
    SetRainCameraAngle {
        x: u32,
        y: u32,
        z: u32,
    },
    SetRainPriority {
        priority: u32,
    },
    // --- Phase 3 unmapped tags ---
    StopAllSe,
    PushHistory,
    WaitVoice,
    QueryMode {
        mode: String,
    },
    StreamingSeVol {
        id: u32,
        volume: u32,
    },
    Blur {
        power: u32,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidityMode {
    Saving,
    Loading,
    Input,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceOption {
    pub text: String,
    pub affection_change: Option<(String, i32)>,
    pub goto: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOp {
    Greater,
    Less,
    Equal,
    NotEqual,
    GreaterEqual,
    LessEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverlayColor {
    Black,
    White,
}

#[allow(dead_code)]
pub type Script = Vec<ScriptCmd>;

#[derive(Resource, Default)]
pub struct ScriptEngine {
    pub current_script: String,
    pub current_line: usize,
    pub call_stack: Vec<(String, usize)>,
    pub flags: HashMap<String, i32>,
    pub global_flags: HashMap<u32, i32>,
    pub local_work: HashMap<u32, i32>,
    pub local_flags: HashMap<u32, i32>,
    pub scripts: HashMap<String, Vec<ScriptCmd>>,
    pub dialogue_idx: usize,
    pub finished: bool,
}

impl ScriptEngine {
    #[allow(dead_code)]
    pub fn load(&mut self, name: &str, script: Vec<ScriptCmd>) {
        self.current_script = name.to_string();
        self.scripts.insert(name.to_string(), script);
        self.current_line = 0;
        self.call_stack.clear();
    }

    pub fn advance(&mut self) -> Option<&ScriptCmd> {
        let idx = self.current_line;
        let cmd = self.scripts.get(&self.current_script)?.get(idx)?;
        self.current_line = idx + 1;
        Some(cmd)
    }

    #[allow(dead_code)]
    pub fn peek(&self) -> Option<&ScriptCmd> {
        self.scripts
            .get(&self.current_script)?
            .get(self.current_line)
    }

    pub fn jump_to_label(&mut self, label: &str) -> bool {
        let Some(script) = self.scripts.get(&self.current_script) else {
            return false;
        };
        for (i, cmd) in script.iter().enumerate() {
            if let ScriptCmd::Label { name } = cmd {
                if name == label {
                    self.current_line = i + 1;
                    return true;
                }
            }
        }
        false
    }

    pub fn call_label(&mut self, label: &str) {
        let return_line = self.current_line;
        if self.jump_to_label(label) {
            self.call_stack.push((self.current_script.clone(), return_line));
        }
    }

    pub fn call_script(&mut self, script: &str, label: Option<&str>) {
        let return_line = self.current_line;
        let return_script = self.current_script.clone();

        if self.scripts.contains_key(script) {
            self.current_script = script.to_string();
            self.dialogue_idx = 0;
            if let Some(lbl) = label {
                self.jump_to_label(lbl);
            } else {
                self.current_line = 0;
            }
            self.call_stack.push((return_script, return_line));
        }
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
            .is_some_and(|s| self.current_line < s.len())
    }

    pub fn next_script(&mut self) -> bool {
        let next = self.find_next_script();
        if let Some(name) = next {
            self.current_script = name;
            self.current_line = 0;
            self.dialogue_idx = 0;
            true
        } else {
            false
        }
    }

    fn find_next_script(&self) -> Option<String> {
        let current = &self.current_script;
        let num_start = current.find(|c: char| c.is_ascii_digit())?;
        let prefix = &current[..num_start];
        let num_part = &current[num_start..];
        let width = num_part.len();
        let num: u32 = num_part.parse().ok()?;
        let next_name = format!("{}{:0>width$}", prefix, num + 10, width = width);
        if self.scripts.contains_key(&next_name) {
            Some(next_name)
        } else {
            None
        }
    }

    pub fn detect_route_completion(&self, config: &RouteConfig) -> Option<String> {
        config.find_by_script(&self.current_script).map(|e| e.name.clone())
    }
}

pub fn evaluate_condition_expression(expr: &str, flags: &HashMap<String, i32>) -> bool {
    let expr = expr.trim();
    let tmp_val = flags.get("tmp").copied().unwrap_or(0);

    if let Some(rhs) = expr.strip_prefix("!=").or_else(|| expr.strip_prefix("=="))
        .or_else(|| expr.strip_prefix(">=")).or_else(|| expr.strip_prefix("<="))
        .or_else(|| expr.strip_prefix(">")).or_else(|| expr.strip_prefix("<"))
    {
        if let Ok(rhs_val) = rhs.trim().parse::<i32>() {
            return if expr.starts_with("!=") { tmp_val != rhs_val }
                else if expr.starts_with("==") { tmp_val == rhs_val }
                else if expr.starts_with(">=") { tmp_val >= rhs_val }
                else if expr.starts_with("<=") { tmp_val <= rhs_val }
                else if expr.starts_with(">") { tmp_val > rhs_val }
                else { tmp_val < rhs_val };
        }
    }
    // Try split_once for "t.tmp == N" style
    for op_str in &["!=", "==", ">=", "<=", ">", "<"] {
        if let Some((_, rhs_str)) = expr.split_once(op_str) {
            if let Ok(rhs) = rhs_str.trim().parse::<i32>() {
                return match *op_str {
                    "!=" => tmp_val != rhs,
                    "==" => tmp_val == rhs,
                    ">=" => tmp_val >= rhs,
                    "<=" => tmp_val <= rhs,
                    ">" => tmp_val > rhs,
                    _ => tmp_val < rhs,
                };
            }
        }
    }
    if let Ok(val) = expr.parse::<i32>() {
        return tmp_val == val;
    }
    true
}

pub fn evaluate_script_expression(expr: &str, flags: &HashMap<String, i32>) -> i32 {
    let resolved = if expr.contains("t.tmp") {
        let tmp_val = flags.get("tmp").copied().unwrap_or(0);
        expr.replace("t.tmp", &tmp_val.to_string())
    } else {
        expr.to_string()
    };
    if let Some((left, right)) = resolved.split_once('+') {
        let l = left.trim().parse::<i32>().unwrap_or(0);
        let r = right.trim().parse::<i32>().unwrap_or(0);
        l + r
    } else if let Some((left, right)) = resolved.split_once('-') {
        let l = left.trim().parse::<i32>().unwrap_or(0);
        let r = right.trim().parse::<i32>().unwrap_or(0);
        l - r
    } else {
        resolved.trim().parse::<i32>().unwrap_or(0)
    }
}
