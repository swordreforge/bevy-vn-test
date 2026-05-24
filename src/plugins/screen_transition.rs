use bevy::prelude::*;
use crate::components::TransitionOverlay;
use crate::resources::ScreenTransition;
use crate::state::AppState;

pub struct ScreenTransitionPlugin;

impl Plugin for ScreenTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenTransition>()
            .add_systems(Update, handle_screen_transition);
    }
}

fn handle_screen_transition(
    time: Res<Time>,
    mut transition: ResMut<ScreenTransition>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mut overlay_query: Query<&mut BackgroundColor, With<TransitionOverlay>>,
) {
    // Process active fades
    let finished = match &mut transition.phase {
        TransitionPhase::FadingToBlack { timer } => {
            timer.tick(time.delta());
            let alpha = timer.fraction();
            if let Some(entity) = transition.overlay {
                if let Ok(mut bg) = overlay_query.get_mut(entity) {
                    bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
                }
            }
            timer.just_finished()
        }
        TransitionPhase::FadingFromBlack { timer } => {
            timer.tick(time.delta());
            let alpha = 1.0 - timer.fraction();
            if let Some(entity) = transition.overlay {
                if let Ok(mut bg) = overlay_query.get_mut(entity) {
                    bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
                }
            }
            timer.just_finished()
        }
        TransitionPhase::Idle => false,
    };

    if finished {
        match transition.phase {
            TransitionPhase::FadingToBlack { .. } => {
                let target = transition.pending_state.take().unwrap_or(AppState::Title);
                next_state.set(target);
                transition.phase = TransitionPhase::FadingFromBlack {
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                };
            }
            TransitionPhase::FadingFromBlack { .. } => {
                if let Some(entity) = transition.overlay.take() {
                    commands.entity(entity).despawn();
                }
                transition.phase = TransitionPhase::Idle;
            }
            _ => {}
        }
        return;
    }

    // Start new transition if idle and pending
    if matches!(transition.phase, TransitionPhase::Idle) && transition.pending_state.is_some() {
        let entity = commands.spawn((
            TransitionOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ZIndex(10),
        )).id();
        transition.overlay = Some(entity);
        transition.phase = TransitionPhase::FadingToBlack {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
        };
    }
}
