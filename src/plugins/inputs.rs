use bevy::prelude::*;
use crate::state::AppState;

pub struct InputPlugin;

#[derive(Message)]
pub struct AdvanceEvent;

#[derive(Message)]
pub struct MenuToggleEvent;

const LONG_PRESS_DURATION: f32 = 3.0;
const LONG_PRESS_MAX_MOVE: f32 = 30.0;

#[derive(Resource)]
pub struct LongPressState {
    touch_id: Option<u64>,
    start_pos: Vec2,
    timer: Timer,
}

impl Default for LongPressState {
    fn default() -> Self {
        Self {
            touch_id: None,
            start_pos: Vec2::ZERO,
            timer: Timer::from_seconds(LONG_PRESS_DURATION, TimerMode::Once),
        }
    }
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<AdvanceEvent>()
            .add_message::<MenuToggleEvent>()
            .init_resource::<LongPressState>()
            .add_systems(Update, (handle_global_input, handle_long_press_menu));
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

fn handle_long_press_menu(
    time: Res<Time>,
    touches: Res<Touches>,
    state: Res<State<AppState>>,
    mut long_press: ResMut<LongPressState>,
    mut menu_ev: MessageWriter<MenuToggleEvent>,
) {
    if *state != AppState::Gameplay {
        long_press.touch_id = None;
        long_press.timer.reset();
        return;
    }

    if let Some(tid) = long_press.touch_id {
        if touches.just_released(tid) || touches.just_canceled(tid) {
            long_press.touch_id = None;
            long_press.timer.reset();
        }
    }

    if long_press.touch_id.is_none() && touches.any_just_pressed() {
        for touch in touches.iter() {
            if touches.just_pressed(touch.id()) {
                long_press.touch_id = Some(touch.id());
                long_press.start_pos = touch.position();
                long_press.timer.reset();
                break;
            }
        }
    }

    if let Some(tid) = long_press.touch_id {
        if let Some(pos) = touches.get_pressed(tid) {
            let dist = pos.position().distance(long_press.start_pos);
            if dist > LONG_PRESS_MAX_MOVE {
                long_press.touch_id = None;
                long_press.timer.reset();
                return;
            }
        }

        long_press.timer.tick(time.delta());
        if long_press.timer.just_finished() {
            menu_ev.write(MenuToggleEvent);
            long_press.touch_id = None;
            long_press.timer.reset();
        }
    }
}
