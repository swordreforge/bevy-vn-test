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
