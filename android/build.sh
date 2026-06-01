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
PATH="/tmp/android-toolchain:$NDK_BIN:$PATH"

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

echo "=== Step 3: Copying assets ==="
rm -rf "$ASSETS_DIR"
mkdir -p "$ASSETS_DIR"
cp -r "$PROJECT_DIR/assets/"* "$ASSETS_DIR/"

echo "=== Step 4: Building APK ==="
cd "$SCRIPT_DIR"
gradle assemble"$(echo "$BUILD_TYPE" | sed 's/.*/\u&/')"

echo "=== Done ==="
echo "APK location: $SCRIPT_DIR/app/build/outputs/apk/$BUILD_TYPE/"
