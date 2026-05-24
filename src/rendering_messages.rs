use bevy::prelude::*;
use crate::script::FgPosition;

#[derive(Message)]
pub struct SetBgMessage {
    pub file: String,
}

#[derive(Message)]
pub struct ShowFgMessage {
    pub char_id: String,
    pub expression: String,
    pub position: FgPosition,
}

#[derive(Message)]
pub struct HideFgMessage {
    pub char_id: String,
}

#[derive(Message)]
pub struct ShowCgMessage {
    pub file: String,
}

#[derive(Message)]
pub struct HideCgMessage;
