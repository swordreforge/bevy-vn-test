use bevy::prelude::*;

#[derive(Message)]
pub struct PlayBgmMessage {
    pub id: String,
    pub volume: Option<f32>,
    pub fade_in: Option<u64>,
}

#[derive(Message)]
pub struct StopBgmMessage {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub fade_out: Option<u64>,
}

#[derive(Message)]
pub struct PlayBgmXMessage {
    pub id: String,
    pub volume: Option<f32>,
    pub fade_in: Option<u64>,
}

#[derive(Message)]
pub struct StopBgmXMessage {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub fade_out: Option<u64>,
}

#[derive(Message)]
pub struct PlaySeMessage {
    pub file: String,
    #[allow(dead_code)]
    pub volume: Option<f32>,
}

#[derive(Message)]
pub struct LoopSeMessage {
    pub file: String,
    #[allow(dead_code)]
    pub volume: Option<f32>,
    pub channel: u32,
}

#[derive(Message)]
pub struct StopStreamingSeMessage {
    pub channel: u32,
}

#[derive(Message)]
pub struct PlayVoiceMessage {
    pub file: String,
    #[allow(dead_code)]
    pub volume: Option<f32>,
}
