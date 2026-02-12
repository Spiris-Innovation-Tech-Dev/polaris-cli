#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: $0 <version> <target> <output-dir>" >&2
  exit 1
fi

VERSION="$1"
TARGET="$2"
OUTPUT_DIR="$3"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN_DIR="${ROOT_DIR}/target/${TARGET}/release"

BIN_EXT=""
if [[ "${TARGET}" == *"windows"* ]]; then
  BIN_EXT=".exe"
fi

if [[ ! -f "${BIN_DIR}/polaris${BIN_EXT}" ]]; then
  echo "Missing binary: ${BIN_DIR}/polaris${BIN_EXT}" >&2
  exit 1
fi

PKG_NAME="polaris-cli-${VERSION}-${TARGET}"
PKG_ROOT="${ROOT_DIR}/${OUTPUT_DIR}"
PKG_DIR="${PKG_ROOT}/${PKG_NAME}"

rm -rf "${PKG_DIR}"
mkdir -p "${PKG_DIR}/bin" "${PKG_DIR}/scripts"

cp "${BIN_DIR}/polaris${BIN_EXT}" "${PKG_DIR}/bin/polaris${BIN_EXT}"

cp "${ROOT_DIR}/skill/SKILL.md" "${PKG_DIR}/SKILL.md"

cat > "${PKG_DIR}/scripts/polaris" <<EOF
#!/usr/bin/env sh
set -eu
SCRIPT_DIR="\$(CDPATH= cd -- "\$(dirname -- "\$0")" && pwd)"
exec "\${SCRIPT_DIR}/../bin/polaris${BIN_EXT}" "\$@"
EOF
chmod +x "${PKG_DIR}/scripts/polaris"

if [[ "${TARGET}" == *"windows"* ]]; then
  cat > "${PKG_DIR}/scripts/polaris.cmd" <<EOF
@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
if "%HOME%"=="" (
  if not "%USERPROFILE%"=="" (
    set "HOME=%USERPROFILE%"
  ) else (
    set "HOME=%HOMEDRIVE%%HOMEPATH%"
  )
)
"%SCRIPT_DIR%..\\bin\\polaris${BIN_EXT}" %*
EOF
fi

mkdir -p "${PKG_ROOT}"

if [[ "${TARGET}" == *"windows"* ]]; then
  (
    cd "${PKG_ROOT}"
    7z a -tzip "${PKG_NAME}.zip" "${PKG_NAME}" > /dev/null
  )
  echo "${OUTPUT_DIR}/${PKG_NAME}.zip"
else
  (
    cd "${PKG_ROOT}"
    tar -czf "${PKG_NAME}.tar.gz" "${PKG_NAME}"
  )
  echo "${OUTPUT_DIR}/${PKG_NAME}.tar.gz"
fi
