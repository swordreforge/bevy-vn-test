use bevy::prelude::*;
use crate::script::{ScriptCmd, ScriptEngine};
use std::fs;

pub struct ScriptLoaderPlugin;

impl Plugin for ScriptLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_test_script);
    }
}

fn load_test_script(mut engine: ResMut<ScriptEngine>) {
    let path = "assets/scripts/test.bscript.ron";
    match fs::read_to_string(path) {
        Ok(content) => {
            match ron::from_str::<Vec<ScriptCmd>>(&content) {
                Ok(script) => {
                    engine.load("test", script);
                    info!("Loaded script: {}", path);
                }
                Err(e) => {
                    error!("Failed to parse {}: {}", path, e);
                }
            }
        }
        Err(e) => {
            error!("Failed to read {}: {}", path, e);
        }
    }
}
