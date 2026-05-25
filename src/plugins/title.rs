use bevy::prelude::*;
use crate::resources::GameFont;
use crate::resources::ScreenTransition;
use crate::resources::SaveLoadMode;
use crate::state::AppState;

pub struct TitlePlugin;

#[derive(Component)]
struct TitleRoot;

#[derive(Component)]
enum TitleButtonAction {
    NewGame,
    LoadGame,
    Settings,
    Gallery,
}

const BTN_COLOR: Color = Color::srgba(0.08, 0.08, 0.12, 0.85);
const BTN_HOVER: Color = Color::srgba(0.25, 0.25, 0.35, 0.9);
const BTN_W: f32 = 280.0;
const BTN_H: f32 = 48.0;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Title), setup_title)
            .add_systems(Update, handle_title_buttons.run_if(in_state(AppState::Title)))
            .add_systems(OnExit(AppState::Title), cleanup_title);
    }
}

fn setup_title(mut commands: Commands, asset_server: Res<AssetServer>, game_font: Res<GameFont>) {
    commands.spawn((
        TitleRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode::new(asset_server.load("images/title/bg.png")),
    ));

    commands.spawn((
        TitleRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            position_type: PositionType::Absolute,
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn((
            ImageNode::new(asset_server.load("images/title/logo.png")),
            Node {
                width: Val::Px(400.0),
                height: Val::Auto,
                margin: UiRect::bottom(Val::Px(50.0)),
                ..default()
            },
        ));

        let items: [(TitleButtonAction, &str); 4] = [
            (TitleButtonAction::NewGame, "New Game"),
            (TitleButtonAction::LoadGame, "Load Game"),
            (TitleButtonAction::Settings, "Settings"),
            (TitleButtonAction::Gallery, "Gallery"),
        ];
        for (action, label) in items {
            parent.spawn((
                action,
                Button,
                Node {
                    width: Val::Px(BTN_W),
                    height: Val::Px(BTN_H),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(BTN_COLOR),
            )).with_child((
                Text::new(label),
                TextFont { font: game_font.0.clone(), font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.95)),
            ));
        }
    });
}

fn handle_title_buttons(
    mut query: Query<(&TitleButtonAction, &Interaction, &mut BackgroundColor), Changed<Interaction>>,
    mut screen_transition: ResMut<ScreenTransition>,
    mut next_state: ResMut<NextState<AppState>>,
    mut mode: ResMut<SaveLoadMode>,
) {
    for (action, interaction, mut bg) in query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(BTN_COLOR);
                match action {
                    TitleButtonAction::NewGame => {
                        screen_transition.pending_state = Some(AppState::Gameplay);
                    }
                    TitleButtonAction::LoadGame => {
                        mode.0 = false;
                        next_state.set(AppState::SaveLoad);
                    }
                    TitleButtonAction::Settings => {
                        next_state.set(AppState::Settings);
                    }
                    TitleButtonAction::Gallery => {
                        next_state.set(AppState::Gallery);
                    }
                }
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(BTN_HOVER);
            }
            Interaction::None => {
                *bg = BackgroundColor(BTN_COLOR);
            }
        }
    }
}

fn cleanup_title(mut commands: Commands, query: Query<Entity, With<TitleRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
