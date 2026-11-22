#!/usr/bin/env bash
# Package a staged Linux build (the dist/magnetite-<version>/ folder of
# scripts/build.sh) as an AppImage for this computer's CPU:
#
#   scripts/linux-appimage.sh <stage folder> [--out DIR]
#
#   dist/magnetite-<version>-linux-<arch>.AppImage
#
# One file to download, make executable and start. It holds what the tarball
# holds and no system libraries (docs/legal/license-compatibility.md), so it
# needs the same libraries from the system. It cannot carry a setuid
# chrome-sandbox: Chromium's sandbox needs user namespaces. The app replaces
# the file with the next release's AppImage, named in the signed update
# manifest (crates/app/src/update.rs, `install_appimage`).
#
# An AppImage is AppImage's runtime (a small static program that mounts what
# follows it) with a squashfs image of the folder appended. The runtime is
# downloaded at the release pinned below and checked against its SHA-256.
#
# Needs mksquashfs (squashfs-tools) and curl.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# https://github.com/AppImage/type2-runtime/releases. To update: the new tag
# and the SHA-256 of its runtime-x86_64 and runtime-aarch64, and a look at
# packaging/linux/appimage-runtime.md (what the runtime is made of).
RUNTIME_TAG=20251108
RUNTIME_SHA256_x86_64=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
RUNTIME_SHA256_aarch64=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444

stage="" out="$ROOT/dist"
while (($#)); do
  case $1 in
    --out) out="${2:?--out needs a folder}"; shift ;;
    -h | --help) sed -n '/^set -euo/q; 2,$p' "$0"; exit 0 ;;
    -*) die "unknown option: $1 (see --help)" ;;
    *) stage="$1" ;;
  esac
  shift
done
[[ -x $stage/magnetite ]] || die "usage: scripts/linux-appimage.sh <stage folder from scripts/build.sh> [--out DIR]"
[[ $(host_os) == linux ]] || die "AppImages are built on Linux"
command -v mksquashfs >/dev/null || die "install mksquashfs (squashfs-tools)"
mkdir -p "$out"
out="$(cd "$out" && pwd)"

arch="$(host_arch)"
name="magnetite-$(workspace_version)-linux-$arch"
sha_var="RUNTIME_SHA256_$arch"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

log "AppImage runtime $RUNTIME_TAG"
curl -fsSL --retry 5 -o "$work/runtime" \
  "https://github.com/AppImage/type2-runtime/releases/download/$RUNTIME_TAG/runtime-$arch"
[[ $(sha256_of "$work/runtime") == "${!sha_var}" ]] || die "the AppImage runtime's SHA-256 is not the pinned one"

# The AppDir: the stage as it is (its desktop file and icon are where the
# format wants them), started through AppRun.
appdir="$work/AppDir"
cp -a "$stage" "$appdir"
# The tarball's README is about setting up the folder. chrome-sandbox works
# only setuid root, which a mounted image never is.
rm -f "$appdir/README.md" "$appdir/chrome-sandbox"
cp "$ROOT/packaging/linux/appimage-runtime.md" "$appdir/APPIMAGE_RUNTIME.md"
ln -s magnetite "$appdir/AppRun"
ln -s magnetite.png "$appdir/.DirIcon"

log "AppImage"
mksquashfs "$appdir" "$work/image.squashfs" -root-owned -noappend -comp zstd -quiet -no-progress >&2
cat "$work/runtime" "$work/image.squashfs" >"$out/$name.AppImage"
chmod 755 "$out/$name.AppImage"
echo "$out/$name.AppImage"
