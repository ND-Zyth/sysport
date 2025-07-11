#!/bin/bash
set -euo pipefail

APP_NAME="SysPort"
BINARY_NAME="sysport"
DIST_DIR="dist"
PLUGINS_DIR="$DIST_DIR/plugins"
ICON_ICNS="sysport/sysport.icns"
APP_BUNDLE="$DIST_DIR/$BINARY_NAME.app"

rm -rf "$DIST_DIR"
mkdir -p "$PLUGINS_DIR"

# Build universal and arch-specific binaries
cargo build --manifest-path ../sysport/Cargo.toml --release --target x86_64-apple-darwin
cargo build --manifest-path ../sysport/Cargo.toml --release --target aarch64-apple-darwin
cp ../sysport/target/x86_64-apple-darwin/release/$BINARY_NAME $DIST_DIR/$BINARY_NAME
cp ../sysport/target/aarch64-apple-darwin/release/$BINARY_NAME $DIST_DIR/${BINARY_NAME}-arm64
echo "[OK] Built $BINARY_NAME for macOS (universal, x86_64, arm64)."

# Build plugins as .dylib
for plugin in ../../plugins/*; do
  if [ -f "$plugin/Cargo.toml" ]; then
    (cd "$plugin" && cargo build --release)
    name=$(basename "$plugin")
    cp "$plugin/target/release/lib${name}.dylib" $PLUGINS_DIR/
    echo "[OK] Built plugin: lib${name}.dylib"
  fi
done

# Create .app bundle
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"
cp $DIST_DIR/$BINARY_NAME "$APP_BUNDLE/Contents/MacOS/$BINARY_NAME"
cp $ICON_ICNS "$APP_BUNDLE/Contents/Resources/sysport.icns"
cat > "$APP_BUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleExecutable</key>
    <string>$BINARY_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.example.sysport</string>
    <key>CFBundleIconFile</key>
    <string>sysport.icns</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
</dict>
</plist>
EOF
echo "[OK] Created .app bundle."

# Create .dmg
if command -v create-dmg &> /dev/null; then
  create-dmg --volname "$APP_NAME" --window-pos 200 120 --window-size 800 400 --icon-size 100 --app-drop-link 600 200 "$DIST_DIR/$APP_NAME.dmg" "$APP_BUNDLE"
  echo "[OK] Built $APP_NAME.dmg."
else
  echo "[WARN] create-dmg not installed. Skipping .dmg."
fi

echo "[SUCCESS] macOS build complete. Artifacts in $DIST_DIR/" 