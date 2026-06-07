# Publishing melo

This guide covers shipping `melo` to **yay (AUR)** and **apt** — both free.

## 0. One-time prep

1. Push this repo to GitHub (e.g. `github.com/yourname/melo`).
   Update the `url`/`repository` fields in `Cargo.toml` and `packaging/PKGBUILD`.
2. Tag a release so the source tarball exists:

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

   GitHub auto-creates `…/archive/refs/tags/v0.1.0.tar.gz`, which the PKGBUILD uses.

## 1. AUR (installable with `yay -S melo`)

The AUR is a free git-based registry of build recipes (`PKGBUILD`).

```sh
# Create the AUR package repo (needs an AUR account + SSH key uploaded)
git clone ssh://aur@aur.archlinux.org/melo.git aur-melo
cp packaging/PKGBUILD aur-melo/

cd aur-melo
updpkgsums                 # fills in the real sha256 (from pacman-contrib)
makepkg -si                # test it builds & installs locally
makepkg --printsrcinfo > .SRCINFO   # required by the AUR

git add PKGBUILD .SRCINFO
git commit -m "Initial import: melo 0.1.0"
git push
```

After this, anyone can `yay -S melo`. On each new version: bump `pkgver`, rerun
`updpkgsums`, regenerate `.SRCINFO`, commit, push.

## 2. apt — the realistic free options

There is **no single central "apt" registry** like the AUR. Pick one (or several):

### Option A — `.deb` on GitHub Releases (simplest)

```sh
cargo install cargo-deb        # one-time
cargo deb                      # produces target/debian/melo_0.1.0_amd64.deb
```

Upload that `.deb` to the GitHub release. Users install with:

```sh
sudo apt install ./melo_0.1.0_amd64.deb
```

The `[package.metadata.deb]` block in `Cargo.toml` already configures this.

### Option B — Launchpad PPA (gives true `apt install melo`, Ubuntu only)

Free, but Ubuntu-only and builds from source on Launchpad. Create a Launchpad
account, a PPA, then `dput` a source package. Users then:

```sh
sudo add-apt-repository ppa:yourname/melo
sudo apt update && sudo apt install melo
```

### Option C — Self-hosted apt repo (all Debian-based distros)

Host a repo on GitHub Pages with `aptly` or `reprepro`. Users add one line to
`/etc/apt/sources.list.d/` and get `apt install melo` + upgrades. More setup,
maximum reach.

## 3. Bonus reach — every other distro

```sh
cargo install melo        # if you also `cargo publish` to crates.io
```

A Flatpak (Flathub, free) covers any distro with one package, if you want it later.

## Build dependency note

`melo` links ALSA at runtime. Build hosts need the dev package:
`alsa-lib` (Arch) or `libasound2-dev` (Debian/Ubuntu). The `depends`/`$auto`
fields in the PKGBUILD and deb metadata declare the runtime lib for users.
