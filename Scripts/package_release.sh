#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="${0:A:h}"
PACKAGE_DIR="${SCRIPT_DIR:h}"
DEFAULT_VER="$(tr -d '[:space:]' < "${PACKAGE_DIR}/VERSION" 2>/dev/null || echo "1.0.0")"
VERSION="${1:-${DEFAULT_VER}}"

"${SCRIPT_DIR}/build_app.sh"

APP_NAME="OpenBooklet"
APP_BUNDLE="${PACKAGE_DIR}/dist/${APP_NAME}.app"
RELEASE_DIR="${PACKAGE_DIR}/dist/release"
DMG_STAGE="/tmp/OpenBookletDMGStage"
CASK_FILE="${PACKAGE_DIR}/Casks/macos-openbooklet.rb"

ZIP_NAME="MacOS-OpenBooklet-${VERSION}.app.zip"
DMG_NAME="MacOS-OpenBooklet-${VERSION}.dmg"

rm -rf "${RELEASE_DIR}" "${DMG_STAGE}"
mkdir -p "${RELEASE_DIR}" "${DMG_STAGE}"

echo "[5/7] Creating '${ZIP_NAME}'..."
ditto -c -k --norsrc --noextattr --keepParent "${APP_BUNDLE}" "${RELEASE_DIR}/${ZIP_NAME}"

echo "[6/7] Assembling '${DMG_NAME}' with Applications shortcut & volume icon..."
ditto --norsrc --noextattr "${APP_BUNDLE}" "${DMG_STAGE}/${APP_NAME}.app"
ln -s /Applications "${DMG_STAGE}/Applications"
cp "${APP_BUNDLE}/Contents/Resources/AppIcon.icns" "${DMG_STAGE}/.VolumeIcon.icns"

if diskutil image create --help >/dev/null 2>&1; then
    diskutil image create from \
        --format UDZO \
        --volumeName "MacOS OpenBooklet" \
        "${DMG_STAGE}" \
        "${RELEASE_DIR}/${DMG_NAME}"
else
    hdiutil create \
        -volname "MacOS OpenBooklet" \
        -srcfolder "${DMG_STAGE}" \
        -ov \
        -format UDZO \
        "${RELEASE_DIR}/${DMG_NAME}"
fi

rm -rf "${DMG_STAGE}"

(
    cd "${RELEASE_DIR}"
    shasum -a 256 "${DMG_NAME}" "${ZIP_NAME}" > SHA256SUMS.txt
)

DMG_SHA256="$(shasum -a 256 "${RELEASE_DIR}/${DMG_NAME}" | awk '{print $1}')"

if [[ -f "${CASK_FILE}" ]]; then
    echo "[7/7] Updating Homebrew Cask (${CASK_FILE}) with v${VERSION} & SHA256 ${DMG_SHA256}..."
    sed -i '' -E "s/^  version \".*\"/  version \"${VERSION}\"/" "${CASK_FILE}"
    sed -i '' -E "s/^  sha256 .*/  sha256 \"${DMG_SHA256}\"/" "${CASK_FILE}"
fi

echo "Release artifacts ready in ${RELEASE_DIR}:"
cat "${RELEASE_DIR}/SHA256SUMS.txt"
