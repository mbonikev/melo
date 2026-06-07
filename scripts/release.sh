#!/usr/bin/env bash
# Cut a release in one command: bump the version, refresh the lockfile, commit,
# and push to main. CI then tags it, builds, and syncs both AUR packages.
#
#   ./scripts/release.sh 0.1.1
set -euo pipefail

v="${1:?usage: scripts/release.sh X.Y.Z}"
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Bump the package version (the [package] version is the only top-level `version =`).
sed -i -E "s/^version = \".*\"/version = \"$v\"/" Cargo.toml

# Keep Cargo.lock in sync (the --locked/--frozen builds require it).
cargo build --release >/dev/null

git add Cargo.toml Cargo.lock
git commit -m "Release v$v"
git push origin main

echo
echo "Pushed v$v to main. CI will tag it, build binaries, publish the GitHub"
echo "Release, and sync melo + melo-bin on the AUR. Watch: gh run watch"
