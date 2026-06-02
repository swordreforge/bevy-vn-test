#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TARGET_DIR="$PROJECT_DIR/target"
JNILIBS_DIR="$SCRIPT_DIR/app/src/main/jniLibs"
ASSETS_DIR="$SCRIPT_DIR/app/src/main/assets"
BUILD_TYPE="${1:-release}"

# ── NDK toolchain paths ──
NDK_BIN="/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
NDK_SYSROOT="/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/sysroot"
# ffmpeg-sys-the-third uses target-prefixed tools (aarch64-linux-android21-*) for configure/make.
# Create wrappers for any that are missing in modern NDK.
WRAPPER_DIR="/tmp/android-toolchain"
mkdir -p "$WRAPPER_DIR"
for tool in ar nm strings objdump dlltool; do
    wrapper="$WRAPPER_DIR/aarch64-linux-android21-$tool"
    if [ ! -f "$wrapper" ]; then
        if [ -f "$NDK_BIN/llvm-$tool" ]; then
            ln -sf "$NDK_BIN/llvm-$tool" "$wrapper"
        else
            ln -sf "$NDK_BIN/llvm-ar" "$wrapper"
        fi
    fi
done
# ranlib needs special handling: llvm-ar -s
if [ ! -f "$WRAPPER_DIR/aarch64-linux-android21-ranlib" ]; then
    cat > "$WRAPPER_DIR/aarch64-linux-android21-ranlib" << 'WRAPEOF'
#!/bin/bash
exec llvm-ar -s "$@"
WRAPEOF
    chmod +x "$WRAPPER_DIR/aarch64-linux-android21-ranlib"
fi
PATH="$WRAPPER_DIR:$NDK_BIN:$PATH"

cd "$PROJECT_DIR"

echo "=== Step 0: Generating launcher icons ==="
ICON_SRC="$PROJECT_DIR/9w.png"
ICON_DIR="$SCRIPT_DIR/app/src/main/res"
if [ -f "$ICON_SRC" ]; then
    MDPI="$ICON_DIR/mipmap-mdpi"
    HDPI="$ICON_DIR/mipmap-hdpi"
    XHDPI="$ICON_DIR/mipmap-xhdpi"
    XXHDPI="$ICON_DIR/mipmap-xxhdpi"
    XXXHDPI="$ICON_DIR/mipmap-xxxhdpi"
    mkdir -p "$MDPI" "$HDPI" "$XHDPI" "$XXHDPI" "$XXXHDPI"
    convert "$ICON_SRC" -resize 48x48   "$MDPI/ic_launcher.png"
    convert "$ICON_SRC" -resize 72x72   "$HDPI/ic_launcher.png"
    convert "$ICON_SRC" -resize 96x96   "$XHDPI/ic_launcher.png"
    convert "$ICON_SRC" -resize 144x144 "$XXHDPI/ic_launcher.png"
    convert "$ICON_SRC" -resize 192x192 "$XXXHDPI/ic_launcher.png"
    for dir in "$MDPI" "$HDPI" "$XHDPI" "$XXHDPI" "$XXXHDPI"; do
        cp "$dir/ic_launcher.png" "$dir/ic_launcher_round.png"
    done
    echo "       Icons generated from 9w.png"
else
    echo "       WARNING: 9w.png not found, skipping icon generation"
fi

echo "=== Step 1: Cross-compiling Rust to aarch64-linux-android ==="
CARGO_FLAGS="--target aarch64-linux-android --features android"
if [ "$BUILD_TYPE" = "release" ]; then
    CARGO_FLAGS="$CARGO_FLAGS --release"
fi

env \
    CC_aarch64-linux-android="$NDK_BIN/aarch64-linux-android21-clang" \
    CXX_aarch64-linux-android="$NDK_BIN/aarch64-linux-android21-clang++" \
    AR_aarch64-linux-android="$NDK_BIN/llvm-ar" \
    CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_BIN/aarch64-linux-android21-clang" \
    ANDROID_NDK_HOME="/opt/android-ndk" \
    ANDROID_HOME="/opt/android-sdk" \
    BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android="--sysroot=$NDK_SYSROOT --target=aarch64-linux-android21" \
    cargo build $CARGO_FLAGS

echo "=== Step 2: Copying .so to jniLibs ==="
mkdir -p "$JNILIBS_DIR/arm64-v8a"
cp "$TARGET_DIR/aarch64-linux-android/$BUILD_TYPE/libbevy_vn.so" \
   "$JNILIBS_DIR/arm64-v8a/libbevy_vn.so"

echo "=== Step 2b: Copying libc++_shared.so ==="
CXX_SHARED="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so"
if [ -f "$CXX_SHARED" ]; then
    cp "$CXX_SHARED" "$JNILIBS_DIR/arm64-v8a/libc++_shared.so"
else
    echo "WARNING: libc++_shared.so not found at $CXX_SHARED"
fi

echo "=== Step 3: Stripping .so ==="
"$NDK_BIN/llvm-strip" "$JNILIBS_DIR/arm64-v8a/libbevy_vn.so"

echo "=== Step 4: Packing assets into PAK bundles ==="
rm -rf "$ASSETS_DIR"
mkdir -p "$ASSETS_DIR/assets_pak"

CACHE_DIR="$PROJECT_DIR/zstd_tmp"
if [ -d "$CACHE_DIR" ] && ls "$CACHE_DIR"/*.pak 2>/dev/null | head -1 | grep -q .; then
    echo "       Using cached PAK bundles from zstd_tmp/"
    cp "$CACHE_DIR"/*.pak "$ASSETS_DIR/assets_pak/"
else
    cd "$PROJECT_DIR"
    cargo run --package asset-packer -- \
        --input assets \
        --output "$ASSETS_DIR/assets_pak" \
        --config pack_config.ron \
        --compression-level 3
fi

echo "=== Step 5: Building APK ==="
cd "$SCRIPT_DIR"
gradle assemble"$(echo "$BUILD_TYPE" | sed 's/.*/\u&/')"

echo "=== Done ==="
echo "APK location: $SCRIPT_DIR/app/build/outputs/apk/$BUILD_TYPE/"
