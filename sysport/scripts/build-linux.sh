#!/bin/bash
set -euo pipefail

APP_NAME="SysPort"
BINARY_NAME="sysport"
DIST_DIR="dist"
PLUGINS_DIR="$DIST_DIR/plugins"
ICON_PNG="../sysport.png"

rm -rf "$DIST_DIR"
mkdir -p "$PLUGINS_DIR"

# Build main binary
cargo build --manifest-path ../sysport/Cargo.toml --release --target x86_64-unknown-linux-gnu
cp ../sysport/target/x86_64-unknown-linux-gnu/release/$BINARY_NAME $DIST_DIR/
echo "[OK] Built $BINARY_NAME for Linux."

# Build plugins as .so
for plugin in ../../plugins/*; do
  if [ -f "$plugin/Cargo.toml" ]; then
    (cd "$plugin" && cargo build --release)
    name=$(basename "$plugin")
    cp "$plugin/target/release/lib${name}.so" $PLUGINS_DIR/
    echo "[OK] Built plugin: lib${name}.so"
  fi
done

# Build .deb package
if command -v cargo-deb &> /dev/null; then
  cargo deb --manifest-path ../sysport/Cargo.toml --target x86_64-unknown-linux-gnu --no-build --output $DIST_DIR/sysport.deb
  echo "[OK] Built .deb package."
else
  echo "[WARN] cargo-deb not installed. Skipping .deb package."
fi

# Build .rpm package
if command -v cargo-generate-rpm &> /dev/null; then
  cargo generate-rpm --target x86_64-unknown-linux-gnu --out $DIST_DIR/
  echo "[OK] Built .rpm package."
else
  echo "[WARN] cargo-generate-rpm not installed. Skipping .rpm package."
fi

# Build AppImage
if command -v appimagetool &> /dev/null; then
  mkdir -p $DIST_DIR/AppDir/usr/bin
  cp $DIST_DIR/$BINARY_NAME $DIST_DIR/AppDir/usr/bin/
  if [ -f $ICON_PNG ]; then
    cp $ICON_PNG $DIST_DIR/AppDir/$ICON_PNG
    ICON_LINE="Icon=$APP_NAME"
  else
    ICON_LINE="Icon=application-default-icon"
  fi
  cat > $DIST_DIR/AppDir/$APP_NAME.desktop <<EOF
[Desktop Entry]
Type=Application
Name=$APP_NAME
Exec=$BINARY_NAME
$ICON_LINE
Categories=Utility;
EOF
  appimagetool $DIST_DIR/AppDir $DIST_DIR/$APP_NAME.AppImage
  echo "[OK] Built AppImage."
else
  echo "[WARN] appimagetool not installed. Skipping AppImage."
fi

echo "[SUCCESS] Linux build complete. Artifacts in $DIST_DIR/" 