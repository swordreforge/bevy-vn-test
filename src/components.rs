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
pub struct SpriteSlotMarker(pub FgPosition);

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
pub struct GalleryRoot;

#[derive(Component)]
pub struct GalleryThumbnail(pub String);

#[derive(Component)]
pub struct GalleryLocked;

#[derive(Component)]
pub struct GalleryFullscreen;

#[derive(Component)]
pub struct GalleryBackButton;
