#!/usr/bin/env bash
set -euo pipefail

: "${ANDROID_NDK_ROOT:?ANDROID_NDK_ROOT must point to the Android NDK}"

ndk_bin="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin"
mkdir -p android/app/jniLibs/arm64-v8a

"$ndk_bin/clang++" \
  --target=aarch64-linux-android21 \
  -fPIC \
  -shared \
  vendor/SDL/src/hidapi/android/hid.cpp \
  -Ivendor/SDL/include \
  -Ivendor/SDL/src/hidapi \
  -llog \
  -o android/app/jniLibs/arm64-v8a/libhidapi.so
