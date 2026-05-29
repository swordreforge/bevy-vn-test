mod asb;
mod iet;
mod lua_config;
mod mapper;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
#[command(name = "artemis-export", about = "Convert Artemis .asb scripts to Bevy VN .bscript.ron")]
struct Args {
    #[arg(long)]
    input: String,
    #[arg(long)]
    output: String,
    #[arg(long, default_value_t = false)]
    verbose: bool,
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let input_root = PathBuf::from(&args.input);
    let output_dir = PathBuf::from(&args.output);

    if !input_root.exists() {
        anyhow::bail!("Input directory not found: {}", args.input);
    }

    if !args.dry_run {
        std::fs::create_dir_all(&output_dir).context("Failed to create output directory")?;
    }

    eprintln!("[1/3] Extracting Lua configs...");
    let config =
        lua_config::extract_config(&input_root).context("Failed to extract Lua configs")?;

    let asb_files = discover_asb_files(&input_root);
    if asb_files.is_empty() {
        anyhow::bail!("No .asb files found under {:?}", input_root);
    }
    eprintln!("[2/3] Found {} .asb files", asb_files.len());

    let mut converted = 0;
    let mut skipped = 0;

    for asb_path in &asb_files {
        let script = match asb::parse_asb(asb_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  [warn] Failed to parse {:?}: {}", asb_path, e);
                skipped += 1;
                continue;
            }
        };

        let output_name = asb_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let output_path = output_dir.join(format!("{}.bscript.ron", output_name));

        let ron_cmds = mapper::map_script(&script, &config, args.verbose);

        if !args.dry_run {
            let ron_str = ron::ser::to_string_pretty(
                &ron_cmds,
                ron::ser::PrettyConfig::default(),
            )
            .context("RON serialization failed")?;
            std::fs::write(&output_path, ron_str)
                .with_context(|| format!("Failed to write {:?}", output_path))?;
        }

        if args.verbose {
            eprintln!("  -> {} ({} commands)", output_name, ron_cmds.len());
        }
        converted += 1;
    }

    eprintln!("[3/6] Building obj file index...");
    write_obj_index(&input_root, &output_dir)
        .context("Failed to write obj_index.ron")?;

    eprintln!(
        "[4/6] ASB done: {} converted, {} skipped",
        converted, skipped
    );

    eprintln!("[5/6] Processing .iet files...");
    let iet_files = discover_iet_files(&input_root);
    for iet_path in &iet_files {
        let name = iet_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // main.iet -> output root; other .iet files -> scripts/ subdirectory
        let output_path = if name == "main" {
            output_dir.join("main.bscript.ron")
        } else {
            output_dir.join(format!("scripts/{}.bscript.ron", name))
        };

        match iet::parse_iet(iet_path, args.verbose) {
            Ok(script) => {
                if !args.dry_run {
                    let ron_str = ron::ser::to_string_pretty(
                        &script,
                        ron::ser::PrettyConfig::default(),
                    )
                    .context("RON serialization failed")?;
                    std::fs::write(&output_path, ron_str)
                        .with_context(|| format!("Failed to write {:?}", output_path))?;
                }
                if args.verbose {
                    eprintln!("  -> {} ({} commands)", name, script.len());
                }
            }
            Err(e) => {
                eprintln!("  [warn] Failed to parse .iet {:?}: {}", iet_path, e);
            }
        }
    }
    eprintln!("  processed {} .iet files", iet_files.len());

    eprintln!("[6/6] All done.");
    Ok(())
}

fn build_obj_index(input_root: &Path) -> HashMap<String, String> {
    let mut entries = Vec::new();
    let obj_dir = input_root.join("image/obj");
    if !obj_dir.exists() {
        eprintln!("  [warn] obj directory not found: {:?}", obj_dir);
        return HashMap::new();
    }
    scan_obj_dir(&obj_dir, input_root, &mut entries);
    entries.sort();
    let mut map: HashMap<String, String> = HashMap::new();
    for (stem, rel) in entries {
        map.entry(stem).or_insert(rel);
    }
    map
}

fn scan_obj_dir(dir: &Path, input_root: &Path, entries: &mut Vec<(String, String)>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_obj_dir(&path, input_root, entries);
            } else if path.is_file() {
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
                let rel_path = path.strip_prefix(input_root).unwrap_or(&path);
                let rel = rel_path.to_string_lossy().replace('\\', "/");
                entries.push((stem.to_string(), rel));
            }
        }
    }
}

fn write_obj_index(input_root: &Path, output_dir: &Path) -> Result<()> {
    let map = build_obj_index(input_root);
    if map.is_empty() {
        eprintln!("  [skip] obj_index.ron (0 entries)");
        return Ok(());
    }
    let ron_str = ron::ser::to_string_pretty(&map, ron::ser::PrettyConfig::default())
        .context("RON serialization of obj index failed")?;
    let output_path = output_dir.join("scripts/obj_index.ron");
    std::fs::write(&output_path, &ron_str)
        .with_context(|| format!("Failed to write obj_index.ron: {:?}", output_path))?;
    eprintln!("  -> obj_index.ron with {} entries", map.len());
    Ok(())
}

fn discover_asb_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let scenario_dir = root.join("scenario");
    if scenario_dir.exists() {
        collect_asb_files(&scenario_dir, &mut files);
    }
    files.sort();
    files
}

fn collect_asb_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_asb_files(&path, files);
            } else if path.extension().map(|e| e == "asb").unwrap_or(false) {
                files.push(path);
            }
        }
    }
}

fn discover_iet_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let scenario_dir = root.join("scenario");
    if scenario_dir.exists() {
        collect_iet_files(&scenario_dir, &mut files);
    }
    files.sort();
    files
}

fn collect_iet_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_iet_files(&path, files);
            } else if path.extension().map(|e| e == "iet").unwrap_or(false) {
                files.push(path);
            }
        }
    }
}
