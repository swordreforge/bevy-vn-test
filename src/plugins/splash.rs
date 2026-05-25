use bevy::prelude::*;
use crate::state::AppState;
use crate::resources::ScreenTransition;

pub struct SplashPlugin;

#[derive(Component)]
struct SplashRoot;

#[derive(Resource)]
struct SplashTimer(Timer);

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Splash), setup_splash)
            .add_systems(Update, splash_tick.run_if(in_state(AppState::Splash)))
            .add_systems(OnExit(AppState::Splash), cleanup_splash);
    }
}

fn setup_splash(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.init_resource::<SplashTimer>();

    commands.spawn((
        SplashRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::BLACK),
    )).with_child((
        ImageNode::new(asset_server.load("images/splash.png")),
        Node {
            width: Val::Px(300.0),
            height: Val::Auto,
            ..default()
        },
    ));
}

fn splash_tick(
    time: Res<Time>,
    mut timer: ResMut<SplashTimer>,
    mut screen_transition: ResMut<ScreenTransition>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        screen_transition.pending_state = Some(AppState::Title);
    }
}

fn cleanup_splash(mut commands: Commands, query: Query<Entity, With<SplashRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

impl Default for SplashTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(3.0, TimerMode::Once))
    }
}
