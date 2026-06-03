use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;
use walkdir::WalkDir;

const PAK_MAGIC: &[u8; 4] = b"BPAK";
const PAK_VERSION: u32 = 1;

#[derive(Parser)]
#[command(name = "asset-packer", about = "Pack assets/ into compressed .pak bundles")]
struct Args {
    #[arg(long, default_value = "assets")]
    input: String,

    #[arg(long, default_value = "assets_pak")]
    output: String,

    #[arg(long, default_value = "pack_config.ron")]
    config: String,

    #[arg(long, default_value_t = 3)]
    compression_level: i32,
}

#[derive(Debug, Deserialize)]
struct PackConfig {
    bundles: Vec<BundleDef>,
}

#[derive(Debug, Deserialize)]
struct BundleDef {
    name: String,
    includes: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let input_dir = PathBuf::from(&args.input);

    if !input_dir.is_dir() {
        eprintln!("Error: input directory not found: {}", args.input);
        std::process::exit(1);
    }

    let config_str = match std::fs::read_to_string(&args.config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: failed to read config '{}': {}", args.config, e);
            std::process::exit(1);
        }
    };

    let config: PackConfig = match ron::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: failed to parse config '{}': {}", args.config, e);
            std::process::exit(1);
        }
    };

    if config.bundles.is_empty() {
        eprintln!("Error: no bundles defined in config");
        std::process::exit(1);
    }

    let output_dir = PathBuf::from(&args.output);
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Scan all files
    eprintln!("Scanning {} ...", args.input);
    let mut all_files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total_uncompressed: u64 = 0;

    for entry in WalkDir::new(&input_dir).follow_links(true) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("  [warn] {}", e);
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let rel_path = entry
            .path()
            .strip_prefix(&input_dir)
            .expect("path should be under input dir");

        let rel = rel_path.to_str().expect("non-UTF-8 path").replace('\\', "/");
        let data = match std::fs::read(entry.path()) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  [warn] Failed to read {}: {}", rel, e);
                continue;
            }
        };

        total_uncompressed += data.len() as u64;
        all_files.push((rel, data));
    }

    eprintln!(
        "Found {} files, {:.1}MB uncompressed",
        all_files.len(),
        total_uncompressed as f64 / 1_048_576.0
    );

    // Assign each file to the first matching bundle
    let mut bundle_files: Vec<(String, Vec<(String, Vec<u8>)>)> = config
        .bundles
        .iter()
        .map(|b| (b.name.clone(), Vec::new()))
        .collect();

    let mut unmatched: Vec<String> = Vec::new();

    'file_loop: for (rel, data) in &all_files {
        for (bi, bundle) in config.bundles.iter().enumerate() {
            if bundle.includes.iter().any(|pat| rel.starts_with(pat)) {
                bundle_files[bi].1.push((rel.clone(), data.clone()));
                continue 'file_loop;
            }
        }
        unmatched.push(rel.clone());
    }

    if !unmatched.is_empty() {
        eprintln!(
            "\n[warning] {} files did not match any bundle:",
            unmatched.len()
        );
        for f in unmatched.iter().take(20) {
            eprintln!("  {}", f);
        }
        if unmatched.len() > 20 {
            eprintln!("  ... and {} more", unmatched.len() - 20);
        }
    }

    // Compress and write each bundle
    let mut _total_compressed: u64 = 0;

    for (bundle_name, files) in &bundle_files {
        if files.is_empty() {
            eprintln!("  [skip] {} — no files", bundle_name);
            continue;
        }

        eprintln!(
            "Packing {} ({} files, {:.1}MB)...",
            bundle_name,
            files.len(),
            files.iter().map(|(_, d)| d.len() as u64).sum::<u64>() as f64 / 1_048_576.0
        );

        let mut data_section: Vec<u8> = Vec::new();
        let mut index: Vec<IndexEntry> = Vec::with_capacity(files.len());

        for (path, raw) in files {
            let compressed = zstd::encode_all(std::io::Cursor::new(raw), args.compression_level)
                .expect("zstd compression failed");

            let offset = data_section.len() as u64;
            let compressed_size = compressed.len() as u64;
            let uncompressed_size = raw.len() as u64;

            data_section.extend_from_slice(&compressed);
            _total_compressed += compressed_size;

            index.push(IndexEntry {
                path: path.clone(),
                offset,
                compressed_size,
                uncompressed_size,
            });
        }

        let mut pak = Vec::with_capacity(data_section.len() + index.len() * 60 + 20);

        // Data section
        pak.extend_from_slice(&data_section);

        // Index section
        let index_offset = pak.len() as u64;
        for entry in &index {
            let path_bytes = entry.path.as_bytes();
            pak.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
            pak.extend_from_slice(path_bytes);
            pak.extend_from_slice(&entry.offset.to_le_bytes());
            pak.extend_from_slice(&entry.compressed_size.to_le_bytes());
            pak.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
        }

        // Footer
        pak.extend_from_slice(PAK_MAGIC);
        pak.extend_from_slice(&PAK_VERSION.to_le_bytes());
        pak.extend_from_slice(&index_offset.to_le_bytes());
        pak.extend_from_slice(&(index.len() as u32).to_le_bytes());

        let pak_path = output_dir.join(format!("{}.pak", bundle_name));
        std::fs::write(&pak_path, &pak).expect("Failed to write PAK file");

        let bundle_uncompressed: u64 = files.iter().map(|(_, d)| d.len() as u64).sum();
        eprintln!(
            "  wrote {} ({:.1}MB -> {:.1}MB)",
            pak_path.display(),
            bundle_uncompressed as f64 / 1_048_576.0,
            pak.len() as f64 / 1_048_576.0,
        );
    }

    let total_pak_size: u64 = std::fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "pak").unwrap_or(false))
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();

    eprintln!(
        "\nDone. {} total -> {:.1}MB in {} PAK bundles",
        if unmatched.is_empty() {
            "All files packed".to_string()
        } else {
            format!("{}/{} files packed", all_files.len() - unmatched.len(), all_files.len())
        },
        total_pak_size as f64 / 1_048_576.0,
        bundle_files.iter().filter(|(_, f)| !f.is_empty()).count(),
    );
}

struct IndexEntry {
    path: String,
    offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
}
