use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("game_data.rs");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rerun-if-changed=assets/scripts/");
    println!("cargo:rerun-if-changed=assets/image/ev/");
    println!("cargo:rerun-if-changed=assets/audio/bgm/");
    println!("cargo:rerun-if-changed=assets/audio/se/");

    let mut f = fs::File::create(&dest_path).unwrap();

    // ---- scripts ----
    writeln!(f, "pub fn all_scripts() -> Vec<(&'static str, &'static str)> {{").unwrap();
    writeln!(f, "    vec![").unwrap();
    let scripts_dir = Path::new(&manifest_dir).join("assets/scripts");
    if let Ok(entries) = fs::read_dir(&scripts_dir) {
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in &entries {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "ron") {
                let fname = path.file_name().unwrap().to_str().unwrap().to_string();
                if fname.ends_with(".bscript.ron") {
                    let name = fname.strip_suffix(".bscript.ron").unwrap();
                    writeln!(
                        f,
                        "        (\"{}\", include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/scripts/{}\"))),",
                        name, fname
                    )
                    .unwrap();
                }
            }
        }
    }
    writeln!(f, "    ]").unwrap();
    writeln!(f, "}}").unwrap();

    // ---- obj_index.ron ----
    writeln!(f, "pub fn obj_index_content() -> &'static str {{").unwrap();
    writeln!(
        f,
        "    include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/scripts/obj_index.ron\"))"
    )
    .unwrap();
    writeln!(f, "}}").unwrap();

    // ---- CG files: scan all ev files recursively ----
    let ev_dir = Path::new(&manifest_dir).join("assets/image/ev");
    let mut top_ev_files: Vec<String> = Vec::new();
    let mut ext_map: HashMap<String, &'static str> = HashMap::new();

    if let Ok(entries) = fs::read_dir(&ev_dir) {
        let file_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        for entry in &file_entries {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if is_image_file(name) {
                        top_ev_files.push(name.to_string());
                        ext_map.insert(strip_ext(name), ext_of(name));
                    }
                }
            }
        }
        for entry in &file_entries {
            let path = entry.path();
            if path.is_dir() {
                scan_ev_subdir(&path, &mut ext_map);
            }
        }
    }
    top_ev_files.sort();

    // Generate all_cg_files (top-level only)
    writeln!(f, "pub fn all_cg_files() -> Vec<&'static str> {{").unwrap();
    writeln!(f, "    vec![").unwrap();
    for name in &top_ev_files {
        writeln!(f, "        \"{}\",", name).unwrap();
    }
    writeln!(f, "    ]").unwrap();
    writeln!(f, "}}").unwrap();

    // Generate ev_file_ext (all files including subdirs)
    // Collect non-png entries for the match
    let non_png: Vec<(&str, &str)> = ext_map.iter()
        .filter(|(_, ext)| **ext != "png")
        .map(|(base, ext)| (base.as_str(), *ext))
        .collect();

    writeln!(f, "pub fn ev_file_ext(file: &str) -> &'static str {{").unwrap();
    if non_png.is_empty() {
        writeln!(f, "    \".png\"").unwrap();
    } else {
        writeln!(f, "    match file {{").unwrap();
        for (base, ext) in &non_png {
            writeln!(f, "        \"{}\" => \".{}\",", base, ext).unwrap();
        }
        writeln!(f, "        _ => \".png\",").unwrap();
        writeln!(f, "    }}").unwrap();
    }
    writeln!(f, "}}").unwrap();

    writeln!(f, "pub fn ev_file_path(file: &str) -> String {{").unwrap();
    writeln!(f, "    format!(\"image/ev/{{}}{{}}\", file, ev_file_ext(file))").unwrap();
    writeln!(f, "}}").unwrap();

    // ---- BGM IDs: scan audio/bgm/ ----
    let bgm_dir = Path::new(&manifest_dir).join("assets/audio/bgm");
    let mut bgm_ids: Vec<String> = Vec::new();
    let mut split_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Ok(entries) = fs::read_dir(&bgm_dir) {
        let mut seen = std::collections::HashSet::new();
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(id) = name.strip_prefix("bgm_").and_then(|s| s.strip_suffix("_a.ogg")) {
                    if seen.insert(id.to_string()) {
                        bgm_ids.push(id.to_string());
                        split_ids.insert(id.to_string());
                    }
                }
            }
        }
    }
    bgm_ids.sort_by(|a, b| {
        let an: u32 = a.parse().unwrap_or(0);
        let bn: u32 = b.parse().unwrap_or(0);
        an.cmp(&bn)
    });

    writeln!(f, "pub fn all_bgm_ids() -> Vec<&'static str> {{").unwrap();
    writeln!(f, "    vec![").unwrap();
    for id in &bgm_ids {
        writeln!(f, "        \"{}\",", id).unwrap();
    }
    writeln!(f, "    ]").unwrap();
    writeln!(f, "}}").unwrap();

    writeln!(f, "pub fn bgm_has_split(id: &str) -> bool {{").unwrap();
    if split_ids.is_empty() {
        writeln!(f, "    false").unwrap();
    } else {
        writeln!(f, "    match id {{").unwrap();
        for id in &split_ids {
            writeln!(f, "        \"{}\" => true,", id).unwrap();
        }
        writeln!(f, "        _ => false,").unwrap();
        writeln!(f, "    }}").unwrap();
    }
    writeln!(f, "}}").unwrap();

    // ---- SE split detection: scan audio/se/ ----
    let se_dir = Path::new(&manifest_dir).join("assets/audio/se");
    let mut se_split_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Ok(entries) = fs::read_dir(&se_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
                if let Some(id) = name.strip_suffix("_a.ogg") {
                    se_split_ids.insert(id.to_string());
                }
            }
        }
    }

    writeln!(f, "pub fn se_has_split(file: &str) -> bool {{").unwrap();
    if se_split_ids.is_empty() {
        writeln!(f, "    false").unwrap();
    } else {
        writeln!(f, "    match file {{").unwrap();
        for id in &se_split_ids {
            writeln!(f, "        \"{}\" => true,", id).unwrap();
        }
        writeln!(f, "        _ => false,").unwrap();
        writeln!(f, "    }}").unwrap();
    }
    writeln!(f, "}}").unwrap();
}

fn is_image_file(name: &str) -> bool {
    name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg")
}

fn strip_ext(name: &str) -> String {
    let s = name.to_string();
    for ext in &[".png", ".jpg", ".jpeg"] {
        if let Some(stripped) = s.strip_suffix(ext) {
            return stripped.to_string();
        }
    }
    s
}

fn ext_of(_name: &str) -> &'static str {
    if _name.ends_with(".png") { return "png"; }
    if _name.ends_with(".jpg") { return "jpg"; }
    if _name.ends_with(".jpeg") { return "jpeg"; }
    ""
}

fn scan_ev_subdir(dir: &Path, ext_map: &mut HashMap<String, &'static str>) {
    if let Ok(entries) = fs::read_dir(dir) {
        let collected: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        for entry in &collected {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if is_image_file(name) {
                        let parent = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        ext_map.insert(format!("{}/{}", parent, strip_ext(name)), ext_of(name));
                    }
                }
            }
        }
    }
}
