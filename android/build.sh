#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TARGET_DIR="$PROJECT_DIR/target"
JNILIBS_DIR="$SCRIPT_DIR/app/src/main/jniLibs"
ASSETS_DIR="$SCRIPT_DIR/app/src/main/assets"
BUILD_TYPE="${1:-release}"

cd "$PROJECT_DIR"

echo "=== Step 1: Cross-compiling Rust to aarch64-linux-android ==="
cargo ndk -t aarch64-linux-android build "--$BUILD_TYPE" --features android

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
