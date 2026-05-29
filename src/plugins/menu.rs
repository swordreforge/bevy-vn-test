use bevy::prelude::*;
use crate::resources::{GameFont, SaveLoadMode, ScreenTransition};
use crate::state::AppState;
use crate::plugins::inputs::MenuToggleEvent;

pub struct MenuPlugin;

#[derive(Component)]
pub struct MenuUiRoot;

#[derive(Component)]
pub enum MenuButtonAction {
    Save,
    Load,
    Settings,
    Gallery,
    Backlog,
    Title,
}

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SaveLoadMode>()
            .add_systems(OnEnter(AppState::Menu), setup_menu_ui)
            .add_systems(OnExit(AppState::Menu), cleanup_menu_ui)
            .add_systems(Update, (
                handle_menu_button_interaction,
                handle_menu_close_outside,
                handle_menu_toggle,
            ));
    }
}

fn setup_menu_ui(mut commands: Commands, game_font: Res<GameFont>) {
    commands.spawn((
        MenuUiRoot,
        Button,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        ZIndex(5),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("MENU"),
            TextFont { font: game_font.0.clone(), font_size: 36.0, ..default() },
            TextColor(Color::WHITE),
            Node { ..default() },
        ));
        let items: [(MenuButtonAction, &str); 6] = [
            (MenuButtonAction::Save, "Save"),
            (MenuButtonAction::Load, "Load"),
            (MenuButtonAction::Backlog, "Backlog"),
            (MenuButtonAction::Settings, "Settings"),
            (MenuButtonAction::Gallery, "Gallery"),
            (MenuButtonAction::Title, "Back to Title"),
        ];
        for (action, label) in items {
            parent.spawn((
                action,
                Button,
                Text::new(label),
                TextFont { font: game_font.0.clone(), font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 1.0)),
                Node { ..default() },
            ));
        }
    });
}

fn cleanup_menu_ui(mut commands: Commands, query: Query<Entity, With<MenuUiRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn handle_menu_button_interaction(
    query: Query<(&MenuButtonAction, &Interaction), Changed<Interaction>>,
    mut mode: ResMut<SaveLoadMode>,
    mut next_state: ResMut<NextState<AppState>>,
    mut screen_transition: ResMut<ScreenTransition>,
) {
    for (action, interaction) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            MenuButtonAction::Save => { mode.0 = true; next_state.set(AppState::SaveLoad); }
            MenuButtonAction::Load => { mode.0 = false; next_state.set(AppState::SaveLoad); }
            MenuButtonAction::Settings => next_state.set(AppState::Settings),
            MenuButtonAction::Gallery => next_state.set(AppState::Gallery),
            MenuButtonAction::Backlog => next_state.set(AppState::Backlog),
            MenuButtonAction::Title => screen_transition.pending_state = Some(AppState::Title),
        }
    }
}

fn handle_menu_close_outside(
    root_query: Query<&Interaction, (Changed<Interaction>, With<MenuUiRoot>)>,
    child_query: Query<&Interaction, (Changed<Interaction>, With<MenuButtonAction>)>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if *state != AppState::Menu {
        return;
    }
    let root_pressed = root_query.iter().any(|i| *i == Interaction::Pressed);
    if !root_pressed {
        return;
    }
    let child_pressed = child_query.iter().any(|i| *i == Interaction::Pressed);
    if child_pressed {
        return;
    }
    next_state.set(AppState::Gameplay);
}

fn handle_menu_toggle(
    mut ev: MessageReader<MenuToggleEvent>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for _ in ev.read() {
        match state.get() {
            AppState::Gameplay => next_state.set(AppState::Menu),
            AppState::Menu => next_state.set(AppState::Gameplay),
            _ => {}
        }
    }
}
