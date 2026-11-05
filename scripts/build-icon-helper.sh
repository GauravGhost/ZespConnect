#!/usr/bin/env sh
# Rebuilds src-tauri/resources/icon-helper.jar from android/icon-helper.
# Needs a JDK and the Android SDK (ANDROID_HOME) with a platform and build-tools installed.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SDK=${ANDROID_HOME:-${ANDROID_SDK_ROOT:?set ANDROID_HOME to the Android SDK}}
PLATFORM=$(ls -d "$SDK"/platforms/android-* | sort -V | tail -n 1)
BUILD_TOOLS=$(ls -d "$SDK"/build-tools/* | sort -V | tail -n 1)

D8="$BUILD_TOOLS/d8"
[ -f "$D8" ] || D8="$BUILD_TOOLS/d8.bat"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

javac --release 8 -nowarn -cp "$PLATFORM/android.jar" -d "$WORK/classes" \
    "$ROOT"/android/icon-helper/src/com/zesp/*.java
"$D8" --min-api 21 --lib "$PLATFORM/android.jar" --output "$WORK" "$WORK"/classes/com/zesp/*.class

mkdir -p "$ROOT/src-tauri/resources"
(cd "$WORK" && jar cfM "$ROOT/src-tauri/resources/icon-helper.jar" classes.dex)
echo "Built src-tauri/resources/icon-helper.jar"
