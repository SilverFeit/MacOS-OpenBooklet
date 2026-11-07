#!/bin/zsh
set -euo pipefail

export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"

SCRIPT_DIR="${0:A:h}"
PACKAGE_DIR="${SCRIPT_DIR:h}"
VERSION="$(tr -d '[:space:]' < "${PACKAGE_DIR}/VERSION" 2>/dev/null || echo "1.0.0")"
APP_NAME="OpenBooklet"
STAGE_BUNDLE="/tmp/${APP_NAME}.app"
APP_BUNDLE="${PACKAGE_DIR}/dist/${APP_NAME}.app"

echo "[1/4] Building Release Rust binaries (OpenBookletApp + openbooklet v${VERSION}) via Cargo..."
cargo build --release --manifest-path "${PACKAGE_DIR}/Cargo.toml"

APP_BIN_PATH="${PACKAGE_DIR}/target/release/OpenBookletApp"
CLI_BIN_PATH="${PACKAGE_DIR}/target/release/openbooklet"

echo "[2/4] Generating high-DPI macOS AppIcon.icns & docs/assets/app-icon.png via Rust CoreGraphics..."
ICONSET_DIR="/tmp/OpenBookletIcon.iconset"
ICON_GEN_BIN="/tmp/openbooklet_generate_icns"
rm -rf "${ICONSET_DIR}" "${ICON_GEN_BIN}"
rustc -O "${SCRIPT_DIR}/generate_icns.rs" -o "${ICON_GEN_BIN}"
"${ICON_GEN_BIN}" "${ICONSET_DIR}" "${PACKAGE_DIR}/docs/assets"
rm -f "${ICON_GEN_BIN}"

echo "[3/4] Assembling '${APP_NAME}.app' bundle (v${VERSION}) with bundled 'openbooklet' CLI..."
rm -rf "${STAGE_BUNDLE}"
mkdir -p "${STAGE_BUNDLE}/Contents/MacOS"
mkdir -p "${STAGE_BUNDLE}/Contents/Resources"

cp "${APP_BIN_PATH}" "${STAGE_BUNDLE}/Contents/MacOS/OpenBookletApp"
cp "${CLI_BIN_PATH}" "${STAGE_BUNDLE}/Contents/MacOS/openbooklet"
chmod 755 "${STAGE_BUNDLE}/Contents/MacOS/OpenBookletApp"
chmod 755 "${STAGE_BUNDLE}/Contents/MacOS/openbooklet"
iconutil -c icns "${ICONSET_DIR}" -o "${STAGE_BUNDLE}/Contents/Resources/AppIcon.icns"

cat << EOF > "${STAGE_BUNDLE}/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>OpenBooklet</string>
    <key>CFBundleExecutable</key>
    <string>OpenBookletApp</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>com.silverfeit.openbooklet</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>OpenBooklet</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>pdf</string>
            </array>
            <key>CFBundleTypeName</key>
            <string>PDF Document</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Alternate</string>
        </dict>
    </array>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLName</key>
            <string>com.silverfeit.openbooklet</string>
            <key>CFBundleURLSchemes</key>
            <array>
                <string>openbooklet</string>
            </array>
        </dict>
    </array>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Released under the MIT License. Copyright © 2026 SilverFeit.</string>
</dict>
</plist>
EOF

plutil -lint "${STAGE_BUNDLE}/Contents/Info.plist"

# Universal multi-user POSIX permissions (755 for directories & executables, 644 for resources)
chmod -R 755 "${STAGE_BUNDLE}"
find "${STAGE_BUNDLE}" -type f -not -path "*/MacOS/*" -exec chmod 644 {} +
find "${STAGE_BUNDLE}/Contents/MacOS" -type f -exec chmod 755 {} +

echo "[4/4] Stripping resource forks, signing & verifying '${APP_NAME}.app' + 'openbooklet'..."
xattr -cr "${STAGE_BUNDLE}"
codesign --force --sign - "${STAGE_BUNDLE}/Contents/MacOS/openbooklet"
codesign --force --deep --sign - "${STAGE_BUNDLE}"
codesign --verify --deep --strict "${STAGE_BUNDLE}"

mkdir -p "${PACKAGE_DIR}/dist"
touch "${PACKAGE_DIR}/dist/.metadata_never_index"
rm -rf "${APP_BUNDLE}"
ditto --norsrc --noextattr "${STAGE_BUNDLE}" "${APP_BUNDLE}"
cp "${STAGE_BUNDLE}/Contents/MacOS/openbooklet" "${PACKAGE_DIR}/dist/openbooklet"
chmod 755 "${PACKAGE_DIR}/dist/openbooklet"
rm -rf "${STAGE_BUNDLE}" "${ICONSET_DIR}"

echo "Successfully built and verified: ${APP_BUNDLE} (v${VERSION}) + openbooklet"
