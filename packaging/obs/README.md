# openSUSE Build Service (OBS)

**For the maintainer: how the OBS project is set up. One OBS package builds all
packages for openSUSE, Debian, Ubuntu and Arch from a tagged release, and a new
tag starts the build.** Users find the installation commands in the
[README](../../README.md#installation); the release steps are in
[docs/releases.md](../../docs/releases.md).

The OBS package holds only `_service`. The service fetches the other files:

| File                                            | Source                                                    |
| ----------------------------------------------- | --------------------------------------------------------- |
| spec, `asus-zenbook-duo-ux8406.dsc`, `PKGBUILD` | the tag                                                   |
| `debian.tar`                                    | `packaging/debian/` of the tag                            |
| `asus-zenbook-duo-ux8406-<version>.tar.gz`      | the GitHub release, from the URL in `Source0` of the spec |

OBS builds without network access. Thus the archive must be the release archive,
which contains the crates.

## Project

The project is `home:cceelen:asus-zenbook-duo-ux8406`, a subproject of the home
project, with the package `asus-zenbook-duo-ux8406`. Do not set an SCM sync URL
on the project: OBS then makes one package from each directory of the
repository. The project definition (Repositories, or the page `/meta`):

```xml
<project name="home:cceelen:asus-zenbook-duo-ux8406">
  <title>asus-zenbook-duo-ux8406</title>
  <description>Community support packages for Asus Zenbook Duo UX8406</description>
  <person userid="cceelen" role="maintainer"/>
  <repository name="xUbuntu_26.04">
    <path project="Ubuntu:26.04" repository="universe"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="openSUSE_Tumbleweed">
    <path project="openSUSE:Factory" repository="snapshot"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="openSUSE_Slowroll">
    <path project="openSUSE:Slowroll" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="16.0">
    <path project="openSUSE:Leap:16.0" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="Debian_Unstable">
    <path project="Debian:Next" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="Debian_Testing">
    <path project="Debian:Testing" repository="update"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="Debian_13">
    <path project="Debian:13" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="Arch">
    <path project="Arch:Extra" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
</project>
```

- Ubuntu 24.04 is not a target: its Rust is 1.75 and 1.85 is necessary.
- Where a distribution has no `udev-hid-bpf`, the keyboard package builds, but
  you cannot install it. The
  [compatibility table](../../README.md#compatibility) names these
  distributions.

## Setup

1. Log in to build.opensuse.org with `osc`.
2. Create the project and the package.
3. After the first GitHub release is published, copy `_service` into the package
   and commit it (`osc add _service && osc commit`). The service needs a tag and
   its release archive.
4. Create a workflow token:
   `osc token --create --operation workflow --scm-token GITHUB_TOKEN`. Keep the
   `id` and the token. The GitHub token is fine-grained, for this repository
   only, with Contents: read and Commit statuses: read and write.
5. In the GitHub repository, add a webhook (Settings, Webhooks): payload URL
   `https://build.opensuse.org/trigger/workflow?id=ID`, content type
   `application/json`, secret: the OBS token, events: Pull requests and Pushes.

## Release

A pushed tag starts the OBS workflow (`.obs/workflows.yml`). The release
workflow makes the tag when it publishes the release, so the archive is there
when the build starts. If a build did not start, redeliver the webhook
(Settings, Webhooks, Recent deliveries).

There is no build for pull requests: a pull request has no release archive.
