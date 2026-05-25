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
    PlaySe {
        file: String,
        volume: Option<f32>,
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
    PlayMovie {
        file: String,
    },
    Label {
        name: String,
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
    GreaterEqual,
    LessEqual,
}

pub type Script = Vec<ScriptCmd>;

#[derive(Resource, Default)]
pub struct ScriptEngine {
    pub current_script: String,
    pub current_line: usize,
    pub call_stack: Vec<(String, usize)>,
    pub flags: HashMap<String, i32>,
    pub scripts: HashMap<String, Vec<ScriptCmd>>,
}

impl ScriptEngine {
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
}
