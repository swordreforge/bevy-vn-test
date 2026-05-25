use bevy::prelude::*;
use crate::script::{ScriptCmd, ScriptEngine};
use std::fs;
use std::path::Path;

pub struct ScriptLoaderPlugin;

impl Plugin for ScriptLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_all_scripts);
    }
}

fn load_all_scripts(mut engine: ResMut<ScriptEngine>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/scripts");
    let mut entries: Vec<_> = match fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            error!("Failed to read scripts dir {:?}: {}", dir, e);
            return;
        }
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let fname = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        if !fname.ends_with(".bscript.ron") {
            continue;
        }
        let name = fname.trim_end_matches(".bscript.ron");
        match fs::read_to_string(&path) {
            Ok(content) => match ron::from_str::<Vec<ScriptCmd>>(&content) {
                Ok(script) => {
                    engine.scripts.insert(name.to_string(), script);
                    info!("Loaded script: {}", name);
                }
                Err(e) => {
                    error!("Failed to parse {}: {}", path.display(), e);
                }
            },
            Err(e) => {
                error!("Failed to read {}: {}", path.display(), e);
            }
        }
    }

    // Start from master routing script
    if engine.scripts.contains_key("main") {
        engine.current_script = "main".to_string();
    } else if engine.scripts.contains_key("aiy00010") {
        engine.current_script = "aiy00010".to_string();
    }
    engine.current_line = 0;
    engine.call_stack.clear();
}
