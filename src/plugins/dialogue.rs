use bevy::prelude::*;
use crate::components::*;
use crate::resources::{DialogueState, GameFont, Settings};
use crate::state::AppState;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueState>()
            .add_systems(OnEnter(AppState::Gameplay), setup_dialogue_ui)
            .add_systems(Update, (
                update_dialogue,
                apply_message_opacity,
            ).run_if(in_state(AppState::Gameplay)))
            .add_systems(OnExit(AppState::Gameplay), cleanup_dialogue);
    }
}

fn setup_dialogue_ui(mut commands: Commands, game_font: Res<GameFont>) {
    commands.spawn((
        DialogueUiRoot,
        DialogueBox,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(200.0),
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexStart,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ZIndex(3),
    ));

    commands.spawn((
        DialogueUiRoot,
        SpeakerNameDisplay,
        Text::new(""),
        TextFont {
            font: game_font.0.clone(),
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.8, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(200.0),
            left: Val::Px(40.0),
            ..default()
        },
        ZIndex(3),
    ));

    commands.spawn((
        DialogueUiRoot,
        DialogueTextDisplay,
        Text::new(""),
        TextFont {
            font: game_font.0.clone(),
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            width: Val::Percent(95.0),
            ..default()
        },
        ZIndex(3),
    ));
}

fn cleanup_dialogue(mut commands: Commands, query: Query<Entity, With<DialogueUiRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn update_dialogue(
    state: Res<DialogueState>,
    mut text_query: Query<&mut Text, (With<DialogueTextDisplay>, Without<SpeakerNameDisplay>)>,
    mut speaker_query: Query<&mut Text, (With<SpeakerNameDisplay>, Without<DialogueTextDisplay>)>,
) {
    if let Ok(mut text) = text_query.single_mut() {
        let end = state.text_progress.min(state.current_text.len());
        text.0 = state.current_text[..end].to_string();
    }
    if let Ok(mut speaker) = speaker_query.single_mut() {
        speaker.0 = state.current_speaker.clone().unwrap_or_default();
    }
}

fn apply_message_opacity(
    settings: Res<Settings>,
    mut query: Query<&mut BackgroundColor, With<DialogueBox>>,
) {
    let alpha = (settings.message_window_opacity as f32) / 100.0;
    for mut bg in query.iter_mut() {
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, alpha));
    }
}
