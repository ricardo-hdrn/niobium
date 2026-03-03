#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FLUTTER="${FLUTTER_BIN:-flutter}"
PLATFORM_PKG="$REPO_ROOT/packages/niobium-linux-x64"
MAIN_PKG="$REPO_ROOT/packages/niobium"
BIN_DIR="$PLATFORM_PKG/bin"

echo "==> Reading version from Cargo.toml"
VERSION=$(grep '^version' "$REPO_ROOT/crates/niobium-mcp/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo "    Version: $VERSION"

echo "==> Building Rust (release)"
cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"

echo "==> Building Flutter (release)"
cd "$REPO_ROOT/app"
$FLUTTER build linux --release

echo "==> Assembling platform package"
rm -rf "$BIN_DIR"
mkdir -p "$BIN_DIR/niobium-app"

# Rust binary
cp "$REPO_ROOT/target/release/niobium" "$BIN_DIR/niobium"
chmod +x "$BIN_DIR/niobium"

# Flutter bundle
cp -r "$REPO_ROOT/app/build/linux/x64/release/bundle/"* "$BIN_DIR/niobium-app/"

# FRB shared library (flutter_rust_bridge needs it in the bundle's lib/)
cp "$REPO_ROOT/target/release/librust_lib_niobium.so" "$BIN_DIR/niobium-app/lib/"

echo "==> Syncing version ($VERSION) into package.json files"
cd "$REPO_ROOT"

# Update both package.json files with the Cargo version
for pkg_json in "$MAIN_PKG/package.json" "$PLATFORM_PKG/package.json"; do
    sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$pkg_json"
done

# Update optionalDependencies version in main package
sed -i "s/\"niobium-mcp-linux-x64\": \"[^\"]*\"/\"niobium-mcp-linux-x64\": \"$VERSION\"/" "$MAIN_PKG/package.json"

echo "==> Done"
echo "    Platform package: $BIN_DIR/"
echo "    Version: $VERSION"
echo ""
echo "To test locally:"
echo "    cd $PLATFORM_PKG && npm link"
echo "    cd $MAIN_PKG && npm link niobium-mcp-linux-x64 && npm link"
echo "    niobium --help"
