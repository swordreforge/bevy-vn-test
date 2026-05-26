use bevy::prelude::*;
use crate::components::*;
use crate::resources::{DialogueState, GameFont, Settings};
use crate::state::AppState;

#[derive(Resource, Default)]
pub struct DialogueInitialized(pub bool);

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueState>()
            .init_resource::<DialogueInitialized>()
            .add_systems(OnEnter(AppState::Gameplay), setup_dialogue_ui)
            .add_systems(Update, (
                update_dialogue,
                apply_message_opacity,
            ).run_if(in_state(AppState::Gameplay)))
            .add_systems(OnExit(AppState::Gameplay), hide_dialogue)
            .add_systems(OnEnter(AppState::Title), cleanup_dialogue);
    }
}

fn setup_dialogue_ui(
    mut commands: Commands,
    game_font: Res<GameFont>,
    mut initialized: ResMut<DialogueInitialized>,
    mut show_query: Query<&mut Visibility, With<DialogueUiRoot>>,
) {
    if initialized.0 {
        for mut vis in show_query.iter_mut() {
            *vis = Visibility::Visible;
        }
        return;
    }
    commands.spawn((
        DialogueUiRoot,
        DialogueBox,
        Node {
            width: Val::Percent(92.0),
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Percent(4.0),
            right: Val::Percent(4.0),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexStart,
            flex_direction: FlexDirection::Column,
            padding: UiRect::new(Val::Px(40.0), Val::Px(40.0), Val::Px(12.0), Val::Px(12.0)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ZIndex(3),
    )).with_children(|parent| {
        parent.spawn((
            SpeakerNameDisplay,
            Text::new(""),
            TextFont {
                font: game_font.0.clone(),
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.8, 0.6)),
            Node {
                margin: UiRect::bottom(Val::Px(4.0)),
                ..default()
            },
            ZIndex(3),
        ));

        parent.spawn((
            DialogueTextDisplay,
            Text::new(""),
            TextFont {
                font: game_font.0.clone(),
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
            ZIndex(3),
        ));
    });
    initialized.0 = true;
}

fn hide_dialogue(
    mut hide_query: Query<&mut Visibility, With<DialogueUiRoot>>,
) {
    for mut vis in hide_query.iter_mut() {
        *vis = Visibility::Hidden;
    }
}

fn cleanup_dialogue(
    mut commands: Commands,
    query: Query<Entity, With<DialogueUiRoot>>,
    mut initialized: ResMut<DialogueInitialized>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    initialized.0 = false;
}

fn update_dialogue(
    state: Res<DialogueState>,
    mut text_query: Query<&mut Text, (With<DialogueTextDisplay>, Without<SpeakerNameDisplay>)>,
    mut speaker_query: Query<&mut Text, (With<SpeakerNameDisplay>, Without<DialogueTextDisplay>)>,
    mut root_query: Query<&mut Visibility, (With<DialogueUiRoot>, Without<DialogueTextDisplay>)>,
) {
    if let Ok(mut root_vis) = root_query.single_mut() {
        *root_vis = if state.current_text.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    if let Ok(mut text) = text_query.single_mut() {
        let visible_count = state.text_progress.min(state.current_text.chars().count());
        text.0 = state.current_text.chars().take(visible_count).collect();
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
