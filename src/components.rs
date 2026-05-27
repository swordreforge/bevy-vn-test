use bevy::prelude::*;
use crate::script::FgPosition;

#[derive(Component)]
pub struct DialogueBox;

#[derive(Component)]
pub struct DialogueTextDisplay;

#[derive(Component)]
pub struct SpeakerNameDisplay;

#[derive(Component)]
pub struct DialogueUiRoot;

#[derive(Component)]
pub struct BackgroundRoot;

#[derive(Component)]
pub struct SpriteSlotMarker(#[allow(dead_code)] pub FgPosition);

#[derive(Component)]
pub struct CgRoot;

#[derive(Component)]
pub struct ChoiceUiRoot;

#[derive(Component)]
pub struct ChoiceButtonIndex(pub usize);

#[derive(Component)]
pub struct SaveLoadUiRoot;

#[derive(Component)]
pub struct SaveSlot(pub usize);

#[derive(Component)]
pub struct ConfirmDialogRoot;

#[derive(Component)]
pub struct ConfirmYesButton;

#[derive(Component)]
pub struct ConfirmNoButton;

#[derive(Component)]
pub struct SaveLoadSlotGrid;

#[derive(Component)]
pub struct SaveLoadPageLeftBtn;

#[derive(Component)]
pub struct SaveLoadPageRightBtn;

#[derive(Component)]
pub struct SaveLoadPageText;

#[derive(Component)]
pub struct GalleryRoot;

#[derive(Component)]
pub struct GalleryThumbnail(pub String);

#[derive(Component)]
pub struct GalleryLocked;

#[derive(Component)]
pub struct GalleryFullscreen;

#[derive(Component)]
pub struct GalleryBackButton;

#[derive(Component)]
pub struct GalleryGridContent;

#[derive(Component)]
pub struct GallerySafeModeBtn;

#[derive(Component)]
pub struct SafeModeLabel;

// === Settings UI Components ===
#[derive(Component)]
pub struct SettingsBackButton;

#[derive(Component)]
pub struct SliderSegment(pub usize);

#[derive(Component, Clone, Copy, PartialEq)]
pub enum SliderSetting {
    BgmVolume,
    SeVolume,
    VoiceVolume,
    TextSpeed,
    MsgOpacity,
}

#[derive(Component)]
pub struct SliderValueText;

#[derive(Component)]
pub struct ToggleOption {
    pub group: String,
    pub value: bool,
}

// === Audio type markers ===
#[derive(Component, Clone, Copy, PartialEq)]
pub enum AudioType {
    Bgm,
    BgmX,
    Se,
    Voice,
}

#[derive(Component)]
pub struct BgmFade {
    pub timer: Timer,
    pub start_mult: f32,
    pub end_mult: f32,
    pub layer: BgmFadeLayer,
}

pub enum BgmFadeLayer {
    Bgm,
    BgmX,
}

#[derive(Component)]
pub struct TransitionOverlay;

#[derive(Component)]
pub struct ScreenOverlayRoot;

#[derive(Component)]
pub struct OverlayTween {
    pub timer: Timer,
    pub start_alpha: f32,
    pub end_alpha: f32,
}

#[derive(Component)]
pub struct FacePortrait;

#[derive(Component)]
pub struct SpriteOverlay {
    pub id: String,
    #[allow(dead_code)]
    pub blend_mode: SpriteBlendMode,
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub enum SpriteBlendMode {
    Normal,
    Add,
    Multiply,
    Screen,
}

impl Default for SpriteBlendMode {
    fn default() -> Self { Self::Normal }
}

impl SpriteBlendMode {
    #[allow(dead_code)]
    pub fn from_attr(s: &str) -> Self {
        match s {
            "1" | "add" => Self::Add,
            "2" | "mul" | "multiply" => Self::Multiply,
            "3" | "screen" => Self::Screen,
            _ => Self::Normal,
        }
    }
}

#[derive(Component)]
pub struct SpriteTween {
    pub timer: Timer,
    pub start_x: f32,
    pub end_x: f32,
    pub start_y: f32,
    pub end_y: f32,
    pub start_alpha: f32,
    pub end_alpha: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub kind: TweenKind,
}

pub enum TweenKind {
    FadeOut,
    FadeIn,
    Move,
}

#[derive(Component)]
pub struct SpriteAnchor {
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub target_x: f32,
    pub target_y: f32,
}

#[derive(Component)]
pub struct BgScroll {
    pub timer: Timer,
    pub start_x: f32,
    pub end_x: f32,
    pub start_y: f32,
    pub end_y: f32,
}
