use crate::script::FgPosition;
use crate::state::AppState;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

pub struct SpriteFade {
    pub timer: Timer,
    pub kind: SpriteFadeKind,
}

pub enum SpriteFadeKind {
    FadeIn,
    FadeOut,
}

pub struct CgFade {
    pub timer: Timer,
    pub kind: CgFadeKind,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CgFadeKind {
    FadeIn,
    FadeOut,
}

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
    #[serde(default)]
    pub global_flags: HashMap<u32, i32>,
    pub affection: HashMap<String, i32>,
    #[serde(default)]
    pub unlock_state: UnlockState,
    pub play_time: u64,
    #[serde(default)]
    pub window_color_idx: i32,
    #[serde(default)]
    pub view_char_id: Option<String>,
    #[serde(default)]
    pub bgm_id: Option<String>,
    #[serde(default)]
    pub bgmx_id: Option<String>,
}

#[derive(Resource, Clone)]
pub struct SaveDir(pub String);

impl Default for SaveDir {
    fn default() -> Self {
        let path = persist_dir();
        let _ = std::fs::create_dir_all(&path);
        Self(path)
    }
}

pub fn persist_dir() -> String {
    #[cfg(feature = "android")]
    {
        if let Some(app) = bevy_android::ANDROID_APP.get() {
            if let Some(path) = app.internal_data_path() {
                return format!("{}", path.display());
            }
        }
    }
    #[cfg(not(feature = "android"))]
    {
        if let Some(dir) = dirs::data_local_dir() {
            let dir = dir.join("bevy-vn");
            let _ = std::fs::create_dir_all(&dir);
            return format!("{}", dir.display());
        }
    }
    "saves".to_string()
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

    pub fn refresh_from_disk(&mut self, save_dir: &SaveDir) {
        for i in 0..self.slots.len() {
            let path = format!("{}/slot_{}.json", save_dir.0, i);
            match std::fs::read_to_string(&path) {
                Ok(json) => self.slots[i] = serde_json::from_str(&json).ok(),
                Err(_) => self.slots[i] = None,
            }
        }
    }

    pub fn save_slot(&mut self, idx: usize, data: SaveData, save_dir: &SaveDir) {
        let _ = std::fs::create_dir_all(&save_dir.0);
        let path = format!("{}/slot_{}.json", save_dir.0, idx);
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&path, &json);
        }
        self.slots[idx] = Some(data);
    }

    pub fn load_slot_from_disk(&mut self, idx: usize, save_dir: &SaveDir) -> Option<SaveData> {
        let path = format!("{}/slot_{}.json", save_dir.0, idx);
        let json = std::fs::read_to_string(path).ok()?;
        let data: SaveData = serde_json::from_str(&json).ok()?;
        self.slots[idx] = Some(data.clone());
        Some(data)
    }
}

pub fn load_settings() -> Settings {
    let dir = persist_dir();
    let path = format!("{}/settings.json", dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) {
    let dir = persist_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = format!("{}/settings.json", dir);
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, &json);
    }
}

pub fn load_unlock_state() -> UnlockState {
    let dir = persist_dir();
    let path = format!("{}/unlock_state.json", dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|json| serde_json::from_str::<UnlockState>(&json).ok())
        .unwrap_or_default()
}

pub fn save_unlock_state(state: &UnlockState) {
    let dir = persist_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = format!("{}/unlock_state.json", dir);
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&path, &json);
    }
}

#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub bgm_volume: f32,
    pub se_volume: f32,
    pub voice_volume: f32,
    pub text_speed: u32,
    pub auto_mode: bool,
    pub skip_mode: bool,
    #[serde(default = "default_auto_delay")]
    pub auto_delay_secs: f32,
    pub message_window_opacity: u8,
    pub window_color_idx: i32,
    pub window_design: i32,
    pub click_to_advance: bool,
}

fn default_auto_delay() -> f32 {
    1.5
}

impl Settings {
    pub fn set_auto_mode(&mut self, value: bool) {
        self.auto_mode = value;
        if value {
            self.skip_mode = false;
        }
    }

    pub fn set_skip_mode(&mut self, value: bool) {
        self.skip_mode = value;
        if value {
            self.auto_mode = false;
        }
    }
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
            auto_delay_secs: 1.5,
            message_window_opacity: 70,
            window_color_idx: 0,
            window_design: 0,
            click_to_advance: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct ViewBlocking(pub bool);

include!(concat!(env!("OUT_DIR"), "/game_data.rs"));

#[derive(Resource)]
pub struct AllCgFiles(pub Vec<String>);

impl AllCgFiles {
    pub fn scan() -> Self {
        let mut files: Vec<String> = all_cg_files().into_iter().map(String::from).collect();
        files.sort();
        Self(files)
    }
}

impl Default for AllCgFiles {
    fn default() -> Self {
        Self::scan()
    }
}

#[derive(Debug, Resource, Default, Clone, Serialize, Deserialize)]
pub struct UnlockState {
    pub cg_unlocked: HashSet<String>,
    #[allow(dead_code)]
    pub bgm_unlocked: HashSet<String>,
    #[allow(dead_code)]
    pub scene_cleared: HashSet<String>,
}

#[derive(Resource, Default)]
pub struct DialogueState {
    pub current_text: String,
    pub current_speaker: Option<String>,
    pub text_progress: usize,
    pub is_displaying: bool,
    #[allow(dead_code)]
    pub text_queue: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BacklogEntry {
    pub speaker: Option<String>,
    pub text: String,
    #[allow(dead_code)]
    pub voice_file: Option<String>,
}

#[derive(Resource, Default)]
pub struct Backlog {
    pub entries: Vec<BacklogEntry>,
}

pub struct BgCrossFade {
    pub timer: Timer,
}

/// Tracks background state with dual-buffer entities
#[derive(Resource)]
pub struct BgState {
    pub entities: [Entity; 2],
    pub active_idx: usize,
    pub fade: Option<BgCrossFade>,
}

impl Default for BgState {
    fn default() -> Self {
        Self {
            entities: [Entity::PLACEHOLDER; 2],
            active_idx: 0,
            fade: None,
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
    pub fade: Option<SpriteFade>,
}

/// Tracks CG overlay state
#[derive(Resource, Default)]
pub struct CgState {
    pub active: bool,
    pub entity: Option<Entity>,
    pub texture: Option<Handle<Image>>,
    pub fade: Option<CgFade>,
}

/// On-demand texture cache
#[derive(Resource, Default)]
pub struct TextureCache {
    pub cache: HashMap<String, Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct SpriteOverlayManager {
    pub sprites: HashMap<String, Entity>,
}

pub struct PendingBgmLoad {
    pub id: String,
    pub handle_a: Handle<AudioSource>,
    pub handle_b: Handle<AudioSource>,
    pub volume: f32,
    pub has_fade: bool,
    pub fade_in_sec: f32,
    pub frames_waited: u32,
}

#[derive(Resource, Default)]
pub struct PendingBgm(pub Option<PendingBgmLoad>);

#[derive(Resource, Default)]
pub struct BgmManager {
    pub current_id: Option<String>,
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct BgmXManager {
    pub current_id: Option<String>,
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct VoiceManager {
    pub entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct SeManager {
    pub entities: HashMap<u32, Entity>,
}

pub enum SeKind {
    OneShot,
    Loop { channel: u32, volume: f32 },
}

pub struct PendingSeLoad {
    pub file: String,
    pub handle_a: Handle<AudioSource>,
    pub handle_b: Option<Handle<AudioSource>>,
    pub handle_single: Handle<AudioSource>,
    pub kind: SeKind,
    pub frames_waited: u32,
}

#[derive(Resource, Default)]
pub struct PendingSe(pub Vec<PendingSeLoad>);

#[derive(Resource, Default)]
pub struct WindowOverride(pub bool);

#[derive(Resource, Default)]
pub struct ObjFileIndex(pub std::collections::HashMap<String, String>);

#[derive(Resource, Default)]
pub struct ChoiceState {
    pub active: bool,
    pub options: Vec<crate::script::ChoiceOption>,
}

#[derive(Resource, Default)]
pub struct IntroPhase(pub bool);

#[derive(Resource, Default)]
pub struct QuakeState {
    pub timer: Option<Timer>,
    pub intensity: f32,
}

#[derive(Resource, Default)]
pub struct SaveLoadMode(pub bool); // true = Save, false = Load

#[derive(Resource, Default)]
pub struct SaveLoadPage(pub usize);

#[derive(Resource, Default)]
pub struct GalleryState {
    pub fullscreen: Option<String>,
    pub page: usize,
}

#[derive(Resource)]
pub struct SafeMode(pub bool);

impl Default for SafeMode {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Resource)]
pub struct GameFont(pub Handle<bevy::text::Font>);

#[derive(Resource, Default)]
pub struct ScreenTransition {
    pub overlay: Option<Entity>,
    pub phase: TransitionPhase,
    pub pending_state: Option<AppState>,
}

pub enum TransitionPhase {
    Idle,
    FadingToBlack { timer: Timer },
    FadingFromBlack { timer: Timer },
}

impl Default for TransitionPhase {
    fn default() -> Self {
        Self::Idle
    }
}
