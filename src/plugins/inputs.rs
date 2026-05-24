use bevy::prelude::*;
use crate::state::AppState;

pub struct InputPlugin;

#[derive(Message)]
pub struct AdvanceEvent;

#[derive(Message)]
pub struct MenuToggleEvent;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<AdvanceEvent>()
            .add_message::<MenuToggleEvent>()
            .add_systems(Update, handle_global_input);
    }
}

fn handle_global_input(
    state: Res<State<AppState>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut advance_ev: MessageWriter<AdvanceEvent>,
    mut menu_ev: MessageWriter<MenuToggleEvent>,
) {
    if *state != AppState::Title {
        if mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed() {
            advance_ev.write(AdvanceEvent);
        }
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        menu_ev.write(MenuToggleEvent);
    }
}
