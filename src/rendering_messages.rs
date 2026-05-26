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
