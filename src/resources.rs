use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use crate::script::FgPosition;

#[derive(Resource, Default, Debug, Clone)]
pub struct AffectionMap(pub HashMap<String, i32>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub timestamp: String,
    pub scene_name: String,
    pub script_path: String,
    pub script_line: usize,
    pub call_stack: Vec<(String, usize)>,
    pub flags: HashMap<String, i32>,
    pub affection: HashMap<String, i32>,
    pub play_time: u64,
}

#[derive(Resource, Default)]
pub struct SaveManager {
    pub slots: Vec<Option<SaveData>>,
}

impl SaveManager {
    pub fn new(slot_count: usize) -> Self {
        Self {
            slots: vec![None; slot_count],
        }
    }

    pub fn refresh_from_disk(&mut self) {
        for i in 0..self.slots.len() {
            let path = format!("saves/slot_{}.json", i);
            match std::fs::read_to_string(&path) {
                Ok(json) => self.slots[i] = serde_json::from_str(&json).ok(),
                Err(_) => self.slots[i] = None,
            }
        }
    }

    pub fn save_slot(&mut self, idx: usize, data: SaveData) {
        let _ = std::fs::create_dir_all("saves");
        let path = format!("saves/slot_{}.json", idx);
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&path, &json);
        }
        self.slots[idx] = Some(data);
    }

    pub fn load_slot_from_disk(&mut self, idx: usize) -> Option<SaveData> {
        let path = format!("saves/slot_{}.json", idx);
        let json = std::fs::read_to_string(path).ok()?;
        let data: SaveData = serde_json::from_str(&json).ok()?;
        self.slots[idx] = Some(data.clone());
        Some(data)
    }
}

#[derive(Resource, Clone)]
pub struct Settings {
    pub bgm_volume: f32,
    pub se_volume: f32,
    pub voice_volume: f32,
    pub text_speed: u32,
    pub auto_mode: bool,
    pub skip_mode: bool,
    pub message_window_opacity: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            bgm_volume: 0.8,
            se_volume: 0.8,
            voice_volume: 1.0,
            text_speed: 40,
            auto_mode: false,
            skip_mode: false,
            message_window_opacity: 70,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct UnlockState {
    pub cg_unlocked: HashSet<String>,
    pub bgm_unlocked: HashSet<String>,
    pub scene_cleared: HashSet<String>,
}

#[derive(Resource, Default)]
pub struct DialogueState {
    pub current_text: String,
    pub current_speaker: Option<String>,
    pub text_progress: usize,
    pub is_displaying: bool,
    pub text_queue: Vec<String>,
}

/// Tracks background state with dual-buffer entities
#[derive(Resource)]
pub struct BgState {
    pub entities: [Entity; 2],
    pub active_idx: usize,
}

impl Default for BgState {
    fn default() -> Self {
        Self {
            entities: [Entity::PLACEHOLDER; 2],
            active_idx: 0,
        }
    }
}

/// Tracks which character sprite occupies each position slot
#[derive(Resource, Default)]
pub struct SpriteManager {
    pub slots: HashMap<FgPosition, SpriteSlotInfo>,
}

pub struct SpriteSlotInfo {
    pub char_id: String,
    pub expression: String,
    pub entity: Entity,
    pub texture: Option<Handle<Image>>,
}

/// Tracks CG overlay state
#[derive(Resource, Default)]
pub struct CgState {
    pub active: bool,
    pub entity: Option<Entity>,
    pub texture: Option<Handle<Image>>,
}

/// On-demand texture cache
#[derive(Resource, Default)]
pub struct TextureCache {
    pub cache: HashMap<String, Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct BgmManager {
    pub current_id: Option<String>,
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct ChoiceState {
    pub active: bool,
    pub options: Vec<crate::script::ChoiceOption>,
}

#[derive(Resource, Default)]
pub struct SaveLoadMode(pub bool); // true = Save, false = Load

#[derive(Resource, Default)]
pub struct GalleryState {
    pub fullscreen: Option<String>,
}
