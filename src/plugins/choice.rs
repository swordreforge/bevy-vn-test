use bevy::prelude::*;
use crate::choice_messages::ChoiceSelectedMessage;
use crate::components::{ChoiceUiRoot, ChoiceButtonIndex};
use crate::resources::ChoiceState;

pub struct ChoicePlugin;

impl Plugin for ChoicePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ChoiceState>()
            .add_message::<ChoiceSelectedMessage>()
            .add_systems(Update, (
                choice_ui_spawn.run_if(|state: Res<ChoiceState>| state.active),
                handle_choice_selection,
                choice_ui_cleanup.run_if(|state: Res<ChoiceState>| !state.active),
            ));
    }
}

const CHOICE_BG_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
const CHOICE_BUTTON_COLOR: Color = Color::srgba(0.15, 0.15, 0.15, 0.95);
const CHOICE_BUTTON_HOVER: Color = Color::srgba(0.35, 0.35, 0.35, 0.95);
const CHOICE_BUTTON_PRESSED: Color = Color::srgba(0.25, 0.25, 0.25, 0.95);

fn choice_ui_spawn(
    mut commands: Commands,
    state: Res<ChoiceState>,
    existing: Query<Entity, With<ChoiceUiRoot>>,
) {
    if !existing.is_empty() {
        return;
    }

    let mut parent = commands.spawn((
        ChoiceUiRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            ..default()
        },
        BackgroundColor(CHOICE_BG_COLOR),
        ZIndex(4),
    ));

    for (i, option) in state.options.iter().enumerate() {
        parent.with_child((
            ChoiceButtonIndex(i),
            Button,
            Node {
                width: Val::Px(768.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(CHOICE_BUTTON_COLOR),
            Text::new(&option.text),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    }
}

fn handle_choice_selection(
    mut query: Query<(&Interaction, &ChoiceButtonIndex, &mut BackgroundColor), Changed<Interaction>>,
    mut writer: MessageWriter<ChoiceSelectedMessage>,
    state: Res<ChoiceState>,
) {
    if !state.active {
        return;
    }

    for (interaction, index, mut bg) in query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(CHOICE_BUTTON_PRESSED);
                writer.write(ChoiceSelectedMessage { index: index.0 });
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(CHOICE_BUTTON_HOVER);
            }
            Interaction::None => {
                *bg = BackgroundColor(CHOICE_BUTTON_COLOR);
            }
        }
    }
}

fn choice_ui_cleanup(
    mut commands: Commands,
    existing: Query<Entity, With<ChoiceUiRoot>>,
) {
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
}
