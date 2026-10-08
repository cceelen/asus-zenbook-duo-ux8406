# Releases

For the maintainer: how to make a release and how to keep the tree current.

## Who builds what

| Distribution                         | Built by                    | Configuration                          |
| ------------------------------------ | --------------------------- | -------------------------------------- |
| Fedora, RHEL 10 with EPEL 10         | Fedora COPR, through Packit | `.packit.yaml`                         |
| openSUSE, Debian, Ubuntu 26.04, Arch | openSUSE Build Service      | `.obs/workflows.yml`, `packaging/obs/` |
| Nix                                  | `release-nix.yml`; Cachix   | `flake.nix`, `packaging/nix/`          |
| Alpine                               | Alpine, through aports      | `packaging/alpine/`                    |

COPR, OBS, Cachix and Alpine sign their repositories, each with its own key. No
signing key is stored in GitHub: the attestations are keyless (Sigstore, through
the OIDC token of the workflow).

## Provenance and SBOMs

| File                                        | Made by             | Attestations                             |
| ------------------------------------------- | ------------------- | ---------------------------------------- |
| Source archive                              | `release-files.yml` | build provenance                         |
| Provenance bundle, `<archive>.intoto.jsonl` | `release-files.yml` | on the release page; GitHub release      |
| The five Nix packages (store paths)         | `release-nix.yml`   | build provenance; SBOM, one for each     |
| SBOMs, `<package>-<version>.cdx.json`       | `release-nix.yml`   | on the release page; GitHub release      |
| The release and all its files               | GitHub              | release attestation (immutable releases) |
| Alpine packages                             | Alpine's builders   | none from this project; signed by Alpine |

The workflows that make and attest the files are reusable workflows, which gives
SLSA Build L3 for the provenance. Each SBOM is a CycloneDX file that bombon
makes from the Nix package: the run-time closure and, for the Rust programs, the
crates in the binary (cargo-cyclonedx). The SBOMs also go to the dependency
graph of the repository (Insights, Dependency graph), as SPDX.

The Alpine packages are built from the same source archive, so they have the
same crates. Their system libraries are those of Alpine, not those in the SBOMs.

## Make a release

1. On the Actions tab, select the workflow "release" and "Run workflow".

The other steps occur automatically:

- [Release Please](https://github.com/googleapis/release-please) opens the
  release pull request. It has the next version in the package files and the new
  section of `CHANGELOG.md`, made from the commit titles since the last release:
  `fix:` gives a patch release, `feat:` a minor release.
- The pull request merges itself when the checks are green.
- `release.yml` makes the source archive (`meson dist`) and the Nix packages
  with their SBOMs, attests them, attaches the archive and the SBOMs to the
  draft release and publishes the release. The tag is made at that moment. The
  releases are immutable: nothing can be added after the publication.
- OBS and COPR start their builds.
- `release.yml` pushes the Nix packages to Cachix, sends the SBOMs to the
  dependency graph, and opens a merge request to Alpine's aports. Each of these
  jobs runs only when its service is set up
  ([Cachix and aports](#cachix-and-aports)).

If no `fix:` or `feat:` was merged since the last release, the run fails and
says so. Do not push a tag by hand, and do not change the version in the files
by hand.

## Check a released file

```sh
gh attestation verify asus-zenbook-duo-ux8406-<version>.tar.gz \
    --repo cceelen/asus-zenbook-duo-ux8406 \
    --signer-workflow cceelen/asus-zenbook-duo-ux8406/.github/workflows/release-files.yml
```

Without the attestation API, with the bundle of the provenance from the release
page:

```sh
gh attestation verify asus-zenbook-duo-ux8406-<version>.tar.gz \
    --bundle asus-zenbook-duo-ux8406-<version>.tar.gz.intoto.jsonl \
    --repo cceelen/asus-zenbook-duo-ux8406 \
    --signer-workflow cceelen/asus-zenbook-duo-ux8406/.github/workflows/release-files.yml
```

A Nix package: the subject of its attestations is the NAR of its store path.

```sh
nix store dump-path /nix/store/<hash>-<package>-<version> > package.nar
gh attestation verify package.nar \
    --repo cceelen/asus-zenbook-duo-ux8406 \
    --signer-workflow cceelen/asus-zenbook-duo-ux8406/.github/workflows/release-nix.yml
# The SBOM of the package:
gh attestation verify package.nar \
    --repo cceelen/asus-zenbook-duo-ux8406 \
    --signer-workflow cceelen/asus-zenbook-duo-ux8406/.github/workflows/release-nix.yml \
    --predicate-type https://cyclonedx.org/bom --format json
```

The release and its files: `gh release verify v<version>` and
`gh release verify-asset v<version> <file>`.

The attestations do not cover the packages from COPR, OBS and Alpine.

## Keep the tree current

- Renovate (`renovate.json`) opens pull requests for GitHub Actions, crates, the
  Nix flake and the container base images. It does not change `rust-version`:
  1.85 is the Rust of Debian 13.
- `ci.yml` runs each week on `main` too, for the RustSec advisories and the
  lints of a new stable Rust that appear without a commit. `watch.yml` lists the
  distributions that package `udev-hid-bpf` each week. When a distribution
  starts to package it, the keyboard package becomes installable there: update
  the README.
- For a new distribution release, add it to `.packit.yaml` or to the OBS
  project, and to `dev/containers/`.

## Set up the repository

- Settings, General, Pull Requests: enable "Allow auto-merge".
- A personal access token for the release workflow: fine-grained, for this
  repository only, with "Contents", "Pull requests" and "Issues" set to read and
  write. Store it as the repository secret `RELEASE_TOKEN` (Settings, Secrets
  and variables, Actions). Renew it before it expires.
- A personal access token for the nightly fuzz run: fine-grained, for this
  repository only, with "Repository security advisories" set to read and write.
  Store it as the repository secret `FUZZ_ADVISORY_TOKEN`. Without it, the run
  fails and does not keep the input: the log and the artifacts of a public
  repository are public.
- Rulesets (Settings, Rules, Rulesets): one for `main` and one for the tags
  `v*`. For `main`: no deletion, no force push, linear history, pull requests,
  and the checks of `ci.yml` as required status checks, with "Require branches
  to be up to date before merging". `ci.yml` does not run on a push to `main`:
  the pull request has checked the same tree. Required checks: `rust`,
  `rust-oldest`, `audit`, `pre-commit`, `workflows`, `meson`,
  `dependency-review` and `release-files / archive`. For the tags: no deletion,
  no update.
- Settings, Code security: enable "Private vulnerability reporting".
- Install the Renovate application on the repository.

## Set up the build services

- Packit and COPR: install the Packit GitHub application on the repository. The
  Fedora account `cceelen` must name the GitHub user `cceelen`.
- OBS: refer to `packaging/obs/README.md`.

`doctor.yml` checks each Monday what a release needs: that `RELEASE_TOKEN`,
`FUZZ_ADVISORY_TOKEN` and `APORTS_TOKEN` are valid for three more weeks, that
the OBS project has only its package, that the COPR project and the Cachix cache
exist, and that the aports merge request is open or merged. A red run names the
check that failed.

## Cachix and aports

The jobs `cachix` and `aports` of `release.yml` run only when their variables
are set. The jobs run in the environments of the table. The aports secrets are
repository secrets (Settings, Secrets and variables, Actions).

| Job      | Environment | Secrets                          | Variables                                                     |
| -------- | ----------- | -------------------------------- | ------------------------------------------------------------- |
| `cachix` | `cachix`    | `CACHIX_AUTH_TOKEN`              | `CACHIX_CACHE`                                                |
| `aports` | `aports`    | `APORTS_SSH_KEY`, `APORTS_TOKEN` | `APORTS_FORK`, `APORTS_KNOWN_HOSTS`; `APORTS_DIR` (`testing`) |

The aports merge requests come from a service account, over SSH; its token
(scope `api`) opens the merge request with glab. Each run sets `master` of the
fork to upstream's and replaces the branch `asus-zenbook-duo-ux8406`, so there
is one merge request, with the latest release. The commits keep the maintainer
of the APKBUILD as their author. Make the changes that the aports reviewers ask
for in `packaging/alpine/`, so that the next release has them. To repeat the
merge request of a release, run the workflow "aports" with its tag.
`release.yml` passes the two repository secrets to `aports.yml` by name: a
called workflow gets no other repository secrets. A run started by hand reads
them itself. A secret or variable that is missing stops the job at its first
step, with its name. When Alpine moves the aport to `community/`, set
`APORTS_DIR` to `community`.
