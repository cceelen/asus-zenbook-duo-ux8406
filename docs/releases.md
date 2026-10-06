# Releases

For the maintainer: how to make a release and how to keep the tree current.

**Status: no release has been made.** The workflows and the build-service files
in this tree have not run on GitHub, COPR or OBS. The packages build and install
in the containers of `dev/containers/` only.

## Who builds what

| Distribution                         | Built by                     | Configuration                          |
| ------------------------------------ | ---------------------------- | -------------------------------------- |
| Fedora, EPEL 10                      | Fedora COPR, through Packit  | `.packit.yaml`                         |
| openSUSE, Debian, Ubuntu 26.04, Arch | openSUSE Build Service       | `.obs/workflows.yml`, `packaging/obs/` |
| Nix                                  | the user, from the flake     | `flake.nix`, `packaging/nix/`          |
| Alpine                               | not distributed; recipe only | `packaging/alpine/`                    |

COPR and OBS sign their repositories. The source archive on the GitHub release
page carries a GitHub build attestation.

## Make a release

1. On the Actions tab, select the workflow "release" and "Run workflow".

The other steps occur automatically:

- [Release Please](https://github.com/googleapis/release-please) opens the
  release pull request. It has the next version in the package files and the new
  section of `CHANGELOG.md`, made from the commit titles since the last release:
  `fix:` gives a patch release, `feat:` a minor release.
- The pull request merges itself when the checks are green.
- `release.yml` makes the source archive (`meson dist`), attests it, attaches it
  to the release and publishes the release. The tag is made at that moment.
- OBS and COPR start their builds.

If no `fix:` or `feat:` was merged since the last release, the run fails and
says so. Do not push a tag by hand, and do not change the version in the files
by hand.

## Check a released file

```sh
gh attestation verify asus-zenbook-duo-ux8406-<version>.tar.gz \
    --repo cceelen/asus-zenbook-duo-ux8406 \
    --signer-workflow cceelen/asus-zenbook-duo-ux8406/.github/workflows/release-files.yml
```

The attestation does not cover the packages from COPR and OBS.

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

- Settings, General, Pull Requests: enable "Allow auto-merge".
- A personal access token for the release workflow: fine-grained, for this
  repository only, with "Contents", "Pull requests" and "Issues" set to read and
  write. Store it as the repository secret `RELEASE_TOKEN` (Settings, Secrets
  and variables, Actions). Renew it before it expires.
- Rulesets (Settings, Rules, Rulesets): one for `main` and one for the tags
  `v*`. For `main`: no deletion, no force push, linear history, pull requests,
  and the checks of `ci.yml` as required status checks. For the tags: no
  deletion, no update.
- Settings, Code security: enable "Private vulnerability reporting".
- Install the Renovate application on the repository.

## Set up the build services

- Packit and COPR: install the Packit GitHub application on the repository. The
  Fedora account `cceelen` must name the GitHub user `cceelen`.
- OBS: refer to `packaging/obs/README.md`.
