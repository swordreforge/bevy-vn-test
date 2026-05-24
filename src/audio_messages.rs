use bevy::prelude::*;

#[derive(Message)]
pub struct PlayBgmMessage {
    pub id: String,
    pub volume: Option<f32>,
    pub fade_in: Option<u64>,
}

#[derive(Message)]
pub struct StopBgmMessage {
    pub id: Option<String>,
    pub fade_out: Option<u64>,
}

#[derive(Message)]
pub struct PlaySeMessage {
    pub file: String,
    pub volume: Option<f32>,
}

#[derive(Message)]
pub struct PlayVoiceMessage {
    pub file: String,
    pub volume: Option<f32>,
}
