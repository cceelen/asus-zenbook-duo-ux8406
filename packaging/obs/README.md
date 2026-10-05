# openSUSE Build Service (OBS)

One OBS package builds all packages for openSUSE, Debian and Ubuntu from a
tagged release. The OBS package holds only `_service`. The service fetches the
other files:

| File                                       | Source                                                    |
| ------------------------------------------ | --------------------------------------------------------- |
| spec, `asus-zenbook-duo-ux8406.dsc`        | the tag                                                   |
| `debian.tar`                               | `packaging/debian/` of the tag                            |
| `asus-zenbook-duo-ux8406-<version>.tar.gz` | the GitHub release, from the URL in `Source0` of the spec |

OBS builds without network access. Thus the archive must be the release archive,
which contains the crates.

## Project

Create the project `home:cceelen:asus-zenbook-duo-ux8406` with this meta
(Project, Advanced, Meta) and the package `asus-zenbook-duo-ux8406` in it:

```xml
<project name="home:cceelen:asus-zenbook-duo-ux8406">
  <title>asus-zenbook-duo-ux8406</title>
  <description>Userland support for the ASUS Zenbook Duo UX8406</description>
  <person userid="cceelen" role="maintainer"/>
  <repository name="openSUSE_Tumbleweed">
    <path project="openSUSE:Tumbleweed" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="16.0">
    <path project="openSUSE:Backports:SLE-16.0" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="16.1">
    <path project="openSUSE:Backports:SLE-16.1" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="Debian_13">
    <path project="Debian:13" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="Debian_Testing">
    <path project="Debian:Testing" repository="standard"/>
    <arch>x86_64</arch>
  </repository>
  <repository name="xUbuntu_26.04">
    <path project="Ubuntu:26.04" repository="universe"/>
    <arch>x86_64</arch>
  </repository>
</project>
```

- Ubuntu 24.04 is not a target: its Rust is 1.75 and 1.85 is necessary.
- Debian 13 and openSUSE have no `udev-hid-bpf`. The keyboard package builds
  there, but you cannot install it.

## Setup

1. Log in to build.opensuse.org with `osc`.
1. Create the project and the package.
1. After the first GitHub release is published, copy `_service` into the package
   and commit it (`osc add _service && osc commit`). The service needs a tag and
   its release archive.
1. Create a workflow token:
   `osc token --create --operation workflow --scm-token GITHUB_TOKEN`. Keep the
   `id` and the token. The GitHub token is fine-grained, for this repository
   only, with Contents: read and Commit statuses: read and write.
1. In the GitHub repository, add a webhook (Settings, Webhooks): payload URL
   `https://build.opensuse.org/trigger/workflow?id=ID`, content type
   `application/json`, secret: the OBS token, events: Pull requests and Pushes.

## Release

A pushed tag starts the OBS workflow immediately. OBS can download the archive
only from a published release, so the first build fails while the release is a
draft. Publish the release, then redeliver the webhook (Settings, Webhooks,
Recent deliveries).

There is no build for pull requests: a pull request has no release archive.

## Users

The "Download package" page of the project gives the commands that add the
repository and its key.
