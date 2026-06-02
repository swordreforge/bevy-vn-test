use std::collections::{BTreeSet, HashMap};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use bevy::asset::io::{
    AssetReader, AssetReaderError, ErasedAssetReader, PathStream, Reader, StackFuture,
    STACK_FUTURE_SIZE,
};
use bevy::prelude::*;
use futures_lite::io::AsyncRead;
use memmap2::Mmap;
use zstd::stream::read::Decoder as ZstdDecoder;

const PAK_MAGIC: &[u8; 4] = b"BPAK";
const PAK_VERSION: u32 = 1;

#[derive(Debug, Clone)]
struct PakEntry {
    offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
}

struct LocatedEntry {
    source_index: usize,
    inner: PakEntry,
}

struct PakSource {
    mmap: Mmap,
}

pub struct PakAssetReader {
    sources: Vec<PakSource>,
    entries: HashMap<String, LocatedEntry>,
}

impl PakAssetReader {
    pub fn load_many(pak_paths: &[impl AsRef<Path>]) -> Result<Self, String> {
        let mut sources = Vec::new();
        let mut entries = HashMap::new();

        for (source_idx, path_ref) in pak_paths.iter().enumerate() {
            let path = path_ref.as_ref();
            let (source, source_entries) = load_single_pak(path)?;
            info!(
                "  PAK[{}] {} — {} entries",
                source_idx,
                path.display(),
                source_entries.len()
            );

            for (path_str, entry) in source_entries {
                entries.entry(path_str).or_insert(LocatedEntry {
                    source_index: source_idx,
                    inner: entry,
                });
            }

            sources.push(source);
        }

        info!(
            "Loaded {} PAK bundles, {} total entries",
            sources.len(),
            entries.len()
        );
        Ok(Self { sources, entries })
    }

    fn get_entry(&self, path: &Path) -> Option<&LocatedEntry> {
        let key = path.to_str().unwrap_or("").trim_start_matches('/');
        self.entries.get(key)
    }

    fn decompress(&self, entry: &LocatedEntry) -> Result<Vec<u8>, AssetReaderError> {
        let source = &self.sources[entry.source_index];
        let start = entry.inner.offset as usize;
        let end = start + entry.inner.compressed_size as usize;
        let compressed = &source.mmap[start..end];

        let mut decompressed = Vec::with_capacity(entry.inner.uncompressed_size as usize);
        let mut decoder = ZstdDecoder::new(std::io::Cursor::new(compressed))
            .map_err(|e| AssetReaderError::Io(std::io::Error::new(std::io::ErrorKind::Other, e).into()))?;
        std::io::Read::read_to_end(&mut decoder, &mut decompressed)
            .map_err(|e| AssetReaderError::Io(e.into()))?;
        Ok(decompressed)
    }
}

fn load_single_pak(path: &Path) -> Result<(PakSource, HashMap<String, PakEntry>), String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open PAK: {e}"))?;
    let file_len = file
        .metadata()
        .map_err(|e| format!("Failed to get PAK metadata: {e}"))?
        .len();

    if file_len < 20 {
        return Err("PAK file too small".into());
    }

    let mut footer = [0u8; 20];
    file.seek(SeekFrom::End(-20))
        .map_err(|e| format!("Failed to seek to PAK footer: {e}"))?;
    file.read_exact(&mut footer)
        .map_err(|e| format!("Failed to read PAK footer: {e}"))?;

    let magic = &footer[0..4];
    if magic != PAK_MAGIC {
        return Err(format!("Invalid PAK magic: {:?}", magic));
    }
    let version = u32::from_le_bytes(footer[4..8].try_into().unwrap());
    if version != PAK_VERSION {
        return Err(format!("Unsupported PAK version: {version}"));
    }
    let index_offset = u64::from_le_bytes(footer[8..16].try_into().unwrap());
    let entry_count = u32::from_le_bytes(footer[16..20].try_into().unwrap());

    file.seek(SeekFrom::Start(index_offset))
        .map_err(|e| format!("Failed to seek to PAK index: {e}"))?;

    let mut entries = HashMap::with_capacity(entry_count as usize);
    for _ in 0..entry_count {
        let mut path_len_buf = [0u8; 4];
        file.read_exact(&mut path_len_buf)
            .map_err(|_| "Failed to read entry path_len".to_string())?;
        let path_len = u32::from_le_bytes(path_len_buf) as usize;

        let mut path_bytes = vec![0u8; path_len];
        file.read_exact(&mut path_bytes)
            .map_err(|_| "Failed to read entry path".to_string())?;
        let path = String::from_utf8_lossy(&path_bytes).to_string();

        let mut entry_data = [0u8; 24];
        file.read_exact(&mut entry_data)
            .map_err(|_| "Failed to read entry data".to_string())?;
        let offset = u64::from_le_bytes(entry_data[0..8].try_into().unwrap());
        let compressed_size = u64::from_le_bytes(entry_data[8..16].try_into().unwrap());
        let uncompressed_size = u64::from_le_bytes(entry_data[16..24].try_into().unwrap());

        entries.insert(
            path,
            PakEntry {
                offset,
                compressed_size,
                uncompressed_size,
            },
        );
    }

    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|e| format!("Failed to mmap PAK: {e}"))?;

    Ok((PakSource { mmap }, entries))
}

// ── Reader implementation ──

pub struct PakDataReader {
    data: Vec<u8>,
    pos: usize,
}

impl Reader for PakDataReader {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, { STACK_FUTURE_SIZE }> {
        let data = &self.data[self.pos..];
        let len = data.len();
        buf.extend_from_slice(data);
        self.pos = self.data.len();
        StackFuture::from(async move { Ok(len) })
    }

    fn seekable(
        &mut self,
    ) -> Result<&mut dyn bevy::asset::io::SeekableReader, bevy::asset::io::ReaderNotSeekableError>
    {
        Err(bevy::asset::io::ReaderNotSeekableError)
    }
}

impl AsyncRead for PakDataReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let available = self.data.len().saturating_sub(self.pos);
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.data[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Poll::Ready(Ok(to_read))
    }
}

impl Unpin for PakDataReader {}

// ── AssetReader implementation ──

impl AssetReader for PakAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let entry = self
            .get_entry(path)
            .ok_or_else(|| AssetReaderError::NotFound(path.to_path_buf()))?;
        let data = self.decompress(entry)?;
        Ok(PakDataReader { data, pos: 0 })
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        Err::<PakDataReader, _>(AssetReaderError::NotFound(path.to_path_buf()))
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let prefix = path.to_str().unwrap_or("");
        let prefix_slash = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.trim_start_matches('/'))
        };

        let mut dirs = BTreeSet::new();
        for key in self.entries.keys() {
            if key.starts_with(&prefix_slash) {
                let remainder = &key[prefix_slash.len()..];
                if let Some(slash_pos) = remainder.find('/') {
                    dirs.insert(format!("{}{}", prefix_slash, &remainder[..=slash_pos]));
                } else {
                    dirs.insert(key.clone());
                }
            }
        }

        if dirs.is_empty() {
            return Err(AssetReaderError::NotFound(path.to_path_buf()));
        }

        let items: Vec<PathBuf> = dirs.into_iter().map(PathBuf::from).collect();
        let stream: Box<PathStream> = Box::new(futures_lite::stream::iter(items));
        Ok(stream)
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        let prefix = path.to_str().unwrap_or("");
        let prefix_slash = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.trim_start_matches('/'))
        };
        Ok(self.entries.keys().any(|k| k.starts_with(&prefix_slash)))
    }
}

// ── Android PAK extraction from APK assets ──

#[cfg(feature = "android")]
use std::sync::OnceLock;
#[cfg(feature = "android")]
static ANDROID_PAK_DIR: OnceLock<String> = OnceLock::new();

#[cfg(feature = "android")]
pub fn ensure_android_paks() -> Option<String> {
    use std::ffi::CString;
    use std::io::Write;

    let android_app = bevy_android::ANDROID_APP.get()?;
    let internal_path = android_app.internal_data_path()?;
    let pak_dir = internal_path.join("assets_pak");

    // Reuse if already extracted
    if pak_dir.join("data.pak").exists() {
        let s = pak_dir.to_string_lossy().to_string();
        ANDROID_PAK_DIR.set(s.clone()).ok()?;
        return Some(s);
    }

    show_loading_screen();
    std::fs::create_dir_all(&pak_dir).ok()?;

    let asset_manager = android_app.asset_manager();
    let bundle_names = ["data", "bgm", "voice", "se", "image", "video"];

    for name in &bundle_names {
        let filename = format!("assets_pak/{}.pak", name);
        let c_filename = CString::new(filename.as_str()).ok()?;

        let mut asset = match asset_manager.open(&c_filename) {
            Some(a) => a,
            None => {
                warn!("Android asset not found: {}", filename);
                continue;
            }
        };

        let buf = match asset.buffer() {
            Ok(b) => b,
            Err(e) => {
                warn!("Failed to buffer asset {}: {}", filename, e);
                continue;
            }
        };

        let target_path = pak_dir.join(format!("{}.pak", name));
        let mut f = match std::fs::File::create(&target_path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to write {}: {}", target_path.display(), e);
                continue;
            }
        };
        if let Err(e) = f.write_all(buf) {
            warn!("Failed to write {}: {}", target_path.display(), e);
        }
        info!("Extracted PAK: {} ({} bytes)", name, buf.len());
    }

    let s = pak_dir.to_string_lossy().to_string();
    ANDROID_PAK_DIR.set(s.clone()).ok();
    Some(s)
}

// ── Android loading hint ──

#[cfg(feature = "android")]
fn show_loading_screen() {
    let app = match bevy_android::ANDROID_APP.get() {
        Some(a) => a,
        None => return,
    };

    if let Some(nw) = app.native_window() {
        if let Ok(mut buf) = nw.lock(None) {
            if let Some(bytes) = buf.bytes() {
                for byte in bytes.iter_mut() {
                    byte.write(18u8);
                }
            }
        }
    }

    show_toast("解包资源中，请稍候...");
}

#[cfg(feature = "android")]
fn show_toast(message: &str) {
    use jni::objects::{JObject, JValue};
    use jni::signature::RuntimeMethodSignature;
    use jni::strings::JNIString;

    let app = match bevy_android::ANDROID_APP.get() {
        Some(a) => a,
        None => return,
    };

    let jvm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) };
    let _ = jvm.attach_current_thread(|env| -> Result<(), jni::errors::Error> {
        let activity =
            unsafe { JObject::from_raw(env, app.activity_as_ptr() as jni::sys::jobject) };

        let toast_class = env.find_class(JNIString::new("android/widget/Toast"))?;

        let jstr = env.new_string(message)?;

        let sig = RuntimeMethodSignature::from_str(
            "(Landroid/content/Context;Ljava/lang/CharSequence;I)Landroid/widget/Toast;",
        )?;

        let result = env.call_static_method(
            &toast_class,
            JNIString::new("makeText"),
            sig.method_signature(),
            &[JValue::Object(&activity), JValue::Object(&jstr), JValue::Int(1)],
        )?;

        let toast_obj = result.l()?;

        let show_sig = RuntimeMethodSignature::from_str("()V")?;
        env.call_method(toast_obj, JNIString::new("show"), show_sig.method_signature(), &[])?;

        Ok(())
    });
}

// ── ErasedAssetReader factory ──

pub fn create_asset_reader(pak_dir: &str) -> Box<dyn ErasedAssetReader> {
    // Android: check extracted path first
    #[cfg(target_os = "android")]
    if let Some(android_path) = ANDROID_PAK_DIR.get() {
        let android_dir = Path::new(android_path);
        if android_dir.is_dir() {
            let bundle_names = ["data", "bgm", "voice", "se", "image", "video"];
            let pak_paths: Vec<PathBuf> = bundle_names
                .iter()
                .map(|name| android_dir.join(format!("{name}.pak")))
                .filter(|p| p.exists())
                .collect();
            if !pak_paths.is_empty() {
                match PakAssetReader::load_many(&pak_paths) {
                    Ok(reader) => {
                        info!("Using Android PAK asset reader from: {}", android_path);
                        return Box::new(reader);
                    }
                    Err(e) => warn!("Failed to load Android PAKs: {e}"),
                }
            }
        }
    }

    let dir = Path::new(pak_dir);
    if dir.is_dir() {
        let bundle_names = ["data", "bgm", "voice", "se", "image", "video"];
        let pak_paths: Vec<PathBuf> = bundle_names
            .iter()
            .map(|name| dir.join(format!("{name}.pak")))
            .filter(|p| p.exists())
            .collect();

        if !pak_paths.is_empty() {
            match PakAssetReader::load_many(&pak_paths) {
                Ok(reader) => {
                    info!("Using PAK asset reader from: {}", pak_dir);
                    return Box::new(reader);
                }
                Err(e) => {
                    warn!("Failed to load PAK bundles from {pak_dir}: {e}");
                }
            }
        } else {
            warn!("No .pak files found in {pak_dir}, falling back to filesystem");
        }
    }

    // Fallback: single assets.pak
    let single = Path::new("assets.pak");
    if single.exists() {
        match PakAssetReader::load_many(&[single]) {
            Ok(reader) => {
                info!("Using monolithic assets.pak");
                return Box::new(reader);
            }
            Err(e) => {
                warn!("Failed to load assets.pak: {e}");
            }
        }
    }

    info!("Using filesystem asset reader (assets/)");
    Box::new(bevy::asset::io::file::FileAssetReader::new("assets"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_test_pak(path: &Path, items: &[(&str, &[u8])]) {
        let mut data_section = Vec::new();
        let mut index = Vec::new();

        for (name, content) in items {
            let compressed = zstd::encode_all(std::io::Cursor::new(content), 3).unwrap();
            let offset = data_section.len() as u64;
            let compressed_size = compressed.len() as u64;
            data_section.extend_from_slice(&compressed);
            index.push((name, offset, compressed_size, content.len() as u64));
        }

        let mut pak = Vec::new();
        pak.extend_from_slice(&data_section);
        let index_offset = pak.len() as u64;

        for (name, offset, compressed_size, uncompressed_size) in &index {
            let name_bytes = name.as_bytes();
            pak.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            pak.extend_from_slice(name_bytes);
            pak.extend_from_slice(&offset.to_le_bytes());
            pak.extend_from_slice(&compressed_size.to_le_bytes());
            pak.extend_from_slice(&uncompressed_size.to_le_bytes());
        }
        pak.extend_from_slice(PAK_MAGIC);
        pak.extend_from_slice(&PAK_VERSION.to_le_bytes());
        pak.extend_from_slice(&index_offset.to_le_bytes());
        pak.extend_from_slice(&(index.len() as u32).to_le_bytes());

        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(&pak).unwrap();
    }

    #[test]
    fn test_load_many_single() {
        build_test_pak(
            Path::new("/tmp/test_mm.pak"),
            &[("fonts/a.otf", b"hello")],
        );
        let reader = PakAssetReader::load_many(&[Path::new("/tmp/test_mm.pak")]).unwrap();
        assert_eq!(reader.entries.len(), 1);
        let entry = reader.get_entry(Path::new("fonts/a.otf")).unwrap();
        assert_eq!(entry.inner.uncompressed_size, 5);
        let data = reader.decompress(entry).unwrap();
        assert_eq!(&data, b"hello");
        let _ = std::fs::remove_file("/tmp/test_mm.pak");
    }

    #[test]
    fn test_load_many_multiple() {
        build_test_pak(
            Path::new("/tmp/test_a.pak"),
            &[("fonts/a.otf", b"aaa")],
        );
        build_test_pak(
            Path::new("/tmp/test_b.pak"),
            &[("fonts/b.otf", b"bbb")],
        );
        let reader =
            PakAssetReader::load_many(&[Path::new("/tmp/test_a.pak"), Path::new("/tmp/test_b.pak")])
                .unwrap();
        assert_eq!(reader.entries.len(), 2);

        let a = reader.get_entry(Path::new("fonts/a.otf")).unwrap();
        assert_eq!(&reader.decompress(a).unwrap(), b"aaa");

        let b = reader.get_entry(Path::new("fonts/b.otf")).unwrap();
        assert_eq!(&reader.decompress(b).unwrap(), b"bbb");

        let _ = std::fs::remove_file("/tmp/test_a.pak");
        let _ = std::fs::remove_file("/tmp/test_b.pak");
    }

    #[test]
    fn test_load_many_dedup() {
        // Same path in both — first wins
        build_test_pak(
            Path::new("/tmp/test_d1.pak"),
            &[("dup.txt", b"first")],
        );
        build_test_pak(
            Path::new("/tmp/test_d2.pak"),
            &[("dup.txt", b"second")],
        );
        let reader =
            PakAssetReader::load_many(&[Path::new("/tmp/test_d1.pak"), Path::new("/tmp/test_d2.pak")])
                .unwrap();
        assert_eq!(reader.entries.len(), 1);
        let entry = reader.get_entry(Path::new("dup.txt")).unwrap();
        assert_eq!(&reader.decompress(entry).unwrap(), b"first");
        let _ = std::fs::remove_file("/tmp/test_d1.pak");
        let _ = std::fs::remove_file("/tmp/test_d2.pak");
    }

    #[test]
    fn test_read_directory_cross_source() {
        build_test_pak(
            Path::new("/tmp/test_da.pak"),
            &[("a/one.txt", b"1")],
        );
        build_test_pak(
            Path::new("/tmp/test_db.pak"),
            &[("b/two.txt", b"2")],
        );
        let reader = PakAssetReader::load_many(&[
            Path::new("/tmp/test_da.pak"),
            Path::new("/tmp/test_db.pak"),
        ])
        .unwrap();

        let items = futures_lite::future::block_on(async {
            use futures_lite::StreamExt;
            let mut stream = AssetReader::read_directory(&reader, Path::new(""))
                .await
                .unwrap();
            let mut v = Vec::new();
            while let Some(item) = stream.next().await {
                v.push(item);
            }
            v
        });

        assert!(items.contains(&PathBuf::from("a/")));
        assert!(items.contains(&PathBuf::from("b/")));

        let _ = std::fs::remove_file("/tmp/test_da.pak");
        let _ = std::fs::remove_file("/tmp/test_db.pak");
    }
}
