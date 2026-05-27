use bevy::prelude::*;
use crate::components::*;
use crate::resources::{DialogueState, GameFont, Settings, WindowOverride};
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
                apply_window_appearance,
            ).chain().run_if(in_state(AppState::Gameplay)))
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
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ZIndex(3),
    )).with_children(|parent| {
        parent.spawn((
            FacePortrait,
            Node {
                width: Val::Px(276.0),
                height: Val::Px(144.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(-40.0),
                ..default()
            },
            ImageNode::default(),
            BackgroundColor(Color::NONE),
            Visibility::Hidden,
            ZIndex(4),
        ));
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
                margin: UiRect::new(Val::Px(236.0), Val::Px(0.0), Val::Px(0.0), Val::Px(4.0)),
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
                margin: UiRect::left(Val::Px(236.0)),
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
    choice_query: Query<Entity, With<ChoiceUiRoot>>,
    mut initialized: ResMut<DialogueInitialized>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    for entity in &choice_query {
        commands.entity(entity).despawn();
    }
    initialized.0 = false;
}

fn update_dialogue(
    state: Res<DialogueState>,
    window_override: Res<WindowOverride>,
    mut text_query: Query<&mut Text, (With<DialogueTextDisplay>, Without<SpeakerNameDisplay>)>,
    mut speaker_query: Query<&mut Text, (With<SpeakerNameDisplay>, Without<DialogueTextDisplay>)>,
    mut root_query: Query<&mut Visibility, (With<DialogueUiRoot>, Without<DialogueTextDisplay>)>,
) {
    if let Ok(mut root_vis) = root_query.single_mut() {
        if !window_override.0 {
            *root_vis = if state.current_text.is_empty() {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        }
    }
    if let Ok(mut text) = text_query.single_mut() {
        let visible_count = state.text_progress.min(state.current_text.chars().count());
        text.0 = state.current_text.chars().take(visible_count).collect();
    }
    if let Ok(mut speaker) = speaker_query.single_mut() {
        speaker.0 = state.current_speaker.clone().unwrap_or_default();
    }
}

fn apply_window_appearance(
    settings: Res<Settings>,
    mut bg_query: Query<&mut BackgroundColor, With<DialogueBox>>,
    mut node_query: Query<&mut Node, With<DialogueUiRoot>>,
) {
    let alpha = (settings.message_window_opacity as f32) / 100.0;
    let tint = match settings.window_color_idx {
        1 => Color::srgba(0.2, 0.3, 0.6, alpha),
        2 => Color::srgba(0.2, 0.5, 0.3, alpha),
        3 => Color::srgba(0.5, 0.2, 0.2, alpha),
        _ => Color::srgba(0.0, 0.0, 0.0, alpha),
    };
    for mut bg in bg_query.iter_mut() {
        *bg = BackgroundColor(tint);
    }
    let is_small = settings.window_design == 1;
    for mut node in node_query.iter_mut() {
        if is_small {
            node.padding = UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(6.0), Val::Px(6.0));
        } else {
            node.padding = UiRect::new(Val::Px(40.0), Val::Px(40.0), Val::Px(12.0), Val::Px(12.0));
        }
    }
}


