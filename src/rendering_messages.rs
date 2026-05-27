use bevy::prelude::*;
use crate::script::{FgPosition, Transition};

#[derive(Message)]
pub struct SetBgMessage {
    pub file: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct ShowFgMessage {
    pub char_id: String,
    pub expression: String,
    pub position: FgPosition,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct HideFgMessage {
    pub char_id: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct ShowFaceMessage {
    pub char_id: String,
}

#[derive(Message)]
pub struct HideFaceMessage;

#[derive(Message)]
pub struct ShowCgMessage {
    pub file: String,
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct HideCgMessage {
    pub transition: Option<Transition>,
    pub duration: Option<f64>,
}

#[derive(Message)]
pub struct ScrollBgMessage {
    pub file: String,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub fade: u64,
    #[allow(dead_code)]
    pub wait: bool,
}

#[derive(Message)]
pub struct DrawSpriteMessage {
    pub id: String,
    pub file: String,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub alpha: i32,
    pub priority: i32,
    #[allow(dead_code)]
    pub time: u64,
    pub rotation: f32,
    #[allow(dead_code)]
    pub anchor_x: f32,
    #[allow(dead_code)]
    pub anchor_y: f32,
    pub blend_mode: i32,
}

#[derive(Message)]
pub struct FadeSpriteMessage {
    pub id: String,
    pub time: u64,
}

#[derive(Message)]
pub struct MoveSpriteMessage {
    pub id: String,
    pub x: f32,
    pub y: f32,
    #[allow(dead_code)]
    pub z: i32,
    pub alpha: i32,
    pub time: u64,
    #[allow(dead_code)]
    pub wait: bool,
}

#[derive(Message)]
pub struct AnimateSpriteMessage {
    pub id: String,
    pub file: String,
    pub max: u32,
    pub frame_time: u64,
    pub style: u32,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub rotation: f32,
    pub draw: u32,
    pub alpha: i32,
    pub priority: i32,
    pub wait: bool,
}
