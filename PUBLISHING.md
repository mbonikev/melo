# Publishing melo

This guide covers shipping `melo` to **yay (AUR)** and **apt** — both free.

## Automated releases (GitHub Actions)

`.github/workflows/release.yml` runs on every `v*` tag push and:

1. Builds native `x86_64` + `aarch64` binaries (ALSA installed on the runners).
2. Packages each into `melo-<ver>-<arch>.tar.gz` and uploads them to the GitHub Release.
3. Computes the source + both binary checksums and **commits the updated
   `packaging/PKGBUILD` and `packaging/melo-bin/PKGBUILD`** (version + sha256s) back to `main`.

4. **Pushes both `melo` and `melo-bin` to the AUR** (the `publish-aur` job) —
   generates `.SRCINFO` and `git push`es to `ssh://aur@aur.archlinux.org`.

So after `git push origin v0.1.0`, the release assets are uploaded, the PKGBUILD
checksums are refreshed, **and both AUR packages are updated automatically** —
`yay -S melo` / `yay -S melo-bin` get the new version with no manual steps.

### One-time setup for the AUR auto-publish

The `publish-aur` job needs an SSH key whose public half is on your AUR account,
stored as a GitHub **repository secret** named `AUR_SSH_PRIVATE_KEY`.

```sh
# 1. Make a dedicated key (no passphrase, so CI can use it non-interactively):
ssh-keygen -t ed25519 -f ~/.ssh/aur_ci -N "" -C "melo-ci"

# 2. Add the PUBLIC key to your AUR account:
cat ~/.ssh/aur_ci.pub        # paste into https://aur.archlinux.org → My Account → SSH key
                             # (you can list multiple keys, one per line)

# 3. Add the PRIVATE key as a GitHub secret:
gh secret set AUR_SSH_PRIVATE_KEY < ~/.ssh/aur_ci   # run in the melo repo
```

That's it — the job auto-creates the AUR packages on first push (the names must
be free) and updates them on every tag after. It only runs on `mbonikev/melo`
(not forks). If the secret is missing the job fails loudly; everything else in
the release still succeeds.

## 0. One-time prep

1. Push this repo to GitHub (e.g. `github.com/mbonikev/melo`).
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

## 1b. AUR binary package (`yay -S melo-bin`, no compiling)

`melo-bin` installs a prebuilt binary in seconds instead of compiling. It
`provides`/`conflicts` `melo`, so users pick one or the other.

```sh
# 1. Build + package the binary tarball (per architecture, on that arch):
./packaging/melo-bin/build-release-tarball.sh        # -> dist/melo-<ver>-<arch>.tar.gz

# 2. Attach it to the GitHub release the PKGBUILD points at:
gh release upload v0.1.0 dist/melo-0.1.0-x86_64.tar.gz

# 3. Publish the recipe to its own AUR repo:
git clone ssh://aur@aur.archlinux.org/melo-bin.git ~/aur-melo-bin
cp packaging/melo-bin/PKGBUILD ~/aur-melo-bin/
cd ~/aur-melo-bin
updpkgsums                                # checksums of the UPLOADED assets
makepkg -si                               # verify it installs
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "Initial import: melo-bin 0.1.0"
git push
```

The PKGBUILD has both `x86_64` and `aarch64` source slots; if you only upload an
x86_64 asset, drop the `aarch64` lines (or upload an aarch64 build too).

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
sudo add-apt-repository ppa:mbonikev/melo
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
`alsa-lib` (Arch) or `libasound2-dev` (Debian/Ubuntu). It also calls
`notify-send` for track-change popups: `libnotify` (Arch) / `libnotify-bin`
(Debian/Ubuntu). Both runtime deps are declared in the PKGBUILD `depends` and
the deb metadata, so users get them automatically.
