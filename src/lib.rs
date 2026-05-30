pub mod script;
pub mod state;
pub mod components;
pub mod resources;
pub mod events;
pub mod plugins;
pub mod rendering_messages;
pub mod audio_messages;
pub mod choice_messages;

pub use script::Transition;

use bevy::prelude::*;
use bevy::camera::ScalingMode;
use bevy::window::{PresentMode, WindowResolution};

use state::AppState;
use resources::{CompletedRoute, GameFont, GameRestrictions, ObjFileIndex, RouteConfig, SelectedRoute};
use script::ScriptEngine;
use plugins::audio::AudioPlugin;
use plugins::title::TitlePlugin;
use plugins::inputs::InputPlugin;
use plugins::menu::MenuPlugin;
use plugins::script_loader::ScriptLoaderPlugin;
use plugins::script_runner::ScriptRunnerPlugin;
use plugins::affection::AffectionPlugin;
use plugins::save_load::SaveLoadPlugin;
use plugins::dialogue::DialoguePlugin;
use plugins::settings::SettingsPlugin;
use plugins::splash::SplashPlugin;
use plugins::gallery::GalleryPlugin;
use plugins::rendering::RenderingPlugin;
use plugins::choice::ChoicePlugin;
use plugins::screen_transition::ScreenTransitionPlugin;
use plugins::backlog::BacklogPlugin;
use plugins::event_system::EventSystemPlugin;
use plugins::route_end::RouteEndPlugin;
use plugins::routing::RoutePlugin;
use bevy_scrollbar::ScrollbarPlugin;

pub fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
            title: "Aiyoku no Eustia".to_string(),
            present_mode: PresentMode::Fifo,
            ..default()
        }),
        ..default()
    }))
    .init_state::<AppState>()
    .init_resource::<ScriptEngine>()
    .add_systems(PostStartup, setup_display_scaling)
    .add_plugins(SplashPlugin)
    .add_plugins(TitlePlugin)
    .add_plugins(InputPlugin)
    .add_plugins(MenuPlugin)
    .add_plugins(ScriptLoaderPlugin)
    .add_plugins(ScriptRunnerPlugin)
    .add_plugins(AffectionPlugin)
    .add_plugins(SaveLoadPlugin)
    .add_plugins(DialoguePlugin)
    .add_plugins(SettingsPlugin)
    .add_plugins(GalleryPlugin)
    .add_plugins(AudioPlugin)
    .add_plugins(RenderingPlugin)
    .add_plugins(ChoicePlugin)
    .add_plugins(ScreenTransitionPlugin)
    .add_plugins(BacklogPlugin)
    .add_plugins(EventSystemPlugin)
    .add_plugins(RoutePlugin)
    .add_plugins(RouteEndPlugin)
    .add_plugins(ScrollbarPlugin)
    .insert_resource(
        ron::from_str::<RouteConfig>(include_str!("../assets/routes.ron"))
            .expect("Failed to parse routes.ron")
    )
    .init_resource::<SelectedRoute>()
    .init_resource::<CompletedRoute>()
    .init_resource::<GameRestrictions>()
    .init_resource::<ObjFileIndex>()
    .add_systems(Startup, (startup, load_obj_index));
    app
}

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection::default_2d()),
    ));
    commands.insert_resource(GameFont(asset_server.load("fonts/sourcehansans-medium.otf")));
    next_state.set(AppState::Splash);
}

fn setup_display_scaling(
    mut commands: Commands,
    windows: Query<&Window>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
) {
    if let Ok(window) = windows.single() {
        let scale = (window.width() / 640.0).max(1.0);
        commands.insert_resource(UiScale(scale));
    }
    if let Ok(mut proj) = camera_query.single_mut() {
        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scaling_mode = ScalingMode::AutoMin {
                min_width: 1280.0,
                min_height: 720.0,
            };
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/game_data.rs"));

fn load_obj_index(mut index: ResMut<ObjFileIndex>) {
    let content = obj_index_content();
    match ron::from_str::<std::collections::HashMap<String, String>>(content) {
        Ok(map) => {
            index.0 = map;
            info!("Loaded obj_index.ron with {} entries", index.0.len());
        }
        Err(e) => warn!("Failed to parse obj_index.ron: {}", e),
    }
}

#[cfg(feature = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    let _ = bevy_android::ANDROID_APP.set(app);
    build_app().run();
}
