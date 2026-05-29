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

    // ---- CG files list ----
    writeln!(f, "pub fn all_cg_files() -> Vec<&'static str> {{").unwrap();
    writeln!(f, "    vec![").unwrap();
    let ev_dir = Path::new(&manifest_dir).join("assets/image/ev");
    if let Ok(entries) = fs::read_dir(&ev_dir) {
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in &entries {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg")
                    {
                        writeln!(f, "        \"{}\",", name).unwrap();
                    }
                }
            }
        }
    }
    writeln!(f, "    ]").unwrap();
    writeln!(f, "}}").unwrap();
}
