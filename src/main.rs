use bevy::prelude::*;
use bevy::window::Window;

mod state;
mod plugins;
mod components;
mod resources;
mod events;
mod rendering_messages;
mod audio_messages;
mod choice_messages;
mod script;

use state::AppState;
use resources::GameFont;
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
use bevy_scrollbar::ScrollbarPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (1280, 720).into(),
                title: "Aiyoku no Eustia".to_string(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .init_resource::<ScriptEngine>()
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
        .add_plugins(ScrollbarPlugin)
        .add_systems(Startup, startup)
        .run();
}

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    commands.spawn(Camera2d);
    commands.insert_resource(GameFont(asset_server.load("fonts/sourcehansans-medium.otf")));
    next_state.set(AppState::Splash);
}
