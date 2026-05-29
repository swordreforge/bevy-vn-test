use bevy::prelude::*;
use crate::script::{ScriptCmd, ScriptEngine};

include!(concat!(env!("OUT_DIR"), "/game_data.rs"));

pub struct ScriptLoaderPlugin;

impl Plugin for ScriptLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_all_scripts);
    }
}

fn load_all_scripts(mut engine: ResMut<ScriptEngine>) {
    for (name, content) in all_scripts() {
        match ron::from_str::<Vec<ScriptCmd>>(content) {
            Ok(script) => {
                engine.scripts.insert(name.to_string(), script);
                info!("Loaded script: {}", name);
            }
            Err(e) => {
                error!("Failed to parse script {}: {}", name, e);
            }
        }
    }

    if engine.scripts.contains_key("main") {
        engine.current_script = "main".to_string();
    } else if engine.scripts.contains_key("aiy00010") {
        engine.current_script = "aiy00010".to_string();
    }
    engine.current_line = 0;
    engine.call_stack.clear();
}
