# Linux packages for the ASUS Zenbook Duo UX8406

[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/cceelen/asus-zenbook-duo-ux8406/badge)](https://scorecard.dev/viewer/?uri=github.com/cceelen/asus-zenbook-duo-ux8406)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/15286/badge)](https://www.bestpractices.dev/projects/15286)
[![codecov](https://codecov.io/github/cceelen/asus-zenbook-duo-ux8406/graph/badge.svg?token=CSOYWM3L9Q)](https://codecov.io/github/cceelen/asus-zenbook-duo-ux8406)

**Five packages that make the
[ASUS Zenbook Duo UX8406](https://www.asus.com/us/laptops/for-home/zenbook/asus-zenbook-duo-2024-ux8406/)
operate correctly with Linux: no power-off with the lid closed, a lower screen
that follows the keyboard and the brightness, a function row with all its keys,
touch on the two screens, and screen rotation. They install from the package
manager of your distribution. They are tested on a UX8406CA with Fedora 44; the
UX8406MA is not tested.**

I own a UX8406CA. The community solutions that I tried gave mixed results and
needed manual installation. The power-off problem stayed for months. These
packages are fixes or workarounds until the upstream projects (kernel, desktops,
BIOS) supply the fix.

## Where to start

| You want to                                           | Go to                                                  |
| ----------------------------------------------------- | ------------------------------------------------------ |
| Know what is fixed and what is not                    | [What the packages fix](#what-the-packages-fix)        |
| Know if your distribution has the packages            | [Compatibility](#compatibility)                        |
| Install the packages                                  | [Installation](#installation)                          |
| Turn the screens of a laptop that is not a UX8406     | [Rotation extension](gnome-rotation/README.md)         |
| Report a result or a problem on your hardware         | [CONTRIBUTING.md](CONTRIBUTING.md#report-your-results) |
| Build, test or change the code                        | [CONTRIBUTING.md](CONTRIBUTING.md#build-and-test)      |
| Make a release, or check the origin of a release file | [docs/releases.md](docs/releases.md)                   |
| Report a vulnerability                                | [SECURITY.md](SECURITY.md)                             |

## What the packages fix

Install only the packages that you need. The two display keys of Keys need the
Keyboard package.

| Without the package                                                                    | Package                                          | With the package                                                                                                           |
| -------------------------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| With the lid closed and under load, the laptop switches off.                           | [Guard](tcc-guard/README.md)                     | A service lowers the temperature limit of the CPU for the time of the thermal alert.                                       |
| The lower screen stays on below the keyboard and ignores the brightness control.       | [Second screen](second-screen/README.md)         | The lower screen is off while the keyboard is on it, and has the brightness of the upper screen.                           |
| More than half of the special function keys do nothing.                                | [Keyboard](keyboard-bpf/README.md)               | The keys operate on the dock, on the cable and with Bluetooth. The keyboard light and Fn+Esc operate and keep their state. |
| The two display keys do nothing. A touch on the lower screen acts on the upper screen. | [Keys](gnome-keys/README.md), GNOME only         | The keys switch the lower screen and swap the windows. Each touchscreen has its screen.                                    |
| The screens do not turn when the laptop stands on its side (book mode).                | [Rotation](gnome-rotation/README.md), GNOME only | The two screens turn and go side by side. Not specific to this model.                                                      |

The names in the table are those of the columns in
[Compatibility](#compatibility). [Installation](#installation) gives the names
of the packages.

No fix in this project:

- Microphone on the TRRS jack: no input. Possibly a hardware limit. Refer to the
  [SOF ticket](https://github.com/thesofproject/linux/issues/5703).
- Battery charge limit: the limit of 80 % operates. The battery charged above it
  two times, to 87 % on 2026-09-30 and to 100 % on 2026-10-03. The cause is not
  known
  ([issue 32](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues/32)).

Not examined: fan profiles, pens, beam-forming microphones, camera.

## Compatibility

| Symbol | Meaning                                                  |
| ------ | -------------------------------------------------------- |
| ✅     | A published package. It installs in a clean container.   |
| 🛠️     | No published package. The package builds from this tree. |
| ❌     | Not available.                                           |
| 🤖     | Tested on the hardware.                                  |

| Distribution         | Guard | Second screen | Keyboard | Keys  | Rotation |
| -------------------- | ----- | ------------- | -------- | ----- | -------- |
| Fedora 44            | ✅ 🤖 | ✅ 🤖         | ✅ 🤖    | ✅ 🤖 | ✅ 🤖    |
| Fedora 43            | ✅    | ✅            | ✅       | ✅    | ✅       |
| RHEL 10 and rebuilds | ✅ ¹  | ✅ ¹          | ✅ ¹     | ✅ ¹  | ✅ ¹     |
| Arch                 | ✅    | ✅            | ✅       | ✅    | ✅       |
| openSUSE Tumbleweed  | ✅    | ✅            | ✅ ²     | ✅    | ✅       |
| openSUSE Slowroll    | ✅ ³  | ✅ ³          | ✅ ² ³   | ✅ ³  | ✅ ³     |
| openSUSE Leap 16.0   | ✅    | ✅            | ❌ ⁴     | ✅    | ✅       |
| Debian 13            | ✅    | ✅            | ❌ ⁴     | ✅    | ✅       |
| Debian testing       | ✅    | ✅            | ✅       | ✅    | ✅       |
| Debian unstable      | ✅    | ✅            | ✅       | ✅    | ✅       |
| Ubuntu 26.04         | ✅    | ✅            | ✅       | ✅    | ✅       |
| Alpine               | 🛠️ ⁵  | 🛠️ ⁵          | ❌ ⁴     | 🛠️ ⁵  | 🛠️ ⁵     |
| Nix                  | ✅ ⁶  | ✅ ⁶          | ❌ ⁴     | ✅ ⁶  | ✅ ⁶     |

1. Built for RHEL 10 with EPEL 10. Installed in AlmaLinux 10 and Rocky Linux 10
   containers, not on RHEL.
2. The distribution has no [udev-hid-bpf], which the keyboard package needs. The
   personal OBS project `home:xanders` builds it. With that repository added,
   the keyboard package installs and the loader reads the program. This project
   does not maintain that repository.
3. Published, but not installed in a container: there is no container image.
4. The distribution has no [udev-hid-bpf]. For Alpine, refer to
   [issue 43](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues/43).
5. Not published yet. Refer to [Alpine](#alpine).
6. In the flake of this repository, with the binary cache
   `asus-zenbook-duo-ux8406.cachix.org`. The container builds each package of
   the flake. The NixOS modules are not tested.

🤖: UX8406CA, Fedora 44, Linux 7.2, GNOME 50. The UX8406MA is not tested; its
keyboard ids and dock port can be different. A result from your hardware adds a
test mark: refer to [CONTRIBUTING.md](CONTRIBUTING.md#report-your-results).

Requirements:

| Part             | Requirement                                                      |
| ---------------- | ---------------------------------------------------------------- |
| All but Rotation | A Zenbook Duo UX8406. On other machines the programs do nothing. |
| Guard            | systemd or OpenRC                                                |
| Second screen    | udev                                                             |
| Keyboard         | Linux 6.11 or later with HID-BPF and BTF, and [udev-hid-bpf]     |
| Keys             | GNOME Shell 45 or later                                          |
| Rotation         | GNOME Shell 45 or later, iio-sensor-proxy; a laptop of any model |

Ubuntu 24.04 is not supported: its Rust is too old.

## Installation

Remove other Zenbook Duo solutions before you install these packages. Refer to
[Related projects](#related-projects).

The packages are in two repositories:

- [COPR project](https://copr.fedorainfracloud.org/coprs/packit/cceelen-asus-zenbook-duo-ux8406-releases/)
  for Fedora.
- [OBS project](https://build.opensuse.org/project/show/home:cceelen:asus-zenbook-duo-ux8406)
  for openSUSE, Debian, Ubuntu and Arch. Its
  [download directory](https://download.opensuse.org/repositories/home:/cceelen:/asus-zenbook-duo-ux8406/)
  has one directory for each distribution.

The packages:

| Part          | Package                                              |
| ------------- | ---------------------------------------------------- |
| Guard         | `asus-zenbook-duo-ux8406-tcc-guard`                  |
| Second screen | `asus-zenbook-duo-ux8406-second-screen`              |
| Keyboard      | `asus-zenbook-duo-ux8406-keyboard-bpf`               |
| Keys          | `gnome-shell-extension-asus-zenbook-duo-ux8406-keys` |
| Rotation      | `gnome-shell-extension-builtin-screen-rotation`      |

The commands below use this list. It does not have the keyboard package: the
commands add it where the distribution has udev-hid-bpf (refer to
[Compatibility](#compatibility)).

```sh
pkgs="asus-zenbook-duo-ux8406-tcc-guard asus-zenbook-duo-ux8406-second-screen gnome-shell-extension-asus-zenbook-duo-ux8406-keys gnome-shell-extension-builtin-screen-rotation"
```

### Fedora and RHEL 10

Fedora 43, Fedora 44 and Rawhide:

```sh
sudo dnf copr enable packit/cceelen-asus-zenbook-duo-ux8406-releases
sudo dnf install $pkgs asus-zenbook-duo-ux8406-keyboard-bpf
```

RHEL 10 and its rebuilds need EPEL 10, the CRB repository, and the name of the
build target. On AlmaLinux 10 and Rocky Linux 10:

```sh
sudo dnf install epel-release dnf-plugins-core
sudo crb enable
sudo dnf copr enable packit/cceelen-asus-zenbook-duo-ux8406-releases rhel+epel-10-x86_64
sudo dnf install $pkgs asus-zenbook-duo-ux8406-keyboard-bpf
```

On RHEL, enable EPEL and CodeReady Builder as the
[EPEL documentation](https://docs.fedoraproject.org/en-US/epel/getting-started/)
tells you, then use the last two commands.

### openSUSE

| Distribution | `dist`                |
| ------------ | --------------------- |
| Tumbleweed   | `openSUSE_Tumbleweed` |
| Slowroll     | `openSUSE_Slowroll`   |
| Leap 16.0    | `16.0`                |

```sh
dist=openSUSE_Tumbleweed
sudo zypper addrepo --refresh https://download.opensuse.org/repositories/home:/cceelen:/asus-zenbook-duo-ux8406/$dist/home:cceelen:asus-zenbook-duo-ux8406.repo
sudo zypper install $pkgs
```

### Debian and Ubuntu

| Distribution    | `dist`            |
| --------------- | ----------------- |
| Debian 13       | `Debian_13`       |
| Debian testing  | `Debian_Testing`  |
| Debian unstable | `Debian_Unstable` |
| Ubuntu 26.04    | `xUbuntu_26.04`   |

```sh
dist=Debian_13
repo=https://download.opensuse.org/repositories/home:/cceelen:/asus-zenbook-duo-ux8406/$dist
curl -fsSL $repo/Release.key | sudo gpg --dearmor -o /etc/apt/keyrings/asus-zenbook-duo-ux8406.gpg
echo "deb [signed-by=/etc/apt/keyrings/asus-zenbook-duo-ux8406.gpg] $repo/ /" | sudo tee /etc/apt/sources.list.d/asus-zenbook-duo-ux8406.list
sudo apt update
sudo apt install $pkgs
```

### Arch

```sh
repo=https://download.opensuse.org/repositories/home:/cceelen:/asus-zenbook-duo-ux8406/Arch
curl -fsSL $repo/x86_64/home_cceelen_asus-zenbook-duo-ux8406_Arch.key -o obs.key
sudo pacman-key --add obs.key
sudo pacman-key --lsign-key "$(gpg --show-keys --with-colons obs.key | awk -F: '$1 == "fpr" { print $10; exit }')"
printf '[home_cceelen_asus-zenbook-duo-ux8406_Arch]\nServer = %s/$arch\n' "$repo" | sudo tee -a /etc/pacman.conf
sudo pacman -Sy $pkgs asus-zenbook-duo-ux8406-keyboard-bpf
```

### Alpine

The packages are not published yet. They are sent to Alpine's `testing`
repository:
[merge request 109394](https://gitlab.alpinelinux.org/alpine/aports/-/merge_requests/109394).
Until Alpine merges it, build the packages with `abuild` from
`packaging/alpine/`, as `dev/containers/alpine.Dockerfile` does.

After the merge (`testing` is in the edge repositories only):

```sh
echo https://dl-cdn.alpinelinux.org/alpine/edge/testing | sudo tee -a /etc/apk/repositories
sudo apk add $pkgs asus-zenbook-duo-ux8406-tcc-guard-openrc
```

### NixOS

The flake of this repository has the packages and two NixOS modules. Add the
flake as an input of your system flake, then:

```nix
{
  imports = [
    asus-zenbook-duo-ux8406.nixosModules.default
    asus-zenbook-duo-ux8406.nixosModules.tcc-guard
  ];
  services.asus-zenbook-duo-ux8406-tcc-guard.enable = true;
  environment.systemPackages = with asus-zenbook-duo-ux8406.packages.x86_64-linux; [
    gnome-shell-extension-asus-zenbook-duo-ux8406-keys
    gnome-shell-extension-builtin-screen-rotation
  ];
  # The binary cache. Without it, Nix builds the packages.
  nix.settings.substituters = [ "https://asus-zenbook-duo-ux8406.cachix.org" ];
  nix.settings.trusted-public-keys = [ "asus-zenbook-duo-ux8406.cachix.org-1:XtBns1QGgU3DVAFWmbBzGpVX1jnhJbNcmp+U86CfQbQ=" ];
}
```

`nix build` and `nix run` of the flake offer the same cache by themselves
(`nixConfig` of `flake.nix`); a NixOS system does not read that, so set it as
above. The cache has the packages of the `flake.lock` of the release. Do not set
`inputs.nixpkgs.follows` for this input if you want the cache: with another
nixpkgs, the store paths are different and Nix builds the packages.

Each package and its SBOM have an attestation; refer to
[docs/releases.md](docs/releases.md#check-a-released-file).

### After the installation

| Part           | What you do                                                                         |
| -------------- | ----------------------------------------------------------------------------------- |
| Guard          | Nothing. The package enables and starts the service.                                |
| Second screen  | Nothing. It operates immediately.                                                   |
| Keyboard       | Take the keyboard off the dock or put it on: the program loads at a new connection. |
| Keys, Rotation | Log in again, then enable the extensions with the commands below.                   |

GNOME Shell on Wayland finds a new extension only when it starts:

```sh
gnome-extensions enable asus-zenbook-duo-ux8406-keys@cceelen.github.io
gnome-extensions enable builtin-screen-rotation@cceelen.github.io
```

### Login screen

This step is optional and manual. The login screen (GDM) has its own display
settings and does not read those of your session. Without a saved layout it
switches all screens on, the built-in screens also, at each logout. To give it
the layouts of your session, copy the file. Do this again after you change the
arrangement of your monitors.

GDM 50 (Fedora 44):

```sh
dir=/var/lib/gdm/seat0/config
sudo install -m 644 -o "$(sudo stat -c %u $dir)" -g "$(sudo stat -c %g $dir)" ~/.config/monitors.xml $dir/monitors.xml
sudo restorecon $dir/monitors.xml
```

Older GDM versions use `~gdm/.config/monitors.xml`. Not tested.

## Known limits

Keyboard:

- After a start of the laptop, the function row is in F1–F12 mode, and the
  keyboard light is as the keyboard has it. From then on, the two stay as you
  set them.
- The Bluetooth key of the keyboard starts a new Bluetooth connection of the
  keyboard. In the tests, the laptop gets no key event from it.
- The package controls the keyboard light. The desktop cannot control it.

Lower screen:

- Windows from the lower screen stay on the upper screen when the lower screen
  comes back
  ([issue 37](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues/37)).

Each part README gives the limits of its part.

## Upstream fixes

State in October 2026:

- Keyboard:
  [hid-asus patch](https://ratatoskr.run/linux-input/2026/05/9004316/t) for the
  Linux kernel. Applied, then removed. Not in Linux 7.2.
- Lower screen brightness:
  [backlight capability](https://ratatoskr.run/dri-devel/2026/09/17528280/t) for
  the Linux kernel. In review.
- Thermal event: one kernel patch that only records the event. Not merged. ASUS
  support confirmed the protection: with the lid closed and under load, Windows
  hibernates. Linux is not in their validation. Questions about the event and a
  fix in the BIOS or in `asus-wmi` are with ASUS; no answer at this time.

## Related projects

I tried, examined and learned from these projects. Their manual installation and
configuration was the reason that I did not keep them. The comparison is from
their README files in October 2026.

| Project                                                                                                 | Model    | Method                                   |
| ------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------- |
| [alesya-h/zenbook-duo-2024-ux8406ma-linux](https://github.com/alesya-h/zenbook-duo-2024-ux8406ma-linux) | UX8406MA | Bash and Python, session programs, GNOME |
| [Fmstrat/zenbook-duo-linux](https://github.com/Fmstrat/zenbook-duo-linux)                               | UX8406CA | Bash, systemd                            |
| [zakstam/zenbook-duo-linux](https://github.com/zakstam/zenbook-duo-linux)                               | UX8406MA | Rust daemons; GNOME, KDE, Niri           |
| [JowiAoun/linux-on-zenbook-duo](https://github.com/JowiAoun/linux-on-zenbook-duo)                       | UX8406MA | Bash and Python, user services, GNOME    |
| [carlosh7/asus_UX8406MA](https://github.com/carlosh7/asus_UX8406MA)                                     | UX8406MA | Shell, services, GNOME                   |

Some of them have functions that this project does not have: battery charge
limit, suspend.

## Contact and contributions

Use the
[GitHub issues](https://github.com/cceelen/asus-zenbook-duo-ux8406/issues) for
problems, questions and results on your hardware. A report from a distribution
or a model that is not tested is useful, also when all functions operate.
[CONTRIBUTING.md](CONTRIBUTING.md) tells you what to send and how to build and
test.

## AI policy

I am a software engineer, but Rust and BPF are not my native languages. This
project was developed with Claude Code.

## Disclaimer

This is a community project. It is not affiliated with ASUSTeK Computer Inc.
ASUS and Zenbook are trademarks of ASUSTeK Computer Inc.

The guard changes the temperature at which the CPU throttles, and the helper
switches a screen off. You use the packages at your own risk. There is no
warranty; refer to the licence.

## Licence

MIT, with one exception: the directory `keyboard-bpf/` and its package are
GPL-2.0-only, because they contain headers from the Linux kernel. `LICENSE` is
the MIT text. `LICENSES/` has both texts, and `REUSE.toml` gives the licence of
each file.

[udev-hid-bpf]: https://gitlab.freedesktop.org/libevdev/udev-hid-bpf
