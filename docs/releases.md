# Releases

For the maintainer: how to make a release and how to keep the tree current.

**Status: no release has been made.** The workflows and the build-service files
in this tree have not run on GitHub, COPR or OBS. The packages build and install
in the containers of `dev/containers/` only.

## Who builds what

| Distribution                   | Built by                                           | Configuration                          |
| ------------------------------ | -------------------------------------------------- | -------------------------------------- |
| Fedora, EPEL 10                | Fedora COPR, through Packit                        | `.packit.yaml`                         |
| openSUSE, Debian, Ubuntu 26.04 | openSUSE Build Service                             | `.obs/workflows.yml`, `packaging/obs/` |
| Arch                           | GitHub release workflow; files on the release page | `.github/workflows/release-files.yml`  |
| Nix                            | the user, from the flake                           | `flake.nix`, `packaging/nix/`          |
| Alpine                         | not distributed; recipe only                       | `packaging/alpine/`                    |

COPR and OBS sign their repositories. The source archive and the Arch packages
on the GitHub release page carry a GitHub build attestation.

## Make a release

1. Set the version in all files: `bump-my-version bump minor` (or `patch`,
   `major`). The files are in `.bumpversion.toml`. Then add the changelog entry
   in the spec and in `packaging/debian/changelog`. A pre-commit hook fails if
   one file has another version.
1. Merge to `main` with a green `ci.yml`.
1. Push the tag `v<version>`. `release.yml` makes the source archive
   (`meson dist`) and the Arch packages, attests them and creates a draft
   release.
1. Publish the draft. OBS and COPR build from the archive of the published
   release.
1. If the OBS build started before the release was published, it fails.
   Redeliver the webhook (GitHub: Settings, Webhooks, Recent deliveries).
1. Check the builds on COPR and OBS.

## Check a released file

```sh
gh attestation verify asus-zenbook-duo-ux8406-<version>.tar.gz \
    --repo cceelen/asus-zenbook-duo-ux8406 \
    --signer-workflow cceelen/asus-zenbook-duo-ux8406/.github/workflows/release-files.yml
```

Use the same command for an Arch package. The attestation does not cover the
packages from COPR and OBS.

## Keep the tree current

- Renovate (`renovate.json`) opens pull requests for GitHub Actions, crates, the
  Nix flake and the container base images. It does not change `rust-version`:
  1.85 is the Rust of Debian 13.
- `watch.yml` runs the RustSec audit each week and lists the distributions that
  package `udev-hid-bpf`. When a distribution starts to package it, the keyboard
  package becomes installable there: update the README.
- For a new distribution release, add it to `.packit.yaml` or to the OBS
  project, and to `dev/containers/`.

## Set up the repository

- Rulesets: import `.github/rulesets/main.json` and
  `.github/rulesets/release-tags.json` (Settings, Rules, Rulesets, Import a
  ruleset). The first makes pull requests and green checks necessary for `main`.
  The second makes release tags permanent.
- Settings, Code security: enable "Private vulnerability reporting".
- Install the Renovate application on the repository.

## Set up the build services

- Packit and COPR: install the Packit GitHub application on the repository. The
  Fedora account `cceelen` must name the GitHub user `cceelen`.
- OBS: refer to `packaging/obs/README.md`.
