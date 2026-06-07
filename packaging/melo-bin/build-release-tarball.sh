#!/usr/bin/env bash
# Build melo in release mode and package the prebuilt tarball that the
# melo-bin PKGBUILD downloads. Run from the repo root or anywhere; it locates
# the repo relative to this script.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
cd "$repo"

ver="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')"
arch="$(uname -m)"
name="melo-$ver-$arch"

echo ">> building melo $ver for $arch"
cargo build --release

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
install -m755 target/release/melo "$stage/melo"
install -m644 LICENSE "$stage/LICENSE"
install -m644 README.md "$stage/README.md"

out="$repo/dist"
mkdir -p "$out"
tar -czf "$out/$name.tar.gz" -C "$stage" melo LICENSE README.md

echo
echo ">> created $out/$name.tar.gz"
echo
echo "Next steps:"
echo "  1. Attach it to the GitHub release:"
echo "       gh release upload v$ver \"$out/$name.tar.gz\""
echo "  2. In your melo-bin AUR checkout, refresh the checksums:"
echo "       updpkgsums && makepkg --printsrcinfo > .SRCINFO"
echo "       git commit -am 'melo-bin $ver' && git push"
