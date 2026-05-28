use bevy::prelude::*;
use crate::state::AppState;
use crate::resources::ScreenTransition;

pub struct SplashPlugin;

#[derive(Component)]
struct SplashRoot;

#[derive(Resource)]
struct SplashState {
    phase: SplashPhase,
    timer: Timer,
}

enum SplashPhase {
    Logo0,
    Logo1,
}

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Splash), setup_splash)
            .add_systems(Update, splash_update.run_if(in_state(AppState::Splash)))
            .add_systems(OnExit(AppState::Splash), cleanup_splash);
    }
}

fn setup_splash(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SplashState {
        phase: SplashPhase::Logo0,
        timer: Timer::from_seconds(3.0, TimerMode::Once),
    });

    commands.spawn((
        SplashRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode::new(asset_server.load("image/logo00.png")),
    ));
}

fn splash_update(
    time: Res<Time>,
    mut splash_state: ResMut<SplashState>,
    mut screen_transition: ResMut<ScreenTransition>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    splash_root_query: Query<Entity, With<SplashRoot>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    splash_state.timer.tick(time.delta());
    let advance = splash_state.timer.just_finished() || mouse.just_pressed(MouseButton::Left);

    if !advance {
        return;
    }

    match splash_state.phase {
        SplashPhase::Logo0 => {
            for entity in &splash_root_query {
                commands.entity(entity).despawn();
            }
            commands.spawn((
                SplashRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ImageNode::new(asset_server.load("image/logo01.png")),
            ));
            splash_state.phase = SplashPhase::Logo1;
            splash_state.timer.reset();
        }
        SplashPhase::Logo1 => {
            screen_transition.pending_state = Some(AppState::Title);
        }
    }
}

fn cleanup_splash(mut commands: Commands, query: Query<Entity, With<SplashRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
