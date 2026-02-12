#!/usr/bin/env bash
set -euo pipefail

REPO="Spiris-Innovation-Tech-Dev/polaris-cli"
SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${SKILL_DIR}/bin"

if [[ -x "${BIN_DIR}/polaris" ]]; then
  echo "polaris binary already installed at ${BIN_DIR}/polaris"
  exit 0
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Darwin) PLATFORM_OS="apple-darwin" ;;
  Linux)  PLATFORM_OS="unknown-linux-musl" ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM_OS="pc-windows-msvc" ;;
  *) echo "Unsupported OS: ${OS}" >&2; exit 1 ;;
esac

case "${ARCH}" in
  x86_64|amd64)  PLATFORM_ARCH="x86_64" ;;
  arm64|aarch64) PLATFORM_ARCH="aarch64" ;;
  *) echo "Unsupported architecture: ${ARCH}" >&2; exit 1 ;;
esac

TARGET="${PLATFORM_ARCH}-${PLATFORM_OS}"

echo "Detecting platform: ${TARGET}"
echo "Fetching latest release from ${REPO}..."

ASSET_URL=$(gh release view --repo "${REPO}" --json assets --jq \
  ".assets[] | select(.name | contains(\"${TARGET}\")) | .url" 2>/dev/null) || true

if [[ -z "${ASSET_URL}" ]]; then
  echo "No release asset found for ${TARGET}" >&2
  echo "Install manually: gh release download --repo ${REPO} --pattern '*${TARGET}*'" >&2
  exit 1
fi

ASSET_NAME=$(gh release view --repo "${REPO}" --json assets --jq \
  ".assets[] | select(.name | contains(\"${TARGET}\")) | .name")

echo "Downloading ${ASSET_NAME}..."

TMPDIR="$(mktemp -d)"
trap 'rm -rf "${TMPDIR}"' EXIT

gh release download --repo "${REPO}" --pattern "*${TARGET}*" --dir "${TMPDIR}"

mkdir -p "${BIN_DIR}"

if [[ "${ASSET_NAME}" == *.tar.gz ]]; then
  tar -xzf "${TMPDIR}/${ASSET_NAME}" -C "${TMPDIR}"
  EXTRACTED_DIR=$(find "${TMPDIR}" -maxdepth 1 -type d -name "polaris-cli-*" | head -1)
  cp "${EXTRACTED_DIR}/bin/polaris" "${BIN_DIR}/polaris"
elif [[ "${ASSET_NAME}" == *.zip ]]; then
  unzip -q "${TMPDIR}/${ASSET_NAME}" -d "${TMPDIR}"
  EXTRACTED_DIR=$(find "${TMPDIR}" -maxdepth 1 -type d -name "polaris-cli-*" | head -1)
  cp "${EXTRACTED_DIR}/bin/polaris.exe" "${BIN_DIR}/polaris.exe"
fi

chmod +x "${BIN_DIR}/polaris" 2>/dev/null || true

echo "Installed polaris to ${BIN_DIR}/polaris"
"${BIN_DIR}/polaris" --version 2>/dev/null || echo "Binary installed successfully"
