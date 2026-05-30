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
}
